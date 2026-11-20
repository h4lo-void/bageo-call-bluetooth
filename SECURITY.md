# Security

Bageo Bluetooth lets Bageo answer and place calls from your real phone number. Whoever controls the Bluetooth computer, or Bageo itself, can do the same, so treat both like access to your phone.

## Reporting a vulnerability

Please do not open a public issue for security problems. Use GitHub's **Report a vulnerability** button (Security tab > Advisories) of this repository to report privately. Include what an attacker can do, how to reproduce it, and the version (`bageo-bluetooth --version`).

## Running it safely

- Pairing is open only for the first minutes after the service starts (`BT_PAIR_MINUTES`, 10 by default), and any phone that pairs in that time is accepted. Pair your phone right after installing; set `BT_PAIR_MINUTES=0` once it is paired.
- The key that links this computer to a Bageo elsewhere (`bageo-bluetooth link`) is kept in `/var/lib/bageo-bluetooth/`, readable by root only. Making new pairing codes in Bageo's console does not revoke it; "Pair phones again" in Bageo does.
- The service runs as root (Bluetooth audio and BlueZ profiles need it). Keep the Bluetooth computer up to date and physically in your control.
- Calls go to Bageo unencrypted over your local network when Bageo runs on another computer. Keep both on a network you trust; reach them from outside only through a VPN.
- The log keeps call events and partly hidden numbers, never call audio or words.

For Bageo itself, see bageo-call's [SECURITY.md](https://github.com/h4lo-void/bageo-call/blob/main/SECURITY.md).
