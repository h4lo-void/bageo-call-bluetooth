# Bageo 蓝牙

> **开发中。** 已经用真实电话测试过、可以使用，但还在持续修改。重要的电话先不要只靠它（见[免责声明](#免责声明)）。

让 [Bageo](https://github.com/h4lo-void/bageo-call) 通过蓝牙接你手机的电话。iPhone、Android 都可以，手机不用装 App，也不用 Root。

一台有蓝牙的 Linux 电脑（树莓派、迷你主机、家里的服务器）会像汽车一样和手机配对。有电话打来时，它响几秒就接起来，交给 Bageo。Bageo 能看到来电号码，会和对方说话，说完自己挂断。

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md)

## 能做什么

- 接听来电：响约 3 秒后由 Bageo 接起，并拿到来电号码。
- Bageo 说完会自己挂断；在 Bageo 黑名单上的号码直接拒接。
- 在 Bageo 控制台切换“AI 接听”或“一般手机”。一般手机模式下，电话留在手机上给你接。
- Bageo 的 AI 代打和回电，会通过这部手机拨出去。
- AI 通话中又有电话打进来，会以忙线拒接，Bageo 会留下记录。
- 你自己打出去、或在手机上自己接起的电话，留在手机上。想从 Bageo 手里接过电话，把那通电话的声音切回手机就行。

## 还不能做

- 短信：蓝牙免提没法收发短信，预约短信和短信转发仍要用 Bageo 的手机 App。
- 有急事时接通你的三方通话（转接）：Bageo 会改为告诉对方你会尽快回电。
- 同时接多部手机，以及比普通电话更好的音质。
- 用 Windows 或 Mac 当蓝牙电脑。

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
- **App 通话（LINE、WhatsApp、FaceTime）：** 手机不会告诉蓝牙设备这类电话在响，所以蓝牙电脑没法替你接。要让 Bageo 也接这类电话，请打开“自动接听来电”并设为 5 秒（iPhone 在“通话音频路由”里）。手机接起后把声音送过来，Bageo 就会接手。App 通话拿不到来电号码。

## 使用须知

- 一次只能接一部手机，手机要在蓝牙电脑附近。
- 双卡手机：程序看不出是哪张卡接的。号码国家判断错时，在 `/etc/default/bageo-bluetooth` 设置 `SIM_COUNTRY`（例如 `us`）。
- AI 代打和回电要知道号码国家才会用这部手机。日志显示“number's country: unknown”时，也请设置 `SIM_COUNTRY`。
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

## 免责声明

本项目是自托管软件，按“现状”提供、不附任何担保；作者不运营任何服务，也不会收到你的数据。部署的人要自行遵守当地关于通话录音、AI 身份披露和自动拨号的法律，并自负电信与 API 费用。请不要把它当成紧急求救、医疗或安全相关的唯一联系方式。使用前请先读 [LEGAL.md](LEGAL.md)；安全注意事项见 [SECURITY.md](SECURITY.md)。本项目与 Apple、Google 及 Bluetooth SIG 无关；iPhone、Android 与 Bluetooth 为其商标。

## 许可

`AGPL-3.0-only` 或商业许可，和 bageo-call 一样。详见 [LICENSING.md](LICENSING.md)。
