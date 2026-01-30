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
M5AtomをUSB接続してください。

```bash
cd relay_server
python relay_server.py
```

### 2. フロントエンドの起動
要件: `trunk` (Rust WASMバンドラ)。

```bash
cd frontend
trunk serve
```
ブラウザで `http://localhost:8080` を開きます。

## 操作方法

*   **SPACE**: セッション開始
*   **A**: 左を選択（密度が高い）
*   **L**: 右を選択（密度が高い）
*   **デバッグ用キー**:
    *   `1`: Neutral Frame（通常）
    *   `2`: Threat Frame（脅威）
    *   `3`: Challenge Frame（挑戦）

## ライセンス
Private / Confidential - 無断転載・公開を禁じます。
