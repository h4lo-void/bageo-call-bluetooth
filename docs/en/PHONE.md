# Phone setup: answer your phone's calls over Bluetooth

> **In development.** It works and has been tested with real calls, but it is still changing. Don't rely on it alone for calls that matter yet.

Your phone stays an ordinary iPhone or Android phone, with no app and no root. A Linux computer with Bluetooth (a Raspberry Pi, a mini PC, a home server) pairs with it the way a car does. When a call comes in, it rings briefly, picks up, and hands the call to Bageo. Bageo sees the caller's number, talks to the caller, and hangs up when it's done. The AI (Gemini Live) runs through Bageo; the Bluetooth computer does no AI work itself.

The program that does this is in [`bluetooth/`](../../bluetooth/README.md). This page covers:

- what it does and what you need
- installing it and connecting it to Bageo
- pairing an iPhone or an Android phone
- normal phone mode
- phone settings and things to know

## What it does

| Job | How |
|---|---|
| Answers incoming calls | A call rings about 3 seconds (as long as Bageo asks), then Bageo picks up and gets the caller's number. |
| Hangs up and turns numbers away | It hangs up when Bageo is done. Numbers on Bageo's Blacklist are refused. |
| Switches between AI and normal phone | One switch in the console. In normal phone mode, calls stay on the phone. See [Normal phone mode](#normal-phone-mode). |
| Places calls for Bageo | AI call-backs and AI errands go out through the phone. |
| Takes one call at a time | A second call during an AI call is turned away (the caller hears busy), and Bageo notes it. |
| Leaves your own calls alone | Calls you dial yourself, or answer on the phone, stay with you. To take a call over from Bageo, switch the call's sound to the phone. |

## What you need

- **Bageo.** Run it on the Bluetooth computer, on another computer (Windows, macOS, Linux) or in Docker.
- **A Linux computer with Bluetooth near the phone** (about 10 m): Debian, Ubuntu or Raspberry Pi OS. Built-in Bluetooth or a USB Bluetooth adapter both work.
- Windows and macOS can't be the Bluetooth computer: Windows doesn't let programs answer or hang up calls over Bluetooth, and macOS isn't supported yet. Bageo itself still runs on them.

Not possible over Bluetooth: texts, more than one phone at a time, and better than phone-call sound. For the urgent case, see [Whitelist](FEATURES.md#four-contact-lists-and-whitelist-call-transfer).

## Install

**From a download (Linux):** get `bageo-call-bluetooth-<version>-linux-x64.tar.gz` from [Releases](https://github.com/h4lo-void/bageo-call-bluetooth/releases) and unpack it. It holds Bageo and the Bluetooth program:

```sh
./start.sh                    # starts Bageo; the console is at http://localhost:8787/
sudo ./bluetooth/install.sh   # adds Bluetooth answering
```

**From the source:**

```sh
git clone https://github.com/h4lo-void/bageo-call-bluetooth.git
cd bageo-call-bluetooth/bluetooth
sudo ./install.sh
```

This installs BlueZ and a few Python packages, makes Bluetooth show up as a hands-free kit, and starts the `bageo-bluetooth` service.

## Connect it to Bageo

- **Bageo on the same computer:** nothing to do.
- **Bageo on another computer or in Docker:** pair by IP address.
  1. In the console open *System* and, under **Pair a phone by IP address**, click **Get a pairing code**. The code works once, for 10 minutes.
  2. On the Bluetooth computer, run this with Bageo's IP address and the code shown:

```sh
sudo bageo-bluetooth link 192.168.1.20 123456
sudo systemctl restart bageo-bluetooth
```

The Bluetooth computer keeps the address and connects again after a restart. The two computers must be on the same network. **Phones can also connect over Wi-Fi** (*Settings → Advanced*) must be on: if Bageo asks you to turn it on, do that and get a new code. A new code replaces the old one, and five wrong codes void it. On Windows, the first time, the firewall asks whether Bageo may use the network: allow it on **private networks**.

## Pair your phone

- **Android:** within 10 minutes after the service starts, open Settings > Bluetooth > Pair new device, tap "Bageo HF", then tap Pair.
- **iPhone:** iPhone doesn't list computers, so pair from the Bluetooth computer. On the iPhone, open Settings > Bluetooth and keep that screen open. Then run:

```sh
sudo bageo-bluetooth pair                      # lists phones nearby
sudo bageo-bluetooth pair 12:34:56:78:9A:BC    # then tap Pair on the iPhone
```

After pairing, the phone reconnects by itself. It shows up in the console under *System → Phones*; you can forget it there.

## Normal phone mode

The phone has two modes. Switch at any time except during a call:

| Mode | What happens |
|---|---|
| **AI answers my calls** (default) | Calls ring about 3 seconds, then Bageo picks up. |
| **Normal phone** | Bageo leaves the calls alone. They ring on the phone and you answer them yourself. |

Open the console, go to *System → Phones* and use the button on the phone's card. The change takes effect on the next call, and it is kept after a restart.

To stop the AI for every phone at once, switch off *AI answers calls* under *Basics* (calls are then picked up and hung up).

## Phone settings

- **Android:** don't run another app that answers calls on the same phone; it would take them itself.
- **iPhone:** keep Settings > Accessibility > Touch > Call Audio Routing on "Automatic".
- **App calls (LINE, WhatsApp, FaceTime):** phones don't tell Bluetooth devices that these are ringing, so the Bluetooth computer can't pick them up. To have Bageo take them too, turn on "Auto-Answer Calls" and set it to 5 seconds (iPhone: inside Call Audio Routing). Bageo takes the call once the phone sends its sound over. App calls come without the caller's number.

## Good to know

- One phone at a time, within Bluetooth range of the computer.
- With two SIMs the program can't tell which SIM got the call. If the number's country comes out wrong, set `SIM_COUNTRY` (for example `us`) in `/etc/default/bageo-bluetooth`. You can also type the SIM's phone number in the console.
- AI errands and call-backs use this phone only when the number's country is known. If the log says "number's country: unknown", set `SIM_COUNTRY` too.
- Desktop Linux with PipeWire or PulseAudio: they use Bluetooth hands-free too. Use a separate USB Bluetooth adapter, or turn off their hands-free support.
- Settings are in `/etc/default/bageo-bluetooth`; restart the service after changing them.

## Check it's working

```sh
journalctl -u bageo-bluetooth -f
```

"Phone connected" means the phone is linked, and each call shows its steps (numbers are partly hidden). The phone also shows up in the console.

## Uninstall

```sh
sudo ./bluetooth/uninstall.sh
```

## When it does not work

See [Troubleshooting](TROUBLESHOOTING.md) and the [Bluetooth README](../../bluetooth/README.md).

<!-- SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call> -->
