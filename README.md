# PPG Clip

<img src="https://github.com/akita11/PPGclip/blob/main/images/PPGclip1.jpg" width="240px">

<img src="https://github.com/akita11/PPGclip/blob/main/images/PPGclip2.jpg" width="240px">

[M5Stack社の心拍センサユニット](https://www.switch-science.com/products/5695)を利用した、測定精度・安定度の高い透過型の光学脈波(PPG)計測器です。
マイコン部はM5Stack社のATOMシリーズを取り付けらて一体化できます。
部品セットからこれらを組み立てることができます。

## 必要なもの
- 部品セット（筐体（4点）、LED基板、コネクタ基板、4p/2mmピンソケット×2、ケーブル（色が写真のものと異なる場合があります）、M2.3×8mmタッピングねじ、M2×4mmねじ、押しバネ0.7×7.5×23）
- [M5Stack社の心拍センサユニット](https://www.switch-science.com/products/5695)
- M5Stack社ATOMシリーズ（ディスプレイのついている[ATOMS3](https://www.switch-science.com/products/8670)が脈波確認などができて便利。ATOMS3用のサンプルファームウエアあり）
- 工具（1.5mm六角レンチ、小プラスドライバ）


<img src="https://github.com/akita11/PPGclip/blob/main/images/PPGclip_parts.jpg" width="240px">


# 組み立て

1. 心拍センサユニットを、1.5mm六角レンチを使って分解し、中のボード(1)を取り出します。
1. LED基板(2)とボード(1)を、ケーブル(3)でFig1のようにはんだ付けして接続します。ケーブルの対応関係を間違えないように注意してください。このとき、ボード(1)側は、ケーブルを横側に引き出し、はんだづけ箇所がなるべく平坦になるようにします。
1. コネクタ基板(4)に、4p/2mmピンソケット(5)を2個、傾かないように注意してはんだ付けし、足をなるべく短くカットします。（Fig2）
1. ボード(1)にコネクタ基板(4)を差し込み、Fig3のように筐体(6）にはめます。ケーブル(3)は、図のように上面基板に重ならないようにします。
# VASC-LAB 100Hz PMDT System

**Perceptual Micro-Decision Task (PMDT) with 100Hz Physiological Streaming**

This system connects a raw physiological sensor stream (M5Atom S3) to a high-performance Rust/WASM frontend to study the interaction between physiological arousal and micro-decision making.

## System Architecture

1.  **Firmware (M5Atom S3)**
    *   Streams raw IR/Red sensor data at 100Hz.
    *   Transmits filtered "Teacher's Waveform" for visual feedback.
    *   (Source code not included / proprietary).

2.  **Relay Server (Python)**
    *   Bridges Serial Port (USB) to WebSocket.
    *   Logs high-precision physiological data to `data/session_*.csv`.
    *   Logs experimental event data to `data/events_*.csv`.

3.  **Frontend (Rust / Leptos / WASM)**
    *   **PMDT Task**: Continuous binary micro-decision task (Visual Density Discrimination).
    *   **Dynamics**:
        *   **Threat Frame**: High physiological arousal -> Visual noise increases, timeout decreases.
        *   **Challenge Frame**: High physiological arousal -> Visual clarity increases, timeout increases.
    *   **Signal Processing**: 100Hz real-time filtering and `r(t)` (Vascular Tone) calculation in WASM.

## Usage

### 1. Start Relay Server
Connect M5Atom via USB.

```bash
cd relay_server
python relay_server.py
```

### 2. Start Frontend
Required: `trunk` (Rust WASM bundler).

```bash
cd frontend
trunk serve
```
Open `http://localhost:8080`.

## Controls

*   **SPACE**: Start Session.
*   **A**: Choose Left (Higher Density).
*   **L**: Choose Right (Higher Density).
*   **Debug Keys**:
    *   `1`: Neutral Frame
    *   `2`: Threat Frame
    *   `3`: Challenge Frame

## License
Private / Confidential - Do not publish without permission.
