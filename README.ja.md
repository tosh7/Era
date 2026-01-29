# Era

RustによるiOS Simulator CLIツール

## 概要

EraはiOS Simulatorを効率的に操作するためのコマンドラインツールです。`xcrun simctl`のラッパーとして機能し、シンプルなコマンドでシミュレータの起動、アプリのインストール、スクリーンショットの撮影などが行えます。

## インストール

```bash
cargo install era
```

### 必要要件

- macOS
- Xcode（iOS Simulatorを含む）
- Rust 1.70以上

## コマンド一覧

### list - シミュレータ一覧の表示

利用可能なシミュレータの一覧を表示します。

```bash
# 全てのシミュレータを表示
era list

# 起動中のシミュレータのみ表示
era list --booted
```

### boot - シミュレータの起動

指定したシミュレータを起動します。

```bash
era boot <DEVICE_ID>

# 例
era boot "iPhone 16 Pro"
era boot 12345678-1234-1234-1234-123456789ABC
```

### shutdown - シミュレータの終了

指定したシミュレータを終了します。

```bash
era shutdown <DEVICE_ID>

# 全てのシミュレータを終了
era shutdown all
```

### install - アプリのインストール

シミュレータにアプリをインストールします。

```bash
era install -d <DEVICE_ID> <APP_PATH>

# 例
era install -d booted ./MyApp.app
era install -d "iPhone 16 Pro" /path/to/MyApp.app
```

### launch - アプリの起動

インストール済みのアプリを起動します。

```bash
era launch -d <DEVICE_ID> <BUNDLE_ID>

# 例
era launch -d booted com.example.myapp
```

### screenshot - スクリーンショットの撮影

シミュレータのスクリーンショットを撮影します。

```bash
era screenshot -d <DEVICE_ID> <OUTPUT_PATH>

# 例
era screenshot -d booted ./screenshot.png
era screenshot -d "iPhone 16 Pro" ~/Desktop/screen.png
```

### input - キーボード入力の送信

シミュレータにキー入力を送信します。

```bash
era input -d <DEVICE_ID> -k <KEY_TYPE>

# 利用可能なキータイプ
# home      - ホームボタン
# lock      - ロックボタン
# return    - Return/Enterキー
# volume-up - 音量上げ
# volume-down - 音量下げ
# shake     - シェイクジェスチャー

# 例
era input -d booted -k home
era input -d booted -k volume-up
```

### openurl - URLを開く

シミュレータでURLを開きます。

```bash
era openurl -d <DEVICE_ID> -u <URL>

# 例
era openurl -d booted -u "https://example.com"
era openurl -d booted -u "myapp://deeplink"
```

### tap - 画面のタップ

指定した座標をタップします（IDB必要）。

```bash
era tap -d <DEVICE_ID> -x <X座標> -y <Y座標> [--scale <スケール係数>]

# 例（ポイント座標）
era tap -d booted -x 200 -y 400

# 例（ピクセル座標を使用する場合）
# --scale オプションでピクセルからポイントへ自動変換
# 例: 1260px / 3 (スケール係数) = 420 ポイント
era tap -d booted -x 1260 -y 2736 --scale 3
```

### swipe - スワイプ操作

指定した座標間をスワイプします（IDB必要）。

```bash
era swipe -d <DEVICE_ID> --start-x <X1> --start-y <Y1> --end-x <X2> --end-y <Y2> [--scale <スケール係数>]

# 例（上にスワイプ、ポイント座標）
era swipe -d booted --start-x 200 --start-y 600 --end-x 200 --end-y 200

# 例（ピクセル座標を使用する場合）
era swipe -d booted --start-x 300 --start-y 1500 --end-x 300 --end-y 600 --scale 3
```

### 座標変換について

`--scale` オプションを使用すると、ピクセル座標からポイント座標への自動変換が行われます。スクリーンショットツールやUI検査ツールから取得したピクセル値を直接使用できます。

| デバイス | スケール係数 |
|---------|-------------|
| 標準ディスプレイ（iPhone SE等） | 2 |
| Super Retinaディスプレイ（iPhone 16 Pro等） | 3 |

計算式: `ポイント = ピクセル / スケール係数`

### enumerate - 入力デバイスの列挙

利用可能な入力デバイスを表示します。

```bash
era enumerate -d <DEVICE_ID>

# 例
era enumerate -d booted
```

## IDB連携（オプション）

`tap`や`swipe`コマンドはFacebookのIDB（iOS Development Bridge）を使用します。IDBはオプショナルな依存関係であり、インストールされていない場合はエラーメッセージが表示されます。

### IDBのインストール

```bash
brew install idb-companion
```

### IDBが提供する機能

- `tap` - 画面タップ
- `swipe` - スワイプジェスチャー
- `text` - テキスト入力（将来実装予定）

IDBがインストールされていない場合でも、`list`、`boot`、`shutdown`、`install`、`launch`、`screenshot`、`input`、`openurl`、`enumerate`コマンドは通常通り使用できます。

