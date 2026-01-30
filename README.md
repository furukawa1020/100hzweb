
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
