#!/bin/sh
# Copyright (C) 2026 h4lo-void (https://github.com/h4lo-void)
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
# Remove bageo-bluetooth:  sudo ./uninstall.sh   (add --purge to also remove settings and the Bageo key)
set -eu
MAIN=/etc/bluetooth/main.conf
say() { printf '[bageo-bluetooth] %s\n' "$*"; }
[ "$(id -u)" = 0 ] || { say "Run it as root: sudo ./uninstall.sh"; exit 1; }

systemctl disable --now bageo-bluetooth >/dev/null 2>&1 || true
rm -f /etc/systemd/system/bageo-bluetooth.service /usr/local/bin/bageo-bluetooth
rm -rf /opt/bageo-bluetooth
rm -f /etc/systemd/system/bluetooth.service.d/bageo-bluetooth.conf
rmdir /etc/systemd/system/bluetooth.service.d 2>/dev/null || true
if [ -f "$MAIN.bageo-backup" ]; then
  mv -f "$MAIN.bageo-backup" "$MAIN"
fi
if [ "${1:-}" = "--purge" ]; then
  rm -f /etc/default/bageo-bluetooth
  rm -rf /var/lib/bageo-bluetooth
fi
systemctl daemon-reload
systemctl restart bluetooth || true
say "Removed. BlueZ and the Python packages stay installed; paired phones stay paired."
