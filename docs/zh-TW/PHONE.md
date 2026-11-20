# 手機設定：用藍牙接手機的電話

> **開發中。** 已經用真的電話測試過、可以使用，但還在持續修改。重要的電話先不要只靠它。

手機維持一般的 iPhone 或 Android，不用裝 App、也不用 Root。一台有藍牙的 Linux 電腦（樹莓派、迷你電腦、家裡的伺服器）會像車子一樣跟手機配對。有電話打來時，它響幾秒就接起來，交給 Bageo。Bageo 看得到來電號碼，會跟對方講話，講完自己掛斷。AI（Gemini Live）透過 Bageo 運作；藍牙電腦本身不做 AI 的事。

負責這件事的程式在 [`bluetooth/`](../../bluetooth/README.zh-TW.md)。這一頁說明：

- 它能做什麼、需要什麼
- 怎麼安裝、怎麼連到 Bageo
- 配對 iPhone 或 Android 手機
- 一般手機模式
- 手機設定與使用須知

## 能做什麼

| 工作 | 做法 |
|---|---|
| 接聽來電 | 響約 3 秒（依 Bageo 的要求）後由 Bageo 接起，並拿到來電號碼。 |
| 掛斷與拒接 | Bageo 講完會自己掛斷；在 Bageo 黑名單上的號碼直接拒接。 |
| 切換 AI 接聽和一般手機 | 控制台一個開關。一般手機模式下，電話留在手機上。見[一般手機模式](#一般手機模式)。 |
| 替 Bageo 打電話 | AI 回電和 AI 代打，都會透過這支手機撥出去。 |
| 一次只接一通 | AI 通話中又有電話打進來，會以忙線拒接，Bageo 會留下紀錄。 |
| 不碰你自己的電話 | 你自己打出去、或在手機上自己接起的電話，留在手機上。想從 Bageo 手上接過電話，把那通電話的聲音切回手機就好。 |

## 需要什麼

- **Bageo。** 可以跑在藍牙電腦上，也可以跑在另一台電腦（Windows、Mac、Linux）或 Docker。
- **一台有藍牙、放在手機附近（約 10 公尺內）的 Linux 電腦**：Debian、Ubuntu 或 Raspberry Pi OS。內建藍牙或 USB 藍牙都可以。
- Windows 和 Mac 不能當藍牙電腦：Windows 不讓程式透過藍牙接聽、掛斷電話，Mac 目前還不支援。Bageo 本身照樣可以裝在它們上面。

藍牙做不到的事：簡訊、同時接多支手機、比一般電話更好的音質。急事的做法見[白名單](FEATURES.md#聯絡人的四種名單與白名單轉接)。

## 安裝

**用下載檔（Linux）：** 從 [Releases](https://github.com/h4lo-void/bageo-call-bluetooth/releases) 下載 `bageo-call-bluetooth-<版本>-linux-x64.tar.gz` 並解壓，裡面有 Bageo 和藍牙程式：

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
- **Bageo 在另一台電腦或 Docker：** 用 IP 位址配對。
  1. 控制台打開「系統」頁，在「**用 IP 位址配對手機**」按「**產生配對碼**」。配對碼 10 分鐘內有效，只能用一次。
  2. 在藍牙電腦上執行下面的指令（換成 Bageo 的 IP 位址和畫面上的配對碼）：

```sh
sudo bageo-bluetooth link 192.168.1.20 123456
sudo systemctl restart bageo-bluetooth
```

藍牙電腦會記住這個位址，重開機後自己連回去。兩台電腦要在同一個網路，「**手機也能用 Wi-Fi 連**」（「設定 → 進階」）要開著：Bageo 要你打開時就打開，再產生一次配對碼。再產生一次會換掉舊的配對碼，輸錯 5 次那組就作廢。Windows 第一次使用時，防火牆會問 Bageo 能不能用網路：請允許「**私人網路**」。

## 配對手機

- **Android：** 服務啟動後 10 分鐘內，打開「設定 → 藍牙 → 配對新裝置」，點「Bageo HF」，再按「配對」。
- **iPhone：** iPhone 的清單不會列出電腦，要從藍牙電腦這邊配對。先在 iPhone 打開「設定 → 藍牙」並停在那一頁，然後執行：

```sh
sudo bageo-bluetooth pair                      # 列出附近的手機
sudo bageo-bluetooth pair 12:34:56:78:9A:BC    # 再到 iPhone 上按「配對」
```

配對好之後，手機會自己連回來。控制台的「系統 → 手機」會看到它，也可以在那裡忘記這支手機。

## 一般手機模式

手機有兩種模式，除了通話中，隨時都能切換：

| 模式 | 會怎樣 |
|---|---|
| **由 AI 接聽**（預設） | 電話響約 3 秒，然後由 Bageo 接起。 |
| **一般手機** | Bageo 不碰電話。電話在手機上響，由你自己接。 |

打開控制台，到「系統 → 手機」，按那支手機卡片上的按鈕。下一通電話開始生效，重開機後也會記得。

想一次停掉所有手機的 AI，在「基本」關掉「由 AI 接聽來電」（來電會被接起再馬上掛斷）。

## 手機設定

- **Android：** 同一支手機不要再開別的自動接電話的 App，不然它會自己去接電話。
- **iPhone：** 「設定 → 輔助使用 → 觸控 → 通話聲音路徑」維持「自動」。
- **App 通話（LINE、WhatsApp、FaceTime）：** 手機不會告訴藍牙裝置這類電話在響，所以藍牙電腦沒辦法替你接。要讓 Bageo 也接這類電話，請打開「自動接聽來電」並設成 5 秒（iPhone 在「通話聲音路徑」裡面）。手機接起後把聲音送過來，Bageo 就會接手。App 通話拿不到來電號碼。

## 使用須知

- 一次只能接一支手機，手機要在藍牙電腦附近。
- 雙卡手機：程式看不出是哪張卡接的。門號國家判斷錯時，在 `/etc/default/bageo-bluetooth` 設 `SIM_COUNTRY`（例如 `us`）。也可以在控制台手動填這張卡的門號。
- AI 代打和回電要知道門號國家才會用這支手機。紀錄顯示「number's country: unknown」時，也請設 `SIM_COUNTRY`。
- 有 PipeWire 或 PulseAudio 的桌面 Linux：它們也會用藍牙免持。請另外插一支 USB 藍牙，或關掉它們的免持功能。
- 設定在 `/etc/default/bageo-bluetooth`，改完要重開服務。

## 確認有沒有在運作

```sh
journalctl -u bageo-bluetooth -f
```

看到「Phone connected」代表手機連上了，每通電話的步驟都會列出來（號碼只顯示一部分）。控制台也會看到這支手機。

## 移除

```sh
sudo ./bluetooth/uninstall.sh
```

## 遇到問題

見[疑難排解](TROUBLESHOOTING.md)和[藍牙說明](../../bluetooth/README.zh-TW.md)。

<!-- SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call> -->
