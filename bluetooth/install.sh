#!/bin/sh
# Copyright (C) 2026 h4lo-void (https://github.com/h4lo-void)
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
# Install bageo-bluetooth on Debian, Ubuntu or Raspberry Pi OS:  sudo ./install.sh
set -eu
cd "$(dirname "$0")"
PREFIX=/opt/bageo-bluetooth
MAIN=/etc/bluetooth/main.conf
DROPIN=/etc/systemd/system/bluetooth.service.d/bageo-bluetooth.conf

say() { printf '[bageo-bluetooth] %s\n' "$*"; }

[ "$(id -u)" = 0 ] || { say "Run it as root: sudo ./install.sh"; exit 1; }
command -v apt-get >/dev/null || { say "This installer is for Debian, Ubuntu and Raspberry Pi OS (apt)."; exit 1; }
[ -n "$(ls /sys/class/bluetooth 2>/dev/null)" ] || say "No Bluetooth adapter found yet. Plug one in; the service waits for it."

say "Installing BlueZ and Python packages..."
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  bluez python3 python3-dbus python3-gi python3-numpy python3-websocket >/dev/null

say "Installing the program..."
install -d "$PREFIX"
install -m 755 bageo_bluetooth.py "$PREFIX/bageo_bluetooth.py"
printf '#!/bin/sh\nexec /usr/bin/python3 %s/bageo_bluetooth.py "$@"\n' "$PREFIX" > /usr/local/bin/bageo-bluetooth
chmod 755 /usr/local/bin/bageo-bluetooth
[ -f /etc/default/bageo-bluetooth ] || install -m 644 bageo-bluetooth.default /etc/default/bageo-bluetooth
install -m 644 bageo-bluetooth.service /etc/systemd/system/bageo-bluetooth.service

say "Setting Bluetooth to show up as a hands-free kit..."
if [ -f "$MAIN" ]; then
  [ -f "$MAIN.bageo-backup" ] || cp -a "$MAIN" "$MAIN.bageo-backup"
  grep -q '^Class = 0x000408' "$MAIN" || sed -i '/^\[General\]/a Class = 0x000408' "$MAIN"
  grep -q '^Class = 0x000408' "$MAIN" || printf '\n[General]\nClass = 0x000408\n' >> "$MAIN"
else
  install -d /etc/bluetooth
  printf '[General]\nClass = 0x000408\n' > "$MAIN"
fi
# bluetoothd's hostname plugin would set the class back to "computer".
BTD=$(systemctl cat bluetooth 2>/dev/null | sed -n 's/^ExecStart=\([^ ]*bluetoothd\).*/\1/p' | tail -n 1)
[ -n "$BTD" ] || BTD=/usr/libexec/bluetooth/bluetoothd
install -d "$(dirname "$DROPIN")"
printf '[Service]\nExecStart=\nExecStart=%s -P hostname\n' "$BTD" > "$DROPIN"

for p in pipewire wireplumber pulseaudio ofonod; do
  if pgrep -x "$p" >/dev/null 2>&1; then
    say "Note: $p is running and may also use Bluetooth hands-free. If phones don't connect, use a separate USB Bluetooth adapter or turn off its hands-free support."
    break
  fi
done

systemctl daemon-reload
systemctl restart bluetooth
systemctl enable --now bageo-bluetooth >/dev/null 2>&1
systemctl restart bageo-bluetooth

say "Done. Bluetooth name: Bageo HF (pairing is open for 10 minutes)."
say "Bageo on another computer:  sudo bageo-bluetooth link <address> <pairing code>"
say "iPhone:                     sudo bageo-bluetooth pair"
say "Log:                        journalctl -u bageo-bluetooth -f"
