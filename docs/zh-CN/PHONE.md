# 手机设置：用蓝牙接手机的电话

> **开发中。** 已经用真实电话测试过、可以使用，但还在持续修改。重要的电话先不要只靠它。

手机保持普通的 iPhone 或 Android，不用装 App，也不用 Root。一台有蓝牙的 Linux 电脑（树莓派、迷你主机、家里的服务器）会像汽车一样和手机配对。有电话打来时，它响几秒就接起来，交给 Bageo。Bageo 能看到来电号码，会和对方说话，说完自己挂断。AI（Gemini Live）通过 Bageo 运行；蓝牙电脑本身不做 AI 的事。

负责这件事的程序在 [`bluetooth/`](../../bluetooth/README.zh-CN.md)。这一页说明：

- 它能做什么、需要什么
- 怎么安装、怎么连到 Bageo
- 配对 iPhone 或 Android 手机
- 普通手机模式
- 手机设置与使用须知

## 能做什么

| 工作 | 做法 |
|---|---|
| 接听来电 | 响约 3 秒（按 Bageo 的要求）后由 Bageo 接起，并拿到来电号码。 |
| 挂断与拒接 | Bageo 说完会自己挂断；在 Bageo 黑名单上的号码直接拒接。 |
| 切换 AI 接听和普通手机 | 控制台一个开关。普通手机模式下，电话留在手机上。见[普通手机模式](#普通手机模式)。 |
| 替 Bageo 打电话 | AI 回电和 AI 代打，都会通过这部手机拨出去。 |
| 一次只接一通 | AI 通话中又有电话打进来，会以忙线拒接，Bageo 会留下记录。 |
| 不碰你自己的电话 | 你自己打出去、或在手机上自己接起的电话，留在手机上。想从 Bageo 手里接过电话，把那通电话的声音切回手机就行。 |

## 需要什么

- **Bageo。** 可以运行在蓝牙电脑上，也可以运行在另一台电脑（Windows、Mac、Linux）或 Docker 里。
- **一台有蓝牙、放在手机附近（约 10 米内）的 Linux 电脑**：Debian、Ubuntu 或 Raspberry Pi OS。内置蓝牙或 USB 蓝牙都可以。
- Windows 和 Mac 不能当蓝牙电脑：Windows 不让程序通过蓝牙接听、挂断电话，Mac 目前还不支持。Bageo 本身照样可以装在上面。

蓝牙做不到的事：短信、同时接多部手机、比普通电话更好的音质。急事的做法见[白名单](FEATURES.md#联系人的四种名单与白名单转接)。

## 安装

**用下载文件（Linux）：** 从 [Releases](https://github.com/h4lo-void/bageo-call-bluetooth/releases) 下载 `bageo-call-bluetooth-<版本>-linux-x64.tar.gz` 并解压，里面有 Bageo 和蓝牙程序：

```sh
./start.sh                    # 启动 Bageo，控制台在 http://localhost:8787/
sudo ./bluetooth/install.sh   # 加上蓝牙接听
```

**用源代码：**

```sh
git clone https://github.com/h4lo-void/bageo-call-bluetooth.git
cd bageo-call-bluetooth/bluetooth
sudo ./install.sh
```

这会安装 BlueZ 和几个 Python 包，把蓝牙设成免提设备，并启动 `bageo-bluetooth` 服务。

## 连到 Bageo

- **Bageo 在同一台电脑：** 不用做什么。
- **Bageo 在另一台电脑或 Docker：** 用 IP 地址配对。
  1. 控制台打开“系统”页，在“**用 IP 地址配对手机**”点“**生成配对码**”。配对码 10 分钟内有效，只能用一次。
  2. 在蓝牙电脑上运行下面的命令（换成 Bageo 的 IP 地址和页面上的配对码）：

```sh
sudo bageo-bluetooth link 192.168.1.20 123456
sudo systemctl restart bageo-bluetooth
```

蓝牙电脑会记住这个地址，重启后自己连回去。两台电脑要在同一个网络，“**手机也能用 Wi-Fi 连接**”（“设置 → 高级”）要开着：Bageo 要你打开时就打开，再生成一次配对码。再生成一次会换掉旧的配对码，输错 5 次这组就作废。Windows 第一次使用时，防火墙会问 Bageo 能不能用网络：请允许“**专用网络**”。

## 配对手机

- **Android：** 服务启动后 10 分钟内，打开“设置 → 蓝牙 → 配对新设备”，点“Bageo HF”，再点“配对”。
- **iPhone：** iPhone 的列表不会显示电脑，要从蓝牙电脑这边配对。先在 iPhone 打开“设置 → 蓝牙”并停在这一页，然后运行：

```sh
sudo bageo-bluetooth pair                      # 列出附近的手机
sudo bageo-bluetooth pair 12:34:56:78:9A:BC    # 再到 iPhone 上点“配对”
```

配对好之后，手机会自己连回来。控制台的“系统 → 手机”会看到它，也可以在那里忘记这部手机。

## 普通手机模式

手机有两种模式，除了通话中，随时都能切换：

| 模式 | 会怎样 |
|---|---|
| **由 AI 接听**（默认） | 电话响约 3 秒，然后由 Bageo 接起。 |
| **普通手机** | Bageo 不碰电话。电话在手机上响，由你自己接。 |

打开控制台，到“系统 → 手机”，点那部手机卡片上的按钮。下一通电话开始生效，重启后也会记得。

想一次停掉所有手机的 AI，在“基本”关掉“由 AI 接听来电”（来电会被接起来再马上挂断）。

## 手机设置

- **Android：** 同一部手机不要再开别的自动接电话的 App，否则它会自己去接电话。
- **iPhone：** “设置 → 辅助功能 → 触控 → 通话音频路由”保持“自动”。
- **App 通话（LINE、WhatsApp、FaceTime）：** 手机不会告诉蓝牙设备这类电话在响，所以蓝牙电脑没法替你接。要让 Bageo 也接这类电话，请打开“自动接听来电”并设为 5 秒（iPhone 在“通话音频路由”里）。手机接起后把声音送过来，Bageo 就会接手。App 通话拿不到来电号码。

## 使用须知

- 一次只能接一部手机，手机要在蓝牙电脑附近。
- 双卡手机：程序看不出是哪张卡接的。号码国家判断错时，在 `/etc/default/bageo-bluetooth` 设置 `SIM_COUNTRY`（例如 `us`）。也可以在控制台手动填这张卡的号码。
- AI 代打和回电要知道号码国家才会用这部手机。日志显示“number's country: unknown”时，也请设置 `SIM_COUNTRY`。
- 有 PipeWire 或 PulseAudio 的桌面 Linux：它们也会用蓝牙免提。请另外插一个 USB 蓝牙，或关掉它们的免提功能。
- 设置在 `/etc/default/bageo-bluetooth`，改完要重启服务。

## 确认是否在运行

```sh
journalctl -u bageo-bluetooth -f
```

看到“Phone connected”表示手机连上了，每通电话的步骤都会列出来（号码只显示一部分）。控制台也会看到这部手机。

## 卸载

```sh
sudo ./bluetooth/uninstall.sh
```

## 遇到问题

见[疑难解答](TROUBLESHOOTING.md)和[蓝牙说明](../../bluetooth/README.zh-CN.md)。

<!-- SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call> -->
