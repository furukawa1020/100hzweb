# ATOMS3 トラブルシューティングガイド

## 問題: シリアルポートにアクセスできない / データが表示されない

### 解決手順:

1. **ATOMS3のUSBケーブルを抜く**
   - PCからATOMS3のUSBケーブルを抜いてください
   - 5秒待つ

2. **VSCodeで開いているシリアルモニターを閉じる**
   - PlatformIOのシリアルモニターが開いていれば閉じる
   - 他のシリアル接続ツール(Arduino IDE, Teratermなど)が開いていれば閉じる

3. **すべてのPythonプロセスを終了**
   ```powershell
   Get-Process python -ErrorAction SilentlyContinue | Stop-Process -Force
   ```

4. **ATOMS3を再接続**
   - USBケーブルをPCに再接続
   - デバイスマネージャーでCOM3が認識されているか確認

5. **ファームウェアが正しく動作しているか確認**
   - ATOMS3のボタンを押したときにLEDが反応するか確認
   - ファームウェアが書き込まれていない場合は再アップロードが必要

6. **HRVアナライザーを起動**
   ```powershell
   python sampleFW\hrv_analyzer.py --mode live --port COM3 --plot
   ```

7. **ボタンを押す**
   - ATOMS3のボタンを押してデータ送信を開始

## ファームウェア再アップロード（必要な場合）

PlatformIO拡張機能を使用:
1. VSCodeでsampleFWフォルダを開く
2. 下のステータスバーの「→」(Upload)アイコンをクリック
3. アップロード完了後、ATOMS3がリセットされる

## まだ動作しない場合

1. COM3が正しいポート番号か確認:
   ```powershell
   Get-WmiObject Win32_SerialPort | Select-Object DeviceID, Description
   ```

2. 別のUSBポートを試す

3. ATOMS3を手動でリセット:
   - RST/RESETボタンを押す（あれば）
   - USBケーブルを抜き差し
