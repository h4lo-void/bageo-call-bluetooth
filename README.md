# Bageo Bluetooth

Let [Bageo](https://github.com/h4lo-void/bageo-call) answer your phone's calls over Bluetooth. Works with iPhone and Android, with no app and no root on the phone.

A Linux computer with Bluetooth (a Raspberry Pi, a mini PC, a home server) pairs with your phone the way a car does. When a call comes in, it rings briefly, picks up, and hands the call to Bageo. Bageo sees the caller's number, talks to the caller, and hangs up when it's done.

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md)

## What you need

- Bageo running on this computer, another computer, or in Docker. Linking to another computer needs Bageo 1.6.9 or later.
- A Linux computer with Bluetooth near the phone (about 10 m): Debian, Ubuntu, or Raspberry Pi OS. Built-in Bluetooth or a USB adapter both work.
- Windows and macOS can't be the Bluetooth computer: Windows doesn't let programs answer or hang up calls over Bluetooth, and macOS isn't supported yet. Bageo itself still runs on them.

## Install

```sh
git clone https://github.com/h4lo-void/bageo-call-bluetooth.git
cd bageo-call-bluetooth
sudo ./install.sh
```

This installs BlueZ and a few Python packages, makes Bluetooth show up as a hands-free kit, and starts the `bageo-bluetooth` service.

## Connect it to Bageo

- **Bageo on the same computer:** nothing to do.
- **Bageo on another computer or in Docker:** in Bageo's console, open "System" > "Pair a phone by IP address" > "Get a pairing code". Then, on the Bluetooth computer, run (with Bageo's address and the code shown):

```sh
sudo bageo-bluetooth link 192.168.1.20 123456
sudo systemctl restart bageo-bluetooth
```

If Bageo asks you to turn on "Phones can also connect over Wi-Fi", turn it on and get a new code.

## Pair your phone

- **Android:** within 10 minutes after the service starts, open Settings > Bluetooth > Pair new device, tap "Bageo HF", then tap Pair.
- **iPhone:** iPhone doesn't list computers, so pair from the Bluetooth computer. On the iPhone, open Settings > Bluetooth and keep that screen open. Then run:

```sh
sudo bageo-bluetooth pair                      # lists phones nearby
sudo bageo-bluetooth pair 12:34:56:78:9A:BC    # then tap Pair on the iPhone
```

After pairing, the phone reconnects by itself.

## Phone settings

- **Android:** don't run Bageo's Android app on the same phone; it would answer the calls itself.
- **iPhone:** keep Settings > Accessibility > Touch > Call Audio Routing on "Automatic".
- **App calls (LINE, WhatsApp, FaceTime):** phones don't tell Bluetooth devices that these are ringing, so the Bluetooth computer can't pick them up. To have Bageo take them too, turn on "Auto-Answer Calls" and set it to 5 seconds (iPhone: inside Call Audio Routing). Bageo takes the call once the phone sends its sound over.

## How calls go

- A phone call rings about 3 seconds, then Bageo picks up and gets the caller's number.
- Bageo hangs up when the call is done. Numbers on Bageo's Blacklist are refused.
- Calls you make, and calls you answer on the phone yourself, stay on the phone. To take a call over from Bageo, switch the call's sound to the phone.

## Limits

- One phone at a time.
- Sound is phone-call quality.
- App calls come without the caller's number.
- Dual-SIM phones: the program can't tell which SIM got the call. If the number's country comes out wrong, set `SIM_COUNTRY` (for example `us`) in `/etc/default/bageo-bluetooth`.
- Desktop Linux with PipeWire or PulseAudio: they use Bluetooth hands-free too. Use a separate USB Bluetooth adapter, or turn off their hands-free support.

## Check it's working

```sh
journalctl -u bageo-bluetooth -f
```

"Phone connected" means the phone is linked, and each call shows its steps (numbers are partly hidden). The phone also shows up in Bageo's console. Settings are in `/etc/default/bageo-bluetooth`; restart the service after changing them.

## Uninstall

```sh
sudo ./uninstall.sh
```

## License

`AGPL-3.0-only` or a commercial license, the same as bageo-call. See [LICENSING.md](LICENSING.md).
