# VASC-LAB 100Hz PMDT System

**Perceptual Micro-Decision Task (PMDT) with 100Hz Physiological Streaming**

本システムは、100Hzの生体信号ストリーミング（M5Atom S3使用）とRust/WASM製の高性能フロントエンドを接続し、生理的覚醒と微視的意思決定（Micro-Decision）の相互作用を研究するための実験プラットフォームです。

## システム構成

1.  **ファームウェア (M5Atom S3)**
    *   IR/Redの生データを100Hzでストリーミング配信。
    *   視覚フィードバック用の「Teacher's Waveform（フィルター済み波形）」を送信。
    * 

2.  **リレーサーバー (Python)**
    *   シリアルポート(USB)とWebSocketのブリッジ。
    *   高精度な生理データを `data/session_*.csv` に記録。
    *   実験イベントデータ（行動ログ）を `data/events_*.csv` に記録。

3.  **フロントエンド (Rust / Leptos / WASM)**
    *   **PMDTタスク**: 連続二択の微視的意思決定課題（視覚密度判別）。
    *   **ダイナミクス**:
        *   **Threat Frame**: 生理的覚醒が高い → ノイズ増加・制限時間短縮（Vicious Cycle）。
        *   **Challenge Frame**: 生理的覚醒が高い → 視認性向上・制限時間延長（Virtuous Cycle）。
    *   **信号処理**: WASM内での100Hzリアルタイムフィルタリングおよび `r(t)` (血管収縮指標) の算出。

## 使用方法

### 1. リレーサーバーの起動
M5AtomをUSB接続し、依存ライブラリをインストールしてからサーバーを起動します。

```bash
cd relay_server
pip install -r requirements.txt
python relay_server.py
```
※ `No serial port found` と出る場合は M5Atom の接続を確認してください。

### 2. フロントエンドの起動
要件: `trunk` (Rust WASMバンドラ)。

```bash
cd frontend
trunk serve
```
ブラウザ（Chrome推奨）で `http://localhost:8080` を開きます。

## 実験プロトコル（操作マニュアル）

1.  **装着 & 接続**: 被験者にセンサを装着し、波形が安定するまで待ちます。
2.  **ID入力**: 画面左上の入力欄に **被験者ID** (例: `sub01`) を入力します。
3.  **キャリブレーション**: 
    - 被験者を安静にさせ、キーボードの `C` を押します。
    - 現在の HRV (RMSSD) が「基準値」として登録されます。
4.  **実験開始**:
    - `SPACE` キーを押すとセッションが開始します。
    - ログファイルが `data/[ID]_raw_...csv` として保存開始されます。
5.  **タスク実行**:
    - 画面左右の「▲」の密度を比較します。
    - **Aキー**: 左が多い
    - **Lキー**: 右が多い
    - **聴覚修飾**: 心拍音（ビープ）が速くなると「覚醒度が高い（ストレス）」状態を示します。

## 指標について
- **HRV (RMSSD)**: 心拍変動のゆらぎ。低いほどストレス/高覚醒と判定されます。
- **ACC / RT**: 正答率と反応時間が画面右上にリアルタイム表示されます。

## ライセンス
Private / Confidential - 研究室内部使用限定。
