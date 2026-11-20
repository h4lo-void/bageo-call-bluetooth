# Bageo 蓝牙

让 [Bageo](https://github.com/h4lo-void/bageo-call) 通过蓝牙接你手机的电话。iPhone、Android 都可以，手机不用装 App，也不用 Root。

一台有蓝牙的 Linux 电脑（树莓派、迷你主机、家里的服务器）会像汽车一样和手机配对。有电话打来时，它响几秒就接起来，交给 Bageo。Bageo 能看到来电号码，会和对方说话，说完自己挂断。

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md)

## 需要什么

- 一个正在运行的 Bageo：装在这台电脑、另一台电脑或 Docker 里都可以。连到另一台电脑需要 Bageo 1.6.9 以上。
- 一台有蓝牙、放在手机附近（约 10 米内）的 Linux 电脑：Debian、Ubuntu 或 Raspberry Pi OS。内置蓝牙或 USB 蓝牙都可以。
- Windows 和 Mac 不能当蓝牙电脑：Windows 不让程序通过蓝牙接听、挂断电话，Mac 目前还不支持。Bageo 本身照样可以装在上面。

## 安装

```sh
git clone https://github.com/h4lo-void/bageo-call-bluetooth.git
cd bageo-call-bluetooth
sudo ./install.sh
```

这会安装 BlueZ 和几个 Python 包，把蓝牙设成免提设备，并启动 `bageo-bluetooth` 服务。

## 连到 Bageo

- **Bageo 在同一台电脑：** 不用做什么。
- **Bageo 在另一台电脑或 Docker：** 在 Bageo 控制台打开“系统”→“用 IP 地址配对手机”→“生成配对码”，然后在蓝牙电脑上运行（换成 Bageo 的地址和页面上的配对码）：

```sh
sudo bageo-bluetooth link 192.168.1.20 123456
sudo systemctl restart bageo-bluetooth
```

如果 Bageo 要你先打开“手机也能用 Wi-Fi 连接”，打开后再生成一次配对码。

## 配对手机

- **Android：** 服务启动后 10 分钟内，打开“设置 → 蓝牙 → 配对新设备”，点“Bageo HF”，再点“配对”。
- **iPhone：** iPhone 的列表不会显示电脑，要从蓝牙电脑这边配对。先在 iPhone 打开“设置 → 蓝牙”并停在这一页，然后运行：

```sh
sudo bageo-bluetooth pair                      # 列出附近的手机
sudo bageo-bluetooth pair 12:34:56:78:9A:BC    # 再到 iPhone 上点“配对”
```

配对好之后，手机会自己连回来。

## 手机设置

- **Android：** 同一部手机不要再开 Bageo 的手机 App，否则它会自己去接电话。
- **iPhone：** “设置 → 辅助功能 → 触控 → 通话音频路由”保持“自动”。
- **App 通话（LINE、WhatsApp、FaceTime）：** 手机不会告诉蓝牙设备这类电话在响，所以蓝牙电脑没法替你接。要让 Bageo 也接这类电话，请打开“自动接听来电”并设为 5 秒（iPhone 在“通话音频路由”里）。手机接起后把声音送过来，Bageo 就会接手。

## 电话怎么处理

- 普通电话响约 3 秒，Bageo 接起来，并拿到来电号码。
- Bageo 说完会自己挂断。在 Bageo 黑名单上的号码会直接拒接。
- 你自己打出去的、或你在手机上自己接起的电话，会留在手机上。想从 Bageo 手里接过电话，把那通电话的声音切回手机就行。

## 限制

- 一次只能接一部手机。
- 声音是普通电话的音质。
- App 通话拿不到来电号码。
- 双卡手机：程序看不出是哪张卡接的。号码国家判断错时，在 `/etc/default/bageo-bluetooth` 设置 `SIM_COUNTRY`（例如 `us`）。
- 有 PipeWire 或 PulseAudio 的桌面 Linux：它们也会用蓝牙免提。请另外插一个 USB 蓝牙，或关掉它们的免提功能。

## 确认是否在运行

```sh
journalctl -u bageo-bluetooth -f
```

看到“Phone connected”表示手机连上了，每通电话的步骤都会列出来（号码只显示一部分）。Bageo 控制台也会看到这部手机。设置在 `/etc/default/bageo-bluetooth`，改完要重启服务。

## 卸载

```sh
sudo ./uninstall.sh
```

## 许可

`AGPL-3.0-only` 或商业许可，和 bageo-call 一样。详见 [LICENSING.md](LICENSING.md)。
