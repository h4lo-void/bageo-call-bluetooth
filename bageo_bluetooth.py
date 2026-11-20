#!/usr/bin/env python3
# Copyright (C) 2026 h4lo-void (https://github.com/h4lo-void)
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
# SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call-bluetooth>
# See LICENSE and LICENSING.md.
"""bageo-call-bluetooth: a Linux computer with Bluetooth becomes a hands-free kit
(like a car) for a phone, and Bageo answers the phone's calls.

No app and no root on the phone: an iPhone or Android phone pairs with this
computer the way it pairs with a car. BlueZ carries the link; this program is
the hands-free unit itself. It speaks HFP on the RFCOMM channel (caller ID from
+CLIP, ATA to answer, AT+CHUP to hang up or refuse) and moves the call audio
over SCO (CVSD, 8 kHz) to and from Bageo's phone connection, with the messages
Bageo's Android app sends.

  bageo_bluetooth.py run                 the service (systemd starts it)
  bageo_bluetooth.py pair [ADDRESS]      list phones nearby, or pair one (iPhone)
  bageo_bluetooth.py link ADDRESS CODE   use a Bageo on another computer (pairing code)

Settings come from the command line or the environment (the service reads
/etc/default/bageo-bluetooth): BAGEO_URL, BAGEO_TOKEN, SIM_COUNTRY, BT_NAME,
BT_ADAPTER, BT_PAIR_MINUTES. The log keeps events and masked numbers only (no
call audio, no words).
"""
import argparse
import json
import os
import queue
import re
import socket
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

import dbus
import dbus.mainloop.glib
import dbus.service
import numpy as np
import websocket
from gi.repository import GLib

BUS = "org.bluez"
ADAPTER = "org.bluez.Adapter1"
DEVICE = "org.bluez.Device1"
PROPS = "org.freedesktop.DBus.Properties"
HFP_HF = "0000111e-0000-1000-8000-00805f9b34fb"
HFP_AG = "0000111f-0000-1000-8000-00805f9b34fb"
SR = 16000                     # Bageo's phone audio
SCO_SR = 8000                  # CVSD over SCO
SCO_FRAME = SCO_SR * 60 // 1000
HF_BRSF = 4 | 16 | 32          # AT+BRSF: caller ID, remote volume, enhanced call status
HF_SDP = 4 | 16                # SDP SupportedFeatures: caller ID, remote volume
STATE_FILE = "bageo.json"      # in the state folder: the Bageo address and key from "link"

# The SIM's country tells Bageo whether the carrier plays its international-call
# warning (only Taiwan's do) and how to read national numbers. HFP only says which
# network the phone is on (AT+COPS): its MCC, or failing that its name.
MCC = {"466": "tw", "302": "ca", "334": "mx", "440": "jp", "441": "jp", "450": "kr", "460": "cn",
       "454": "hk", "455": "mo", "502": "my", "525": "sg", "520": "th", "452": "vn", "510": "id",
       "515": "ph", "404": "in", "405": "in", "406": "in", "234": "gb", "235": "gb", "262": "de",
       "208": "fr", "222": "it", "214": "es", "268": "pt", "204": "nl", "724": "br", "505": "au",
       "530": "nz", "250": "ru", "286": "tr", "424": "ae", "420": "sa"}
MCC.update({str(m): "us" for m in range(310, 317)})
OPERATOR_NAMES = [("chunghwa", "tw"), ("中華電信", "tw"), ("far eastone", "tw"), ("fareastone", "tw"),
                  ("遠傳", "tw"), ("taiwan mobile", "tw"), ("tw mobile", "tw"), ("台灣大哥大", "tw"),
                  ("台灣之星", "tw"), ("t star", "tw"), ("t-star", "tw"), ("tstar", "tw"), ("taiwan star", "tw"),
                  ("亞太", "tw"), ("aptg", "tw"), ("t-mobile", "us"), ("at&t", "us"), ("verizon", "us")]

_log_lock = threading.Lock()
_debug = False


def log(msg):
    t = time.time()
    with _log_lock:
        print("%s.%03d %s" % (time.strftime("%H:%M:%S", time.localtime(t)), int(t * 1000) % 1000, msg), flush=True)


def mask(number):
    n = (number or "").strip()
    if not n:
        return "(no number)"
    return n if len(n) <= 5 else n[:3] + "*" * (len(n) - 5) + n[-2:]


def mask_line(line):
    """An AT line for the log, with any number in quotes masked."""
    return re.sub(r'"([^"]*)"', lambda m: '"%s"' % mask(m.group(1)) if re.search(r"\d{4}", m.group(1)) else m.group(0),
                  line)


def state_dir():
    return os.environ.get("STATE_DIRECTORY", "").split(":")[0] or "/var/lib/bageo-bluetooth"


def with_token(url, token):
    if not token:
        return url
    return url + ("&" if "?" in url else "?") + "token=" + urllib.parse.quote(token, safe="")


def short_error(e):
    status = getattr(e, "status_code", None)
    if status == 401:
        return "the pairing key is not accepted (make a new pairing code in Bageo and run link again)"
    return "%s%s" % (e.__class__.__name__, " %s" % status if status else "")


# ---------- BlueZ: pairing agent and the hands-free profile ----------

class Agent(dbus.service.Object):
    """Accepts pairing while pairing is open (Just Works), and trusts the phone."""
    I = "org.bluez.Agent1"

    def __init__(self, bus, path, owner):
        super().__init__(bus, path)
        self.owner = owner

    def _ok(self, dev):
        if not self.owner.pair_open:
            log("Pairing refused (pairing is closed): %s" % self.owner.name_of(dev))
            raise dbus.DBusException("org.bluez.Error.Rejected", "pairing closed")
        log("Pairing: %s (accepted)" % self.owner.name_of(dev))
        GLib.timeout_add(500, lambda: (self.owner.trust(dev), False)[1])

    @dbus.service.method(I, in_signature="", out_signature="")
    def Release(self):
        pass

    @dbus.service.method(I, in_signature="o", out_signature="s")
    def RequestPinCode(self, dev):
        self._ok(dev)
        return "0000"

    @dbus.service.method(I, in_signature="o", out_signature="u")
    def RequestPasskey(self, dev):
        self._ok(dev)
        return dbus.UInt32(0)

    @dbus.service.method(I, in_signature="ouq", out_signature="")
    def DisplayPasskey(self, dev, passkey, entered):
        pass

    @dbus.service.method(I, in_signature="os", out_signature="")
    def DisplayPinCode(self, dev, pin):
        pass

    @dbus.service.method(I, in_signature="ou", out_signature="")
    def RequestConfirmation(self, dev, passkey):
        self._ok(dev)

    @dbus.service.method(I, in_signature="o", out_signature="")
    def RequestAuthorization(self, dev):
        self._ok(dev)

    @dbus.service.method(I, in_signature="os", out_signature="")
    def AuthorizeService(self, dev, uuid):
        pass

    @dbus.service.method(I, in_signature="", out_signature="")
    def Cancel(self):
        pass


class Profile(dbus.service.Object):
    I = "org.bluez.Profile1"

    def __init__(self, bus, path, app):
        super().__init__(bus, path)
        self.app = app

    @dbus.service.method(I, in_signature="", out_signature="")
    def Release(self):
        pass

    @dbus.service.method(I, in_signature="oha{sv}", out_signature="")
    def NewConnection(self, dev, fd, props):
        self.app.new_link(str(dev), fd.take())

    @dbus.service.method(I, in_signature="o", out_signature="")
    def RequestDisconnection(self, dev):
        self.app.drop_link(str(dev))

    @dbus.service.method(I, in_signature="", out_signature="")
    def Cancel(self):
        pass


class Bluez:
    """The adapter and the device properties this program needs."""

    def __init__(self, hci):
        self.bus = dbus.SystemBus()
        self.adapter = "/org/bluez/" + hci
        self.aprops = dbus.Interface(self.bus.get_object(BUS, self.adapter), PROPS)
        self.aprops.Set(ADAPTER, "Powered", dbus.Boolean(True))
        self.addr = str(self.aprops.Get(ADAPTER, "Address"))
        self.pair_open = False

    def dev_prop(self, dev, name, default=""):
        try:
            return dbus.Interface(self.bus.get_object(BUS, dev), PROPS).Get(DEVICE, name)
        except dbus.DBusException:
            return default

    def name_of(self, dev):
        return str(self.dev_prop(dev, "Alias", dev.rsplit("/", 1)[-1]))

    def addr_of(self, dev):
        return str(self.dev_prop(dev, "Address", ""))

    def trust(self, dev):
        try:
            dbus.Interface(self.bus.get_object(BUS, dev), PROPS).Set(DEVICE, "Trusted", dbus.Boolean(True))
        except dbus.DBusException as e:
            log("Could not mark the phone as trusted: %s" % e.get_dbus_name())

    def devices(self):
        om = dbus.Interface(self.bus.get_object(BUS, "/"), "org.freedesktop.DBus.ObjectManager")
        return [(p, i[DEVICE]) for p, i in om.GetManagedObjects().items()
                if DEVICE in i and str(p).startswith(self.adapter + "/")]


# ---------- HFP on one RFCOMM link ----------

class Hfp:
    """The hands-free side of HFP: SLC setup, indicators, caller ID, answer, hang up."""

    def __init__(self, app, dev, fd):
        self.app, self.dev = app, dev
        self.sock = socket.socket(fileno=fd)
        self.sock.settimeout(None)
        self.lock = threading.Lock()
        self.results = queue.Queue()
        self.names = []                # indicator names, AT+CIND=? order
        self.val = {}
        self.number = ""
        self.clcc = []                 # [(dir, stat, number)] from the last AT+CLCC
        self.oper = None               # operator from the last AT+COPS?
        self.ag_features = 0
        self.inband = None
        self.alive = True
        threading.Thread(target=self._reader, daemon=True).start()
        threading.Thread(target=self._setup, daemon=True).start()

    def _reader(self):
        buf = b""
        while True:
            try:
                data = self.sock.recv(1024)
            except OSError:
                data = b""
            if not data:
                break
            buf += data
            while True:
                i = buf.find(b"\r")
                if i < 0:
                    break
                line, buf = buf[:i], buf[i + 1:]
                line = line.strip().decode("utf-8", "replace")
                if line:
                    if _debug and line != "OK":
                        log("phone > %s" % mask_line(line))
                    try:
                        self._line(line)
                    except Exception as e:  # noqa: BLE001
                        log("Could not read a line from the phone (%s)" % e.__class__.__name__)
        self.alive = False
        self.results.put("CLOSED")
        self.app.link_closed(self)

    def _line(self, line):
        if line == "OK" or line == "ERROR" or line.startswith("+CME ERROR"):
            self.results.put(line)
        elif line.startswith("+BRSF:"):
            self.ag_features = int(line.split(":", 1)[1].strip() or 0)
        elif line.startswith("+CIND:"):
            body = line.split(":", 1)[1]
            if "(" in body:
                self.names = re.findall(r'\("([^"]+)"', body)
            else:
                for name, v in zip(self.names, body.split(",")):
                    self.val[name] = int(v.strip() or 0)
        elif line.startswith("+CIEV:"):
            i, v = [int(x) for x in line.split(":", 1)[1].split(",")[:2]]
            if 1 <= i <= len(self.names):
                name = self.names[i - 1]
                self.val[name] = v
                self.app.on_indicator(self, name, v)
        elif line == "RING":
            self.app.on_ring(self)
        elif line.startswith("+CLIP:"):
            m = re.search(r'"([^"]*)"', line)
            self.number = m.group(1) if m else ""
            self.app.on_ring(self)
        elif line.startswith("+CLCC:"):
            m = re.search(r'"([^"]*)"', line)
            parts = line.split(":", 1)[1].split(",")
            num = m.group(1) if m else ""
            if len(parts) >= 3:
                self.clcc.append((int(parts[1].strip() or 0), int(parts[2].strip() or 0), num))
            if num and not self.number:
                self.number = num
        elif line.startswith("+BSIR:"):
            self.inband = line.split(":", 1)[1].strip() == "1"
        elif line.startswith("+COPS:"):
            m = re.search(r'"([^"]*)"', line)
            self.oper = m.group(1).strip() if m else None

    def cmd(self, c, timeout=5.0):
        with self.lock:
            while not self.results.empty():
                self.results.get_nowait()
            try:
                if _debug:
                    log("hf > %s" % c)
                self.sock.send((c + "\r").encode())
            except OSError:
                return "CLOSED"
            try:
                return self.results.get(timeout=timeout)
            except queue.Empty:
                return "TIMEOUT"

    def _setup(self):
        steps = [("AT+BRSF=%d" % HF_BRSF, True), ("AT+CIND=?", True), ("AT+CIND?", True),
                 ("AT+CMER=3,0,0,1", True), ("AT+CLIP=1", False), ("AT+CMEE=1", False)]
        for c, must in steps:
            r = self.cmd(c)
            if must and r != "OK":
                log("Hands-free setup failed: %s -> %s" % (c, r))
                self.close()
                return
        self.inband = bool(self.ag_features & 8) if self.inband is None else self.inband
        country = self.sim_country()
        log("Phone connected: %s (number's country: %s, network: %s)" % (
            self.app.bt.name_of(self.dev), country or "unknown", self.oper or "none"))
        self.app.link_ready(self)

    def call(self):
        return self.val.get("call", 0)

    def callsetup(self):
        return self.val.get("callsetup", 0)

    def sim_country(self):
        """Two-letter country of the network the phone is on, or None (AT+COPS)."""
        if self.app.args.sim_country:
            return self.app.args.sim_country.lower()
        for fmt in ("2", "0"):          # numeric (MCC+MNC) first; HFP only requires the name
            self.oper = None
            if self.cmd("AT+COPS=3," + fmt) != "OK" or self.cmd("AT+COPS?") != "OK" or not self.oper:
                continue
            if fmt == "2" and self.oper[:3].isdigit():
                if self.oper[:3] in MCC:
                    return MCC[self.oper[:3]]
                continue
            name = self.oper.lower()
            for key, country in OPERATOR_NAMES:
                if key in name:
                    return country
        return None

    def current_call(self):
        """(number, outgoing) of the active call from AT+CLCC; dir 0 = dialed here, 1 = came in."""
        self.clcc = []
        self.cmd("AT+CLCC")
        incoming = [c for c in self.clcc if c[0] == 1]
        outgoing = [c for c in self.clcc if c[0] == 0]
        number = incoming[0][2] if incoming else ""
        return (number if re.fullmatch(r"\+?[0-9*#]{3,}", number or "") else ""), bool(outgoing and not incoming)

    def close(self):
        try:
            self.sock.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        try:
            self.sock.close()
        except OSError:
            pass


# ---------- call audio over SCO ----------

class Audio:
    """Phone audio in (8 kHz) -> 60 ms 16 kHz frames for Bageo; Bageo's 16 kHz -> 8 kHz out."""

    def __init__(self):
        self.lock = threading.Lock()
        self.play = bytearray()
        self.carry = np.zeros(0, np.int32)
        self.pending = np.zeros(0, np.int16)
        self.q = None
        self.last_rx = 0.0

    def rx(self, data):
        """One SCO packet in; returns the packet to send back (same size)."""
        x = np.frombuffer(data[: len(data) // 2 * 2], "<i2")
        with self.lock:
            self.last_rx = time.time()
            if self.q is not None:
                self.pending = np.concatenate([self.pending, x])
                while len(self.pending) >= SCO_FRAME:
                    f, self.pending = self.pending[:SCO_FRAME], self.pending[SCO_FRAME:]
                    f = f.astype(np.float32)
                    y = np.empty(len(f) * 2, np.float32)
                    y[0::2] = f
                    y[1::2] = (f + np.append(f[1:], f[-1])) / 2
                    self.q.put(np.clip(y, -32768, 32767).astype("<i2").tobytes())
            n = len(data)
            out = bytes(self.play[:n])
            del self.play[:n]
        return out + bytes(n - len(out))

    def feed(self, pcm16k):
        x = np.concatenate([self.carry, np.frombuffer(pcm16k[: len(pcm16k) // 2 * 2], "<i2").astype(np.int32)])
        even = len(x) // 2 * 2
        self.carry = x[even:]
        y = ((x[0:even:2] + x[1:even:2]) // 2).astype("<i2")
        with self.lock:
            self.play.extend(y.tobytes())

    def clear(self):
        with self.lock:
            self.play.clear()

    def pending_s(self):
        with self.lock:
            return len(self.play) / 2.0 / SCO_SR

    def start(self, q):
        with self.lock:
            self.q, self.pending, self.carry = q, np.zeros(0, np.int16), np.zeros(0, np.int32)
            self.play.clear()

    def stop(self):
        with self.lock:
            self.q = None
            self.play.clear()


class Sco:
    """Listens for the phone's audio link; keeps it only for a call Bageo takes."""

    def __init__(self, app, addr):
        self.app, self.addr = app, addr
        self.conn = None
        self.lock = threading.Lock()
        self.srv = socket.socket(socket.AF_BLUETOOTH, socket.SOCK_SEQPACKET, socket.BTPROTO_SCO)
        self.srv.bind(addr.encode())
        self.srv.listen(1)
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self):
        while True:
            try:
                c, peer = self.srv.accept()
            except OSError as e:
                log("Waiting for call audio failed: %s" % e)
                time.sleep(1)
                continue
            if not self.app.taking:
                if self.app.take_audio(c):
                    continue
                log("The phone sent call audio for a call Bageo is not on: sent it back to the phone")
                c.close()
                continue
            self.attach(c, "from the phone")

    def connect(self, peer):
        s = socket.socket(socket.AF_BLUETOOTH, socket.SOCK_SEQPACKET, socket.BTPROTO_SCO)
        s.bind(self.addr.encode())
        s.settimeout(5)
        s.connect(peer.encode())
        s.settimeout(None)
        self.attach(s, "opened here")

    def attach(self, c, who):
        with self.lock:
            old, self.conn = self.conn, c
        if old is not None:
            old.close()
        log("Call audio connected (%s)" % who)
        threading.Thread(target=self._pump, args=(c,), daemon=True).start()

    def _pump(self, c):
        while True:
            try:
                data = c.recv(1024)
            except OSError:
                break
            if not data:
                break
            try:
                c.send(self.app.audio.rx(data))
            except OSError:
                break
        with self.lock:
            if self.conn is c:
                self.conn = None
        log("Call audio closed")

    def up(self):
        return self.conn is not None

    def close(self):
        with self.lock:
            c, self.conn = self.conn, None
        if c is not None:
            try:
                c.close()
            except OSError:
                pass


# ---------- Bageo's always-on phone connection ----------

class Control:
    """Shows the phone in Bageo's console and carries "incoming"."""

    def __init__(self, url, serial, model):
        self.url, self.serial, self.model = url, serial, model
        self.ws = None
        self.plan = None
        self.plan_ev = threading.Event()
        self.send_lock = threading.Lock()
        threading.Thread(target=self._loop, daemon=True).start()

    def _loop(self):
        said = None
        while True:
            try:
                ws = websocket.create_connection(self.url, timeout=10)
                ws.send(json.dumps({"type": "hello", "serial": self.serial, "mode": "CONTROL_IDLE",
                                    "play_tts_locally": False, "model": self.model, "supports_reject": True}))
                ws.settimeout(None)
                self.ws = ws
                log("Connected to Bageo")
                said = None
                while True:
                    raw = ws.recv()
                    if not isinstance(raw, str):
                        continue
                    try:
                        m = json.loads(raw)
                    except ValueError:
                        continue
                    if m.get("type") == "answer_plan":
                        self.plan = m
                        self.plan_ev.set()
            except Exception as e:  # noqa: BLE001
                self.ws = None
                why = short_error(e)
                if why != said:
                    said = why
                    log("Cannot reach Bageo (%s); trying again every 5 seconds" % why)
                time.sleep(5)

    def send(self, obj):
        ws = self.ws
        if ws is None:
            return False
        try:
            with self.send_lock:
                ws.send(json.dumps(obj))
            return True
        except Exception:  # noqa: BLE001
            return False

    def incoming(self, caller, country=None):
        self.plan = None
        self.plan_ev.clear()
        msg = {"type": "incoming", "serial": self.serial, "caller_id": caller, "sample_rate": SR}
        if country:
            msg["sim_country"] = country
        if not self.send(msg):
            return None
        self.plan_ev.wait(1.5)
        return self.plan

    def cancel(self, caller):
        self.send({"type": "incoming_cancel", "serial": self.serial, "caller_id": caller})


# ---------- one call ----------

class Flow:
    """One incoming call from ring to hang-up (taken: already connected, audio already here)."""

    def __init__(self, app, hfp, taken=False, number=""):
        self.app, self.hfp = app, hfp
        self.taken = taken
        self.number = number
        self.country = None
        self.ended = threading.Event()
        threading.Thread(target=self._run, daemon=True).start()

    def _ringing(self):
        return self.hfp.alive and self.hfp.callsetup() == 1 and self.hfp.call() == 0 and not self.ended.is_set()

    def _run(self):
        app, hfp, args = self.app, self.hfp, self.app.args
        try:
            if self.taken:
                self.country = hfp.sim_country()
                session(app, self, self.number)
                return
            t0 = time.time()
            while not hfp.number and time.time() - t0 < 1.5 and self._ringing():
                time.sleep(0.05)
            if not hfp.number and self._ringing():
                hfp.cmd("AT+CLCC")
            number = hfp.number
            self.country = hfp.sim_country()
            log(">>> Incoming call %s (number's country: %s)" % (mask(number), self.country or "unknown"))
            if app.ctl.ws is None:
                log("Bageo is not connected: the call stays on the phone")
                return
            plan = app.ctl.incoming(number, self.country)
            if plan and plan.get("reject"):
                log("Blocked number: refused (%s)" % hfp.cmd("AT+CHUP"))
                return
            delay = args.ring_wait if args.ring_wait is not None else (plan or {}).get("delay_ms", 3000) / 1000.0
            log("Ringing %.1f s before picking up" % max(0.0, delay - (time.time() - t0)))
            while time.time() - t0 < delay:
                if not self._ringing():
                    if hfp.call() == 1:
                        # picked up on the phone (its auto-answer, or you): Bageo takes it
                        # if the phone sends the audio here, else it stays with you
                        log("The phone picked up by itself")
                        t1 = time.time()
                        while not app.sco.up() and time.time() - t1 < 3 and hfp.call() == 1:
                            time.sleep(0.05)
                        if app.sco.up():
                            session(app, self, number)
                        else:
                            app.own_call = True
                            app.ctl.cancel(number)
                            log("The call audio stayed on the phone: the call is yours")
                    else:
                        app.ctl.cancel(number)
                        log("The caller hung up before pick-up")
                    return
                time.sleep(0.05)
            app.taking = True
            r = hfp.cmd("ATA")
            if r != "OK" and hfp.call() != 1:
                log("Could not pick up (%s): the call stays on the phone" % r)
                app.ctl.cancel(number)
                return
            log("Picked up")
            t1 = time.time()
            while not app.sco.up() and time.time() - t1 < 2.0 and hfp.call() == 1:
                time.sleep(0.05)
            if not app.sco.up() and hfp.call() == 1:
                try:
                    app.sco.connect(app.bt.addr_of(hfp.dev))
                except OSError as e:
                    log("The phone sent no call audio and opening it failed (%s)" % e)
            session(app, self, number)
        finally:
            if hfp.alive and hfp.call() == 1:
                app.own_call = True        # still connected after Bageo let go: leave it on the phone
            app.taking = False
            app.sco.close()
            app.audio.stop()
            app.flow = None


def session(app, flow, caller):
    """Bageo talks on the picked-up call until someone hangs up."""
    args, hfp, audio = app.args, flow.hfp, app.audio
    try:
        ws = websocket.create_connection(app.url, timeout=10)
    except Exception as e:  # noqa: BLE001
        log("Could not open the call to Bageo (%s): hanging up" % short_error(e))
        hfp.cmd("AT+CHUP")
        return
    ws.settimeout(None)            # Bageo stays quiet while it listens: no read timeout
    st = {"hangup": None, "closed": False, "first_ai": None, "ai_bytes": 0}
    t0 = time.time()
    q = queue.Queue()
    audio.start(q)

    def reader():
        while True:
            try:
                op, data = ws.recv_data()
            except Exception:  # noqa: BLE001
                st["closed"] = True
                return
            if op == websocket.ABNF.OPCODE_BINARY:
                if len(data) >= 2:
                    if st["first_ai"] is None and np.max(np.abs(np.frombuffer(
                            data[: len(data) // 2 * 2], "<i2").astype(np.int32))) >= 1000:
                        st["first_ai"] = time.time()
                        log("Bageo started talking (%.2f s after pick-up)" % (st["first_ai"] - t0))
                    st["ai_bytes"] += len(data)
                    audio.feed(data)
            elif op == websocket.ABNF.OPCODE_CLOSE:
                st["closed"] = True
                return
            elif op == websocket.ABNF.OPCODE_TEXT:
                try:
                    m = json.loads(data)
                except ValueError:
                    continue
                if m.get("type") == "hangup" and st["hangup"] is None:
                    st["hangup"] = time.time()
                    log("Bageo hangs up")
                elif m.get("type") == "tts_start" and not str(m.get("source", "")).startswith("gemini"):
                    audio.clear()

    def sender():
        while True:
            pcm = q.get()
            if pcm is None:
                return
            try:
                ws.send(pcm, opcode=websocket.ABNF.OPCODE_BINARY)
            except Exception:  # noqa: BLE001
                return

    hello = {"type": "hello", "serial": app.serial, "caller_id": caller, "sample_rate": SR,
             "model": app.bt.name_of(hfp.dev) + " (Bluetooth)", "mode": "SILENT_LOCAL",
             "play_tts_locally": False, "local_menu": False}
    if flow.country:
        hello["sim_country"] = flow.country
    ws.send(json.dumps(hello))
    ws.send(json.dumps({"type": "ringing", "serial": app.serial, "caller_id": caller}))
    threading.Thread(target=reader, daemon=True).start()
    threading.Thread(target=sender, daemon=True).start()
    log("Call handed to Bageo")

    reason, hang_up = "?", False
    while True:
        time.sleep(0.1)
        now = time.time()
        if st["closed"]:
            reason, hang_up = "Bageo closed the call", True
            break
        if st["hangup"] is not None and (audio.pending_s() < 0.05 or now - st["hangup"] > 10):
            reason, hang_up = "Bageo hung up", True
            break
        if not hfp.alive:
            reason = "the phone's hands-free link dropped"
            break
        if hfp.call() == 0 or flow.ended.is_set():
            reason = "the caller hung up"
            break
        if now - t0 > 4 and now - audio.last_rx > 4:
            reason = "the call audio went back to the phone (you took over?)"
            break
        if now - t0 > args.max_min * 60:
            reason, hang_up = "reached %d minutes" % args.max_min, True
            break
    audio.stop()
    q.put(None)
    if hang_up and hfp.alive and hfp.call() == 1:
        log("Hang up: %s" % hfp.cmd("AT+CHUP"))
    try:
        ws.send(json.dumps({"type": "call_end", "serial": app.serial, "caller_id": caller}))
    except Exception:  # noqa: BLE001
        pass
    time.sleep(0.5)
    try:
        ws.close()
    except Exception:  # noqa: BLE001
        pass
    log("Call ended: %s, %.0f s, Bageo talked %.1f s" % (reason, time.time() - t0, st["ai_bytes"] / 2.0 / SR))


# ---------- the service ----------

class App:
    def __init__(self, args, url):
        self.args, self.url = args, url
        self.bt = Bluez(args.adapter)
        self.bt.aprops.Set(ADAPTER, "Alias", dbus.String(args.name))
        self.serial = "bt-" + self.bt.addr.replace(":", "").lower()[-6:]
        self.hfp = None
        self.flow = None
        self.ring_seen = False         # this ringing call already has a Flow
        self.own_call = False          # a call you made or picked up yourself
        self.taking = False
        self.audio = Audio()
        self.ctl = Control(url, self.serial, "Bluetooth hands-free")
        self.agent = Agent(self.bt.bus, "/bageo/agent", self.bt)
        am = dbus.Interface(self.bt.bus.get_object(BUS, "/org/bluez"), "org.bluez.AgentManager1")
        am.RegisterAgent("/bageo/agent", "NoInputNoOutput")
        am.RequestDefaultAgent("/bageo/agent")
        self.profile = Profile(self.bt.bus, "/bageo/hfp", self)
        pm = dbus.Interface(self.bt.bus.get_object(BUS, "/org/bluez"), "org.bluez.ProfileManager1")
        pm.RegisterProfile("/bageo/hfp", HFP_HF, {
            "Name": dbus.String("Hands-Free"), "Version": dbus.UInt16(0x0107),
            "Features": dbus.UInt16(HF_SDP), "RequireAuthentication": dbus.Boolean(True),
            "RequireAuthorization": dbus.Boolean(False), "AutoConnect": dbus.Boolean(True)})
        self.sco = Sco(self, self.bt.addr)
        log("Bluetooth hands-free ready: \"%s\"" % args.name)
        if args.pair_minutes > 0:
            self.open_pairing(int(args.pair_minutes * 60))
        GLib.timeout_add_seconds(2, self.reconnect)

    def open_pairing(self, secs):
        self.bt.pair_open = True
        tries = [0]

        def attempt():
            try:
                self.bt.aprops.Set(ADAPTER, "Pairable", dbus.Boolean(True))
                self.bt.aprops.Set(ADAPTER, "DiscoverableTimeout", dbus.UInt32(secs))
                self.bt.aprops.Set(ADAPTER, "Discoverable", dbus.Boolean(True))
            except dbus.DBusException as e:
                tries[0] += 1
                if tries[0] < 15:
                    return True            # the adapter is busy: try again in a second
                log("Could not open pairing (%s)" % e.get_dbus_name())
                return False
            log("Pairing open for %d minutes: Android phones find \"%s\" under Bluetooth" % (
                secs // 60, self.args.name))
            GLib.timeout_add_seconds(secs, self.close_pairing)
            return False

        if attempt():
            GLib.timeout_add_seconds(1, attempt)

    def close_pairing(self):
        self.bt.pair_open = False
        for k in ("Discoverable", "Pairable"):
            try:
                self.bt.aprops.Set(ADAPTER, k, dbus.Boolean(False))
            except dbus.DBusException:
                pass
        log("Pairing closed (phones already paired still connect)")
        return False

    def reconnect(self):
        """Paired phones with a hands-free gateway: ask them to connect."""
        for path, d in self.bt.devices():
            if not d.get("Paired") or d.get("Blocked") or HFP_AG not in [str(u) for u in d.get("UUIDs", [])]:
                continue
            if self.hfp is not None and self.hfp.alive:
                break
            log("Connecting to the paired phone: %s" % d.get("Alias"))
            dbus.Interface(self.bt.bus.get_object(BUS, path), DEVICE).ConnectProfile(
                HFP_AG, reply_handler=lambda: None,
                error_handler=lambda e: log("Not reachable now (%s); it connects when it is near" % e.get_dbus_name()))
        return False

    def new_link(self, dev, fd):
        if self.hfp is not None and self.hfp.alive:
            log("Another phone is already connected; one phone at a time")
            os.close(fd)
            return
        GLib.timeout_add(100, lambda: (self.bt.trust(dev), False)[1])
        self.hfp = Hfp(self, dev, fd)

    def drop_link(self, dev):
        if self.hfp is not None and self.hfp.dev == dev:
            self.hfp.close()

    def link_ready(self, hfp):
        if hfp.callsetup() == 1 and hfp.call() == 0:
            self.on_ring(hfp)

    def link_closed(self, hfp):
        log("The phone disconnected")
        if self.flow is not None:
            self.flow.ended.set()

    def on_ring(self, hfp):
        if self.flow is None and not self.ring_seen and hfp.call() == 0 and hfp.names:
            self.ring_seen = True
            self.flow = Flow(self, hfp)

    def take_audio(self, c):
        """The phone sends the audio of an active call Bageo did not pick up. Bageo takes
        it unless you dialed it yourself or already moved it back to the phone.
        Returns True when the audio link was taken."""
        hfp = self.hfp
        if hfp is None or not hfp.alive or self.own_call:
            return False
        if hfp.call() != 1 and self.flow is not None:
            # the audio of a call picked up on the phone can arrive a moment before
            # its "call active" indicator: wait briefly before calling it ring audio
            t = time.time()
            while hfp.call() != 1 and time.time() - t < 0.6:
                time.sleep(0.02)
        if hfp.call() != 1:
            return False
        if self.flow is not None:
            # it rang here and the phone picked it up: the Flow carries on with it
            self.taking = True
            self.sco.attach(c, "from the phone")
            return True
        # never rang here (an app call reports only "call active"): who started it?
        number, outgoing = hfp.current_call()
        if outgoing:
            self.own_call = True
            log("A call you made (app call?): it stays on the phone")
            return False
        log(">>> Incoming call that did not ring here (app call or picked up by the phone) %s" % mask(number))
        self.taking = True
        self.sco.attach(c, "from the phone")
        self.flow = Flow(self, hfp, taken=True, number=number)
        return True

    def on_indicator(self, hfp, name, v):
        if name == "callsetup":
            if v == 1:
                self.on_ring(hfp)
            elif v in (2, 3):
                self.own_call = True
                log("A call you made: it stays on the phone")
            elif v == 0 and hfp.call() == 0:
                hfp.number = ""
            if v == 0:
                self.ring_seen = False
        elif name == "call" and v == 0:
            hfp.number = ""
            self.ring_seen = False
            self.own_call = False
            if self.flow is not None:
                self.flow.ended.set()
        if _debug:
            log("state: call=%s callsetup=%s callheld=%s" % (
                hfp.val.get("call"), hfp.val.get("callsetup"), hfp.val.get("callheld")))


def bageo_target(args):
    """Bageo's phone connection with its key: what "link" saved wins, unless a key is given."""
    url, token = args.url, args.token
    if not token:
        try:
            with open(os.path.join(state_dir(), STATE_FILE), encoding="utf-8") as f:
                saved = json.load(f)
            if saved.get("url") and saved.get("token"):
                url, token = saved["url"], saved["token"]
        except (OSError, ValueError):
            pass
    return with_token(url or "ws://127.0.0.1:8787/ws", token)


def cmd_run(args):
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    App(args, bageo_target(args))
    try:
        GLib.MainLoop().run()
    except KeyboardInterrupt:
        log("Stopped")


# ---------- pair: find a phone and pair it from here (iPhone does not list this computer) ----------

def cmd_pair(args):
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bt = Bluez(args.adapter)
    bt.pair_open = True
    Agent(bt.bus, "/bageo/pair_agent", bt)
    am = dbus.Interface(bt.bus.get_object(BUS, "/org/bluez"), "org.bluez.AgentManager1")
    am.RegisterAgent("/bageo/pair_agent", "NoInputNoOutput")
    adapter = dbus.Interface(bt.bus.get_object(BUS, bt.adapter), ADAPTER)
    loop = GLib.MainLoop()
    want = args.address.upper() if args.address else ""

    def find(addr):
        for path, d in bt.devices():
            if str(d.get("Address", "")).upper() == addr:
                return path
        return None

    try:
        adapter.SetDiscoveryFilter({"Transport": dbus.String("bredr")})
    except dbus.DBusException:
        pass
    adapter.StartDiscovery()
    deadline = time.time() + args.seconds

    def tick():
        if want and find(want):
            loop.quit()
            return False
        if time.time() >= deadline:
            loop.quit()
            return False
        return True

    GLib.timeout_add(500, tick)
    print("Looking for phones for up to %d seconds (keep the phone on its Bluetooth screen)..." % args.seconds,
          flush=True)
    loop.run()
    try:
        adapter.StopDiscovery()
    except dbus.DBusException:
        pass

    if not want:
        phones = []
        for path, d in bt.devices():
            cls = int(d.get("Class", 0))
            if ((cls >> 8) & 0x1F) == 2 or str(d.get("Icon", "")).startswith("phone"):
                phones.append((str(d.get("Address")), str(d.get("Alias", "")), bool(d.get("Paired"))))
        if not phones:
            print("No phone found. On the phone, open Settings > Bluetooth and keep that screen open, then try again.")
            return 1
        for addr, name, paired in phones:
            print("  %s  %s%s" % (addr, name, "  (paired)" if paired else ""))
        print("Pair one with: bageo-bluetooth pair <address>   (then tap Pair on the phone)")
        return 0

    path = find(want)
    if not path:
        print("%s was not found. Keep the phone on its Bluetooth screen and try again." % want)
        return 1
    dev = dbus.Interface(bt.bus.get_object(BUS, path), DEVICE)
    result = {}

    def ok():
        result["ok"] = True
        loop.quit()

    def bad(e):
        result["error"] = e.get_dbus_name()
        loop.quit()

    if not bt.dev_prop(path, "Paired", False):
        print("Pairing with %s: tap Pair on the phone..." % bt.name_of(path), flush=True)
        dev.Pair(reply_handler=ok, error_handler=bad, timeout=90)
        loop.run()
        if "error" in result:
            print("Pairing failed (%s). Try again and tap Pair on the phone." % result["error"])
            return 1
    bt.trust(path)
    result.clear()
    dev.ConnectProfile(HFP_AG, reply_handler=ok, error_handler=bad, timeout=30)
    loop.run()
    if "error" in result:
        print("Paired. It connects by itself in a moment (%s)." % result["error"])
    else:
        print("Paired and connected: %s" % bt.name_of(path))
    return 0


# ---------- link: a Bageo on another computer, with a pairing code from its console ----------

def cmd_link(args):
    target = args.address.strip()
    if "://" not in target:
        target = "http://" + target
    u = urllib.parse.urlparse(target)
    host, port = u.hostname, u.port or 8787
    if not host:
        print("Give Bageo's address, for example 192.168.1.20 or 192.168.1.20:8787")
        return 1
    hostpart = "[%s]" % host if ":" in host else host
    body = json.dumps({"code": args.code.strip(), "name": "Bluetooth hands-free (%s)" % socket.gethostname()}).encode()
    req = urllib.request.Request("http://%s:%d/api/phone-pairing/claim" % (hostpart, port), data=body,
                                 headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            answer = json.load(r)
    except urllib.error.HTTPError as e:
        try:
            code = json.load(e).get("code", "")
        except ValueError:
            code = ""
        words = {"phone_wifi_off": "In Bageo's console, turn on \"Phones can also connect over Wi-Fi\" first.",
                 "no_code": "That code is no longer valid. Get a new pairing code in Bageo's console.",
                 "wrong_code": "Wrong pairing code.",
                 "too_many_tries": "Too many wrong codes. Wait a minute and try again."}
        print(words.get(code, "Bageo refused the pairing code (HTTP %d)." % e.code))
        return 1
    except OSError as e:
        print("Cannot reach Bageo at %s:%d (%s)." % (host, port, e))
        return 1
    token = answer.get("token", "")
    if not token:
        print("Bageo gave no pairing key.")
        return 1
    candidates = ["ws://%s:%d%s" % (hostpart, port, answer.get("path", "/ws"))]
    if answer.get("ws_port"):
        candidates.append("ws://%s:%d%s" % (hostpart, int(answer["ws_port"]), answer.get("path", "/ws")))
    for url in candidates:
        try:
            websocket.create_connection(with_token(url, token), timeout=10).close()
        except Exception:  # noqa: BLE001
            continue
        os.makedirs(state_dir(), exist_ok=True)
        path = os.path.join(state_dir(), STATE_FILE)
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump({"url": url, "token": token}, f)
        print("Linked to Bageo at %s. Restart the service: sudo systemctl restart bageo-bluetooth" % url.split("/ws")[0])
        return 0
    print("Bageo accepted the code but its phone connection cannot be reached from here.")
    return 1


def main():
    global _debug
    env = os.environ.get
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--adapter", default=env("BT_ADAPTER", "hci0"), help="Bluetooth adapter (hci0)")
    sub = p.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run", help="the service")
    r.add_argument("--url", default=env("BAGEO_URL", ""), help="Bageo's phone connection (ws://127.0.0.1:8787/ws)")
    r.add_argument("--token", default=env("BAGEO_TOKEN", ""), help="pairing key or DEVICE_TOKEN for a Bageo elsewhere")
    r.add_argument("--name", default=env("BT_NAME", "Bageo HF"), help="Bluetooth name the phone sees")
    r.add_argument("--pair-minutes", type=float, default=float(env("BT_PAIR_MINUTES", "10") or 0),
                   help="minutes pairing stays open after start (0 = closed)")
    r.add_argument("--sim-country", default=env("SIM_COUNTRY", ""),
                   help="two-letter country of the phone's number (default: read from the phone)")
    r.add_argument("--ring-wait", type=float, default=None, help="seconds of ringing before pick-up (default: Bageo's)")
    r.add_argument("--max-min", type=int, default=40, help="longest call in minutes")
    r.add_argument("--debug", action="store_true", default=env("BT_DEBUG", "") == "1",
                   help="log every hands-free line (numbers masked)")
    pa = sub.add_parser("pair", help="list phones nearby, or pair one (needed for iPhone)")
    pa.add_argument("address", nargs="?", default="", help="the phone's Bluetooth address from the list")
    pa.add_argument("--seconds", type=int, default=20)
    li = sub.add_parser("link", help="use a Bageo on another computer, with a pairing code from its console")
    li.add_argument("address", help="Bageo's address, e.g. 192.168.1.20 or 192.168.1.20:8787")
    li.add_argument("code", help="6-digit pairing code")
    args = p.parse_args()
    _debug = bool(getattr(args, "debug", False))
    raise SystemExit({"run": cmd_run, "pair": cmd_pair, "link": cmd_link}[args.command](args) or 0)


if __name__ == "__main__":
    main()
