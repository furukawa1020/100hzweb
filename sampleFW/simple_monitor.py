"""
シンプルなリアルタイムHRモニター（グラフなし・軽量版）
"""
import serial
import numpy as np
from collections import deque
import time
import sys

class SimpleMonitor:
    def __init__(self, port='COM3', baudrate=115200):
        self.port = port
        self.baudrate = baudrate
        self.signal_buffer = deque(maxlen=300)
        self.timestamp_buffer = deque(maxlen=300)
        
    def detect_peaks(self, signal_data, min_distance=50):
        """簡易ピーク検出"""
        if len(signal_data) < min_distance * 2:
            return []
        
        peaks = []
        threshold = np.std(signal_data) * 0.3
        
        for i in range(min_distance, len(signal_data) - min_distance):
            if signal_data[i] > threshold:
                is_peak = True
                for j in range(i - min_distance, i + min_distance):
                    if j != i and signal_data[j] >= signal_data[i]:
                        is_peak = False
                        break
                if is_peak:
                    peaks.append(i)
        
        return peaks
    
    def calculate_hr_rmssd(self):
        """心拍数とRMSSDを計算"""
        if len(self.signal_buffer) < 200:
            return None, None, None
        
        signal_data = np.array(list(self.signal_buffer))
        timestamps = np.array(list(self.timestamp_buffer))
        
        # ピーク検出
        peaks = self.detect_peaks(signal_data, min_distance=50)
        
        if len(peaks) < 4:
            return None, None, len(peaks)
        
        # R-R間隔計算
        rr_intervals = np.diff(timestamps[peaks])
        
        # 有効なR-R間隔のみ
        valid_rr = rr_intervals[(rr_intervals >= 400) & (rr_intervals <= 1500)]
        
        if len(valid_rr) < 3:
            return None, None, len(peaks)
        
        # HR計算
        mean_rr = np.mean(valid_rr)
        hr = 60000.0 / mean_rr if mean_rr > 0 else 0
        
        # RMSSD計算
        rr_diff = np.diff(valid_rr)
        rmssd = np.sqrt(np.mean(rr_diff ** 2)) if len(rr_diff) > 0 else 0
        
        return hr, rmssd, len(peaks)
    
    def parse_csv_line(self, line):
        """CSVデータ行をパース"""
        try:
            parts = line.strip().split(',')
            if len(parts) == 6:
                return {
                    'timestamp': int(parts[1]),
                    'filt_ir': float(parts[5])
                }
        except:
            pass
        return None
    
    def run(self):
        """リアルタイムモニタリング開始"""
        print(f"=== PPGclip Simple Monitor ===")
        print(f"Connecting to {self.port}...")
        
        try:
            ser = serial.Serial(self.port, self.baudrate, timeout=0.5)
            print("Connected!")
            print("\n待機中... ATOMS3のボタンを押してください")
            print("-" * 60)
            
            in_data_mode = False
            sample_count = 0
            last_hr_time = time.time()
            
            while True:
                line = ser.readline().decode('utf-8', errors='ignore').strip()
                
                if not line:
                    continue
                
                # データモード開始検出
                if line.startswith('# Index'):
                    in_data_mode = True
                    sample_count = 0
                    self.signal_buffer.clear()
                    self.timestamp_buffer.clear()
                    print("\n[データ収集開始]")
                    print("-" * 60)
                    continue
                
                # データモード終了検出
                if 'Buffer cleared' in line:
                    if in_data_mode:
                        print(f"\n[データ収集終了] 合計 {sample_count} サンプル")
                        print("-" * 60)
                        print("\n待機中... ATOMS3のボタンを押してください")
                        print("-" * 60)
                    in_data_mode = False
                    continue
                
                # データ行処理
                if in_data_mode:
                    data = self.parse_csv_line(line)
                    if data:
                        self.signal_buffer.append(data['filt_ir'])
                        self.timestamp_buffer.append(data['timestamp'])
                        sample_count += 1
                        
                        # 1秒ごとに表示更新
                        current_time = time.time()
                        if current_time - last_hr_time >= 1.0 and sample_count >= 300:
                            hr, rmssd, peaks = self.calculate_hr_rmssd()
                            
                            if hr is not None and 40 <= hr <= 120:
                                # プログレスバー
                                progress = min(int(sample_count / 60), 100)
                                bar = '█' * (progress // 2) + '░' * (50 - progress // 2)
                                
                                # 1行で表示（上書き）
                                sys.stdout.write(f'\r[{bar}] {sample_count:4d}/6000 | '
                                               f'HR: {hr:5.1f} bpm | '
                                               f'RMSSD: {rmssd:5.1f} ms | '
                                               f'Peaks: {peaks:3d}')
                                sys.stdout.flush()
                            
                            last_hr_time = current_time
        
        except KeyboardInterrupt:
            print("\n\n停止しました")
        except Exception as e:
            print(f"\nエラー: {e}")
        finally:
            if 'ser' in locals() and ser.is_open:
                ser.close()
            print("接続を終了しました")

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description='PPGclip Simple Monitor')
    parser.add_argument('--port', default='COM3', help='Serial port')
    parser.add_argument('--baudrate', type=int, default=115200, help='Baud rate')
    
    args = parser.parse_args()
    
    monitor = SimpleMonitor(args.port, args.baudrate)
    monitor.run()
