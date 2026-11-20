# Bageo 藍牙

讓 [Bageo](https://github.com/h4lo-void/bageo-call) 透過藍牙接你手機的電話。iPhone、Android 都可以，手機不用裝 App、也不用 Root。

一台有藍牙的 Linux 電腦（樹莓派、迷你電腦、家裡的伺服器）會像車子一樣跟手機配對。有電話打來時，它響幾秒就接起來，交給 Bageo。Bageo 看得到來電號碼，會跟對方講話，講完自己掛斷。

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md)

## 需要什麼

- 一個在執行的 Bageo：裝在這台電腦、另一台電腦或 Docker 都可以。連到另一台電腦要 Bageo 1.6.9 以上。
- 一台有藍牙、放在手機附近（約 10 公尺內）的 Linux 電腦：Debian、Ubuntu 或 Raspberry Pi OS。內建藍牙或 USB 藍牙都可以。
- Windows 和 Mac 不能當藍牙電腦：Windows 不讓程式透過藍牙接聽、掛斷電話，Mac 目前還不支援。Bageo 本身照樣可以裝在它們上面。

## 安裝

```sh
git clone https://github.com/h4lo-void/bageo-call-bluetooth.git
cd bageo-call-bluetooth
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
- **App 通話（LINE、WhatsApp、FaceTime）：** 手機不會告訴藍牙裝置這類電話在響，所以藍牙電腦沒辦法替你接。要讓 Bageo 也接這類電話，請打開「自動接聽來電」並設成 5 秒（iPhone 在「通話聲音路徑」裡面）。手機接起後把聲音送過來，Bageo 就會接手。

## 電話怎麼處理

- 一般電話響約 3 秒，Bageo 接起來，並拿到來電號碼。
- Bageo 講完會自己掛斷。在 Bageo 黑名單上的號碼會直接拒接。
- 你自己打出去的、或你在手機上自己接起的電話，會留在手機上。想從 Bageo 手上接過電話，把那通電話的聲音切回手機就好。

## 限制

- 一次只能接一支手機。
- 聲音是一般電話的音質。
- App 通話拿不到來電號碼。
- 雙卡手機：程式看不出是哪張卡接的。門號國家判斷錯時，在 `/etc/default/bageo-bluetooth` 設 `SIM_COUNTRY`（例如 `us`）。
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

## 授權

`AGPL-3.0-only` 或商業授權，跟 bageo-call 一樣。詳見 [LICENSING.md](LICENSING.md)。
