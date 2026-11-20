# Bageo 藍牙

> **開發中。** 已經用真的電話測試過、可以使用，但還在持續修改。重要的電話先不要只靠它（見[免責聲明](#免責聲明)）。

讓 [Bageo](../README.zh-TW.md) 透過藍牙接你手機的電話。iPhone、Android 都可以，手機不用裝 App、也不用 Root。

一台有藍牙的 Linux 電腦（樹莓派、迷你電腦、家裡的伺服器）會像車子一樣跟手機配對。有電話打來時，它響幾秒就接起來，交給 Bageo。Bageo 看得到來電號碼，會跟對方講話，講完自己掛斷。

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md)

## 能做什麼

- 接聽來電：響約 3 秒後由 Bageo 接起，並拿到來電號碼。
- Bageo 講完會自己掛斷；在 Bageo 黑名單上的號碼直接拒接。
- 在 Bageo 控制台切換「AI 接聽」或「一般手機」。一般手機模式下，電話留在手機上給你接。
- Bageo 的 AI 代打和回電，會透過這支手機撥出去。
- AI 通話中又有電話打進來，會以忙線拒接，Bageo 會留下紀錄。
- 你自己打出去、或在手機上自己接起的電話，留在手機上。想從 Bageo 手上接過電話，把那通電話的聲音切回手機就好。

## 還不能做

- 簡訊：藍牙免持沒辦法收發簡訊，預約簡訊和簡訊轉發仍要用 Bageo 的手機 App。
- 有急事時接通你的三方通話（轉接）：Bageo 會改成告訴對方你會盡快回電。
- 同時接多支手機，以及比一般電話更好的音質。
- 用 Windows 或 Mac 當藍牙電腦。

## 需要什麼

- Bageo：就在這個 repo 和每個下載檔裡。可以跑在藍牙電腦上，也可以跑在另一台電腦（Windows、Mac、Linux）或 Docker。
- 一台有藍牙、放在手機附近（約 10 公尺內）的 Linux 電腦：Debian、Ubuntu 或 Raspberry Pi OS。內建藍牙或 USB 藍牙都可以。
- Windows 和 Mac 不能當藍牙電腦：Windows 不讓程式透過藍牙接聽、掛斷電話，Mac 目前還不支援。Bageo 本身照樣可以裝在它們上面。

## 安裝

**用下載檔（Linux）：** 從 [Releases](https://github.com/h4lo-void/bageo-call-bluetooth/releases) 下載 `bageo-call-bluetooth-<版本>-linux-x64.tar.gz` 並解壓，裡面有 Bageo 和這個程式：

```sh
./start.sh                    # 啟動 Bageo，控制台在 http://localhost:8787/
sudo ./bluetooth/install.sh   # 加上藍牙接聽
```

**用原始碼：**

```sh
git clone https://github.com/h4lo-void/bageo-call-bluetooth.git
cd bageo-call-bluetooth/bluetooth
sudo ./install.sh
```

這會安裝 BlueZ 和幾個 Python 套件，把藍牙設成免持裝置，並啟動 `bageo-bluetooth` 服務。

## 連到 Bageo

- **Bageo 在同一台電腦：** 不用做什麼。
- **Bageo 在另一台電腦或 Docker：** 在 Bageo 控制台打開「系統」→「用 IP 位址配對手機」→「產生配對碼」，然後在藍牙電腦上執行（換成 Bageo 的位址和畫面上的配對碼）：

```sh
sudo bageo-bluetooth link 192.168.1.20 123456
sudo systemctl restart bageo-bluetooth
```

如果 Bageo 要你先打開「手機也能用 Wi-Fi 連」，打開後再產生一次配對碼。

## 配對手機

- **Android：** 服務啟動後 10 分鐘內，打開「設定 → 藍牙 → 配對新裝置」，點「Bageo HF」，再按「配對」。
- **iPhone：** iPhone 的清單不會列出電腦，要從藍牙電腦這邊配對。先在 iPhone 打開「設定 → 藍牙」並停在那一頁，然後執行：

```sh
sudo bageo-bluetooth pair                      # 列出附近的手機
sudo bageo-bluetooth pair 12:34:56:78:9A:BC    # 再到 iPhone 上按「配對」
```

配對好之後，手機會自己連回來。

## 手機設定

- **Android：** 同一支手機不要再開 Bageo 的手機 App，不然它會自己去接電話。
- **iPhone：** 「設定 → 輔助使用 → 觸控 → 通話聲音路徑」維持「自動」。
- **App 通話（LINE、WhatsApp、FaceTime）：** 手機不會告訴藍牙裝置這類電話在響，所以藍牙電腦沒辦法替你接。要讓 Bageo 也接這類電話，請打開「自動接聽來電」並設成 5 秒（iPhone 在「通話聲音路徑」裡面）。手機接起後把聲音送過來，Bageo 就會接手。App 通話拿不到來電號碼。

## 使用須知

- 一次只能接一支手機，手機要在藍牙電腦附近。
- 雙卡手機：程式看不出是哪張卡接的。門號國家判斷錯時，在 `/etc/default/bageo-bluetooth` 設 `SIM_COUNTRY`（例如 `us`）。
- AI 代打和回電要知道門號國家才會用這支手機。紀錄顯示「number's country: unknown」時，也請設 `SIM_COUNTRY`。
- 有 PipeWire 或 PulseAudio 的桌面 Linux：它們也會用藍牙免持。請另外插一支 USB 藍牙，或關掉它們的免持功能。

## 確認有沒有在運作

```sh
journalctl -u bageo-bluetooth -f
```

看到「Phone connected」代表手機連上了，每通電話的步驟都會列出來（號碼只顯示一部分）。Bageo 控制台也會看到這支手機。設定在 `/etc/default/bageo-bluetooth`，改完要重開服務。

## 移除

```sh
sudo ./uninstall.sh
```

## 免責聲明

本專案是自架軟體，按「現狀」提供、不附任何擔保；作者不維運任何服務，也不會收到你的資料。架設的人要自行遵守當地關於通話錄音、AI 身分揭露和自動撥號的法律，並自負電信與 API 費用。請不要把它當成緊急求救、醫療或安全相關的唯一聯絡方式。使用前請先讀 [LEGAL.md](../LEGAL.md)；安全注意事項見 [SECURITY.md](../SECURITY.md)。本專案與 Apple、Google 及 Bluetooth SIG 無關；iPhone、Android 與 Bluetooth 為其商標。

## 授權

`AGPL-3.0-only` 或商業授權，跟 bageo-call 一樣。詳見 [LICENSING.md](../LICENSING.md)。
