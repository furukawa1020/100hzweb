"""
PPGclip HRV Analyzer with Database Storage and Visualization
============================================================
シリアルポートからPPGデータを受信し、データベースに保存してHRV解析を行います。

機能:
- リアルタイムデータ受信とデータベース保存
- R-R間隔（心拍間隔）検出
- 詳細なHRV解析（RMSSD, SDNN, pNN50, HR, LF/HF比など）
- リアルタイムグラフ表示
- 過去データの再解析と可視化

使用方法:
    python hrv_analyzer.py --port COM3 --mode live
    python hrv_analyzer.py --mode analyze --session <session_id>
"""

import serial
import sqlite3
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation
from scipy import signal, interpolate
from scipy.fft import fft, fftfreq
from datetime import datetime
import argparse
import json
from collections import deque
import threading
import time
import warnings
warnings.filterwarnings('ignore')


class PPGDatabase:
    """PPGデータとHRV解析結果を保存するデータベース"""
    
    def __init__(self, db_path='ppg_data.db'):
        self.db_path = db_path
        self.conn = sqlite3.connect(db_path)
        self.create_tables()
    
    def create_tables(self):
        """データベーステーブルを作成"""
        cursor = self.conn.cursor()
        
        # セッションテーブル
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS sessions (
                session_id INTEGER PRIMARY KEY AUTOINCREMENT,
                start_time TIMESTAMP,
                end_time TIMESTAMP,
                sample_count INTEGER,
                notes TEXT
            )
        ''')
        
        # 生データテーブル
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS raw_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER,
                timestamp INTEGER,
                red INTEGER,
                ir INTEGER,
                filt_red REAL,
                filt_ir REAL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            )
        ''')
        
        # R-R間隔テーブル
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS rr_intervals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER,
                peak_index INTEGER,
                peak_time REAL,
                rr_interval REAL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            )
        ''')
        
        # HRV解析結果テーブル
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS hrv_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER,
                analysis_time TIMESTAMP,
                mean_hr REAL,
                mean_rr REAL,
                sdnn REAL,
                rmssd REAL,
                pnn50 REAL,
                sd1 REAL,
                sd2 REAL,
                lf_power REAL,
                hf_power REAL,
                lf_hf_ratio REAL,
                total_power REAL,
                vlf_power REAL,
                lf_nu REAL,
                hf_nu REAL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            )
        ''')
        
        self.conn.commit()
    
    def create_session(self, notes=''):
        """新しいセッションを作成"""
        cursor = self.conn.cursor()
        cursor.execute('''
            INSERT INTO sessions (start_time, notes) 
            VALUES (?, ?)
        ''', (datetime.now(), notes))
        self.conn.commit()
        return cursor.lastrowid
    
    def close_session(self, session_id, sample_count):
        """セッションを終了"""
        cursor = self.conn.cursor()
        cursor.execute('''
            UPDATE sessions 
            SET end_time = ?, sample_count = ? 
            WHERE session_id = ?
        ''', (datetime.now(), sample_count, session_id))
        self.conn.commit()
    
    def save_raw_data(self, session_id, data_list):
        """生データをバッチ保存"""
        cursor = self.conn.cursor()
        cursor.executemany('''
            INSERT INTO raw_data (session_id, timestamp, red, ir, filt_red, filt_ir)
            VALUES (?, ?, ?, ?, ?, ?)
        ''', [(session_id, d['timestamp'], d['red'], d['ir'], 
               d['filt_red'], d['filt_ir']) for d in data_list])
        self.conn.commit()
    
    def save_rr_intervals(self, session_id, rr_data):
        """R-R間隔データを保存"""
        cursor = self.conn.cursor()
        cursor.executemany('''
            INSERT INTO rr_intervals (session_id, peak_index, peak_time, rr_interval)
            VALUES (?, ?, ?, ?)
        ''', [(session_id, d['peak_index'], d['peak_time'], d['rr_interval']) 
              for d in rr_data])
        self.conn.commit()
    
    def save_hrv_results(self, session_id, hrv_metrics):
        """HRV解析結果を保存"""
        cursor = self.conn.cursor()
        cursor.execute('''
            INSERT INTO hrv_results (
                session_id, analysis_time, mean_hr, mean_rr, sdnn, rmssd, pnn50,
                sd1, sd2, lf_power, hf_power, lf_hf_ratio, total_power,
                vlf_power, lf_nu, hf_nu
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (session_id, datetime.now(), 
              hrv_metrics['mean_hr'], hrv_metrics['mean_rr'],
              hrv_metrics['sdnn'], hrv_metrics['rmssd'], hrv_metrics['pnn50'],
              hrv_metrics['sd1'], hrv_metrics['sd2'],
              hrv_metrics['lf_power'], hrv_metrics['hf_power'], 
              hrv_metrics['lf_hf_ratio'], hrv_metrics['total_power'],
              hrv_metrics['vlf_power'], hrv_metrics['lf_nu'], hrv_metrics['hf_nu']))
        self.conn.commit()
    
    def get_session_data(self, session_id):
        """セッションの生データを取得"""
        cursor = self.conn.cursor()
        cursor.execute('''
            SELECT timestamp, red, ir, filt_red, filt_ir
            FROM raw_data
            WHERE session_id = ?
            ORDER BY timestamp
        ''', (session_id,))
        return cursor.fetchall()
    
    def get_rr_intervals(self, session_id):
        """セッションのR-R間隔を取得"""
        cursor = self.conn.cursor()
        cursor.execute('''
            SELECT peak_index, peak_time, rr_interval
            FROM rr_intervals
            WHERE session_id = ?
            ORDER BY peak_index
        ''', (session_id,))
        return cursor.fetchall()
    
    def get_all_sessions(self):
        """全セッション一覧を取得"""
        cursor = self.conn.cursor()
        cursor.execute('''
            SELECT session_id, start_time, end_time, sample_count, notes
            FROM sessions
            ORDER BY start_time DESC
        ''')
        return cursor.fetchall()
    
    def close(self):
        """データベース接続を閉じる"""
        self.conn.close()


class HRVAnalyzer:
    """詳細なHRV解析を行うクラス"""
    
    def __init__(self, sampling_rate=100):
        self.fs = sampling_rate  # サンプリング周波数
    
    def detect_peaks(self, signal_data, min_distance=50, threshold=None):
        """
        ピーク検出（R波に相当）
        
        Parameters:
        -----------
        signal_data : array-like
            フィルタ後の信号（filtIR推奨）
        min_distance : int
            ピーク間の最小距離（サンプル数）。デフォルト50 = 0.5秒
        threshold : float
            ピーク検出の閾値。Noneの場合は自動設定
        
        Returns:
        --------
        peaks : array
            ピーク位置のインデックス
        """
        if threshold is None:
            # 信号の標準偏差の30%を閾値とする
            threshold = np.std(signal_data) * 0.3
        
        # scipy.signal.find_peaksを使用
        peaks, properties = signal.find_peaks(
            signal_data,
            height=threshold,
            distance=min_distance,
            prominence=threshold * 0.5
        )
        
        return peaks
    
    def calculate_rr_intervals(self, peaks, timestamps=None):
        """
        R-R間隔を計算
        
        Parameters:
        -----------
        peaks : array
            ピーク位置のインデックス
        timestamps : array, optional
            各サンプルのタイムスタンプ（ms）
        
        Returns:
        --------
        rr_intervals : array
            R-R間隔（ms）
        rr_times : array
            各R-R間隔の中点時刻
        """
        if len(peaks) < 2:
            return np.array([]), np.array([])
        
        if timestamps is not None:
            # タイムスタンプベースで計算
            rr_intervals = np.diff(timestamps[peaks])
            rr_times = (timestamps[peaks[:-1]] + timestamps[peaks[1:]]) / 2
        else:
            # サンプルインデックスベースで計算（ms変換）
            rr_intervals = np.diff(peaks) * 1000.0 / self.fs
            rr_times = (peaks[:-1] + peaks[1:]) / 2 * 1000.0 / self.fs
        
        return rr_intervals, rr_times
    
    def clean_rr_intervals(self, rr_intervals, min_rr=300, max_rr=2000):
        """
        異常なR-R間隔を除去
        
        Parameters:
        -----------
        rr_intervals : array
            R-R間隔（ms）
        min_rr : float
            最小R-R間隔（ms）デフォルト300ms = 200bpm
        max_rr : float
            最大R-R間隔（ms）デフォルト2000ms = 30bpm
        
        Returns:
        --------
        cleaned_rr : array
            クリーニング後のR-R間隔
        """
        # 範囲外の値を除去
        mask = (rr_intervals >= min_rr) & (rr_intervals <= max_rr)
        
        # 外れ値除去（中央値の±30%以上離れた値）
        if np.sum(mask) > 0:
            median_rr = np.median(rr_intervals[mask])
            outlier_mask = np.abs(rr_intervals - median_rr) < median_rr * 0.3
            mask = mask & outlier_mask
        
        return rr_intervals[mask]
    
    def calculate_time_domain_hrv(self, rr_intervals):
        """
        時間領域のHRV指標を計算
        
        Parameters:
        -----------
        rr_intervals : array
            R-R間隔（ms）
        
        Returns:
        --------
        metrics : dict
            時間領域HRV指標
        """
        if len(rr_intervals) < 2:
            return {}
        
        # 基本統計
        mean_rr = np.mean(rr_intervals)
        mean_hr = 60000.0 / mean_rr if mean_rr > 0 else 0
        
        # SDNN: 全R-R間隔の標準偏差
        sdnn = np.std(rr_intervals, ddof=1)
        
        # RMSSD: 連続するR-R間隔の差の二乗平均平方根
        rr_diff = np.diff(rr_intervals)
        rmssd = np.sqrt(np.mean(rr_diff ** 2))
        
        # pNN50: 連続R-R間隔差が50ms以上の割合
        nn50 = np.sum(np.abs(rr_diff) > 50)
        pnn50 = 100.0 * nn50 / len(rr_diff) if len(rr_diff) > 0 else 0
        
        # ポアンカレプロット指標
        # SD1: 短期変動（副交感神経活動）
        sd1 = np.sqrt(0.5 * rmssd ** 2)
        # SD2: 長期変動（交感神経活動）
        sd2 = np.sqrt(2 * sdnn ** 2 - 0.5 * rmssd ** 2)
        
        return {
            'mean_rr': mean_rr,
            'mean_hr': mean_hr,
            'sdnn': sdnn,
            'rmssd': rmssd,
            'pnn50': pnn50,
            'sd1': sd1,
            'sd2': sd2
        }
    
    def calculate_frequency_domain_hrv(self, rr_intervals, rr_times, 
                                       method='welch'):
        """
        周波数領域のHRV指標を計算
        
        Parameters:
        -----------
        rr_intervals : array
            R-R間隔（ms）
        rr_times : array
            各R-R間隔の時刻（ms）
        method : str
            'welch' または 'fft'
        
        Returns:
        --------
        metrics : dict
            周波数領域HRV指標
        """
        if len(rr_intervals) < 10:
            return {}
        
        # R-R間隔を等間隔にリサンプリング（4Hz推奨）
        resampling_freq = 4.0  # Hz
        
        # 時間軸を秒単位に変換
        time_sec = rr_times / 1000.0
        time_min = np.min(time_sec)
        time_max = np.max(time_sec)
        
        # 等間隔時間軸を作成
        time_interp = np.arange(time_min, time_max, 1.0 / resampling_freq)
        
        # 線形補間
        f_interp = interpolate.interp1d(time_sec, rr_intervals, 
                                        kind='cubic', fill_value='extrapolate')
        rr_interp = f_interp(time_interp)
        
        # 平均を引く（デトレンド）
        rr_detrended = rr_interp - np.mean(rr_interp)
        
        if method == 'welch':
            # Welch法でパワースペクトル密度を計算
            freqs, psd = signal.welch(
                rr_detrended,
                fs=resampling_freq,
                nperseg=min(256, len(rr_detrended)),
                scaling='density'
            )
        else:
            # FFT法
            n = len(rr_detrended)
            fft_vals = fft(rr_detrended)
            psd = np.abs(fft_vals) ** 2 / n
            freqs = fftfreq(n, 1.0 / resampling_freq)
            
            # 正の周波数のみ
            mask = freqs >= 0
            freqs = freqs[mask]
            psd = psd[mask]
        
        # 周波数帯域のパワーを計算
        # VLF: 0.003-0.04 Hz（非常に長期変動）
        # LF: 0.04-0.15 Hz（低周波、交感神経+副交感神経）
        # HF: 0.15-0.4 Hz（高周波、副交感神経）
        
        vlf_mask = (freqs >= 0.003) & (freqs < 0.04)
        lf_mask = (freqs >= 0.04) & (freqs < 0.15)
        hf_mask = (freqs >= 0.15) & (freqs < 0.4)
        
        vlf_power = np.trapz(psd[vlf_mask], freqs[vlf_mask])
        lf_power = np.trapz(psd[lf_mask], freqs[lf_mask])
        hf_power = np.trapz(psd[hf_mask], freqs[hf_mask])
        total_power = np.trapz(psd, freqs)
        
        # 正規化単位（nu: normalized unit）
        lf_nu = 100.0 * lf_power / (lf_power + hf_power) if (lf_power + hf_power) > 0 else 0
        hf_nu = 100.0 * hf_power / (lf_power + hf_power) if (lf_power + hf_power) > 0 else 0
        
        # LF/HF比（交感神経バランス指標）
        lf_hf_ratio = lf_power / hf_power if hf_power > 0 else 0
        
        return {
            'vlf_power': vlf_power,
            'lf_power': lf_power,
            'hf_power': hf_power,
            'total_power': total_power,
            'lf_nu': lf_nu,
            'hf_nu': hf_nu,
            'lf_hf_ratio': lf_hf_ratio,
            'freqs': freqs,
            'psd': psd
        }
    
    def analyze(self, signal_data, timestamps=None):
        """
        完全なHRV解析を実行
        
        Parameters:
        -----------
        signal_data : array-like
            フィルタ後の信号
        timestamps : array, optional
            タイムスタンプ（ms）
        
        Returns:
        --------
        results : dict
            全HRV指標とR-R間隔データ
        """
        # ピーク検出
        peaks = self.detect_peaks(signal_data)
        
        if len(peaks) < 2:
            return {'error': 'Not enough peaks detected'}
        
        # R-R間隔計算
        rr_intervals, rr_times = self.calculate_rr_intervals(peaks, timestamps)
        
        # R-R間隔クリーニング
        rr_clean = self.clean_rr_intervals(rr_intervals)
        
        if len(rr_clean) < 2:
            return {'error': 'Not enough valid RR intervals'}
        
        # 時間領域解析
        time_metrics = self.calculate_time_domain_hrv(rr_clean)
        
        # 周波数領域解析
        freq_metrics = {}
        if len(rr_clean) >= 10:
            # クリーニング後のR-R間隔に対応する時刻を再取得
            mask = (rr_intervals >= 300) & (rr_intervals <= 2000)
            median_rr = np.median(rr_intervals[mask])
            outlier_mask = np.abs(rr_intervals - median_rr) < median_rr * 0.3
            final_mask = mask & outlier_mask
            rr_times_clean = rr_times[final_mask]
            
            freq_metrics = self.calculate_frequency_domain_hrv(
                rr_clean, rr_times_clean
            )
        
        # 結果を統合
        results = {**time_metrics, **freq_metrics}
        results['peaks'] = peaks
        results['rr_intervals'] = rr_intervals
        results['rr_times'] = rr_times
        results['rr_clean'] = rr_clean
        results['num_peaks'] = len(peaks)
        results['num_rr'] = len(rr_intervals)
        results['num_rr_clean'] = len(rr_clean)
        
        return results


class LiveMonitor:
    """リアルタイムモニタリングとグラフ表示"""
    
    def __init__(self, port='COM3', baudrate=115200, db_path='ppg_data.db', show_plot=False):
        self.port = port
        self.baudrate = baudrate
        self.db = PPGDatabase(db_path)
        self.analyzer = HRVAnalyzer()
        self.session_id = None
        self.data_buffer = []
        self.show_plot = show_plot
        
        # グラフ用データバッファ
        self.time_buffer = deque(maxlen=500)
        self.signal_buffer = deque(maxlen=500)
        self.hr_buffer = deque(maxlen=100)
        self.rmssd_buffer = deque(maxlen=100)
        
        # リアルタイムプロット用
        self.fig = None
        self.axes = None
        self.lines = None
        self.ser = None
        self.running = True
        
        # スレッド制御用
        self.data_lock = threading.Lock()
        self.in_data_mode = False
        
    def parse_csv_line(self, line):
        """CSVデータ行をパース（連続モード対応）"""
        try:
            # ヘッダー行や空行をスキップ
            if not line or line.startswith('#') or 'Index' in line or 'Buffer' in line or 'ESP-ROM' in line:
                return None
            
            parts = line.strip().split(',')
            
            # フォーマット1: Index,Timestamp,Red,IR,FiltRed,FiltIR (6要素)
            if len(parts) == 6 and parts[0].isdigit():
                try:
                    return {
                        'timestamp': int(parts[1]),
                        'red': int(parts[2]),
                        'ir': int(parts[3]),
                        'filt_red': float(parts[4]),
                        'filt_ir': float(parts[5])
                    }
                except:
                    pass
            
            # それ以外のデータは無視
            return None
        except:
            pass
        return None
    
    def run(self):
        """リアルタイムモニタリングを開始"""
        if self.show_plot:
            self._run_with_plot()
        else:
            self._run_without_plot()
    
    def _run_without_plot(self):
        """グラフなしのリアルタイムモニタリング"""
        print(f"Connecting to {self.port}...")
        
        try:
            ser = serial.Serial(self.port, self.baudrate, timeout=1)
            print("Connected! Waiting for data...")
            print("Press Ctrl+C to stop")
            
            in_data_mode = False
            
            while True:
                line = ser.readline().decode('utf-8', errors='ignore').strip()
                
                if not line:
                    continue
                
                # データモード開始検出
                if line.startswith('# Index'):
                    in_data_mode = True
                    self.session_id = self.db.create_session(notes='Live monitoring')
                    self.data_buffer = []
                    print(f"\n[Session {self.session_id}] Data recording started")
                    continue
                
                # データモード終了検出
                if 'Buffer cleared' in line:
                    if in_data_mode and self.data_buffer:
                        self._process_session()
                    in_data_mode = False
                    continue
                
                # データ行処理
                if in_data_mode:
                    data = self.parse_csv_line(line)
                    if data:
                        self.data_buffer.append(data)
                        
                        # リアルタイムグラフ更新用
                        self.time_buffer.append(data['timestamp'] / 1000.0)
                        self.signal_buffer.append(data['filt_ir'])
                
        except KeyboardInterrupt:
            print("\n\nStopping...")
        except Exception as e:
            print(f"Error: {e}")
        finally:
            if 'ser' in locals():
                ser.close()
            self.db.close()
    
    def _serial_reader_thread(self):
        """シリアルポート読み取り専用スレッド（連続モード）"""
        try:
            print(f"\n=== Continuous monitoring mode ===")
            print(f"Press the ATOMS3 button to start data streaming")
            print(f"Data will be displayed continuously without needing to press again")
            
            in_data_stream = False
            
            while self.running:
                if not self.ser or not self.ser.is_open:
                    time.sleep(0.1)
                    continue
                
                try:
                    line = self.ser.readline().decode('utf-8', errors='ignore').strip()
                    
                    if not line:
                        time.sleep(0.001)
                        continue
                    
                    # データストリーム開始検出
                    if '# Index' in line:
                        # 新しいストリーム開始時にバッファをクリア
                        with self.data_lock:
                            self.data_buffer.clear()
                            self.time_buffer.clear()
                            self.signal_buffer.clear()
                            self.hr_buffer.clear()
                            self.rmssd_buffer.clear()
                        in_data_stream = True
                        print(f"\n[Data streaming started - graphs will update continuously]")
                        continue
                    
                    # データストリーム継続中のみ処理
                    if in_data_stream:
                        # "Buffer cleared"でストリーム終了を検出するが、継続フラグは保持
                        if 'Buffer cleared' in line:
                            print(f"\n[One cycle complete - ready for next button press]")
                            in_data_stream = False
                            continue
                        
                        # CSVフォーマットのデータ行を処理
                        data = self.parse_csv_line(line)
                        if data:
                            with self.data_lock:
                                self.data_buffer.append(data)
                                self.time_buffer.append(data['timestamp'] / 1000.0)
                                self.signal_buffer.append(data['filt_ir'])
                                
                                # 100サンプルごとにHR/RMSSD計算
                                if len(self.data_buffer) % 100 == 0 and len(self.data_buffer) >= 300:
                                    recent_data = self.data_buffer[-300:]
                                    timestamps = np.array([d['timestamp'] for d in recent_data])
                                    signal_data = np.array([d['filt_ir'] for d in recent_data])
                                    
                                    try:
                                        std_signal = np.std(signal_data)
                                        if std_signal > 1.0:
                                            peaks = self.analyzer.detect_peaks(signal_data, min_distance=50, threshold=std_signal * 0.2)
                                            
                                            if len(peaks) >= 4:
                                                rr_intervals, _ = self.analyzer.calculate_rr_intervals(peaks, timestamps)
                                                valid_rr = rr_intervals[(rr_intervals >= 400) & (rr_intervals <= 1500)]
                                                
                                                if len(valid_rr) >= 3:
                                                    mean_rr = np.mean(valid_rr)
                                                    hr = 60000.0 / mean_rr if mean_rr > 0 else 0
                                                    rr_diff = np.diff(valid_rr)
                                                    rmssd = np.sqrt(np.mean(rr_diff ** 2)) if len(rr_diff) > 0 else 0
                                                    
                                                    if 40 <= hr <= 120:
                                                        self.hr_buffer.append(hr)
                                                        self.rmssd_buffer.append(rmssd)
                                                        print(f"HR: {hr:.1f} bpm, RMSSD: {rmssd:.1f} ms (samples: {len(self.data_buffer)})")
                                    except Exception as e:
                                        pass
                
                except Exception as e:
                    continue
        except Exception as e:
            print(f"Reader thread error: {e}")
            import traceback
            traceback.print_exc()
    
    def _run_with_plot(self):
        """グラフ付きリアルタイムモニタリング（マルチスレッド版）"""
        print(f"Connecting to {self.port}...")
        
        try:
            self.ser = serial.Serial(self.port, self.baudrate, timeout=0.1)
            print("Connected! Opening plot window...")
            print("Close plot window to stop")
            
            # matplotlibバックエンド設定
            import matplotlib
            matplotlib.use('TkAgg')  # より安定したバックエンド
            
            # プロット初期化
            plt.rcParams['path.simplify'] = True
            plt.rcParams['path.simplify_threshold'] = 1.0
            plt.rcParams['agg.path.chunksize'] = 10000
            
            self.fig = plt.figure(figsize=(14, 9))
            self.fig.suptitle('PPGclip Real-time Monitor', fontsize=14, fontweight='bold')
            
            # グリッド配置
            gs = self.fig.add_gridspec(3, 1, hspace=0.3)
            
            # サブプロット1: フィルタ後信号（固定範囲）
            ax1 = self.fig.add_subplot(gs[0, 0])
            ax1.set_title('Filtered IR Signal', fontsize=11)
            ax1.set_ylabel('Amplitude', fontsize=10)
            ax1.set_ylim(-150, 150)  # 固定範囲で自動スケール無効化
            ax1.grid(True, alpha=0.3, linewidth=0.5)
            line1, = ax1.plot([], [], 'b-', linewidth=0.7)
            
            # サブプロット2: 心拍数
            ax2 = self.fig.add_subplot(gs[1, 0])
            ax2.set_title('Heart Rate', fontsize=11)
            ax2.set_ylabel('HR (bpm)', fontsize=10)
            ax2.set_ylim(40, 120)
            ax2.set_xlim(0, 100)
            ax2.axhline(y=60, color='gray', linestyle='--', alpha=0.3, linewidth=0.5)
            ax2.axhline(y=80, color='gray', linestyle='--', alpha=0.3, linewidth=0.5)
            ax2.grid(True, alpha=0.3, linewidth=0.5)
            line2, = ax2.plot([], [], 'r-', linewidth=1.2, marker='o', markersize=2)
            
            # サブプロット3: RMSSD
            ax3 = self.fig.add_subplot(gs[2, 0])
            ax3.set_title('RMSSD (Heart Rate Variability)', fontsize=11)
            ax3.set_xlabel('Measurement Count', fontsize=10)
            ax3.set_ylabel('RMSSD (ms)', fontsize=10)
            ax3.set_ylim(0, 100)
            ax3.set_xlim(0, 100)
            ax3.axhline(y=20, color='gray', linestyle='--', alpha=0.3, linewidth=0.5)
            ax3.axhline(y=50, color='gray', linestyle='--', alpha=0.3, linewidth=0.5)
            ax3.grid(True, alpha=0.3, linewidth=0.5)
            line3, = ax3.plot([], [], 'g-', linewidth=1.2, marker='s', markersize=2)
            
            self.axes = [ax1, ax2, ax3]
            self.lines = [line1, line2, line3]
            
            # タイトルテキスト（データカウント表示用）
            status_text = self.fig.text(0.02, 0.98, '', fontsize=9, 
                                       verticalalignment='top', family='monospace')
            
            # データ読み取りスレッド開始
            reader_thread = threading.Thread(target=self._serial_reader_thread, daemon=True)
            reader_thread.start()
            
            # アニメーション更新関数（最適化版）
            last_update = [0]  # ミュータブルなリストで共有
            
            def animate(frame):
                current_time = time.time()
                # 最低でも100ms間隔を保つ
                if current_time - last_update[0] < 0.1:
                    return self.lines
                last_update[0] = current_time
                
                try:
                    with self.data_lock:
                        # 信号データ更新（最新500点のみ）
                        if len(self.time_buffer) > 10:
                            times = np.array(list(self.time_buffer))
                            signals = np.array(list(self.signal_buffer))
                            
                            self.lines[0].set_data(times, signals)
                            
                            # X軸範囲のみ動的調整（Y軸は固定）
                            if len(times) > 0:
                                time_range = times[-1] - times[0]
                                if time_range > 0:
                                    ax1.set_xlim(times[0], times[-1])
                        
                        # HR/RMSSD更新
                        if len(self.hr_buffer) > 0:
                            hr_count = len(self.hr_buffer)
                            hr_times = np.arange(hr_count)
                            hrs = np.array(list(self.hr_buffer))
                            rmssds = np.array(list(self.rmssd_buffer))
                            
                            self.lines[1].set_data(hr_times, hrs)
                            self.lines[2].set_data(hr_times, rmssds)
                            
                            # X軸を動的調整
                            if hr_count > 1:
                                ax2.set_xlim(0, max(100, hr_count))
                                ax3.set_xlim(0, max(100, hr_count))
                            
                            # ステータス表示
                            if len(self.data_buffer) > 0:
                                latest_hr = hrs[-1] if len(hrs) > 0 else 0
                                latest_rmssd = rmssds[-1] if len(rmssds) > 0 else 0
                                status_text.set_text(
                                    f'Samples: {len(self.data_buffer):5d} | '
                                    f'HR: {latest_hr:5.1f} bpm | '
                                    f'RMSSD: {latest_rmssd:5.1f} ms'
                                )
                    
                    return self.lines + [status_text]
                except Exception as e:
                    return self.lines
            
            # FuncAnimationで更新（blit無効化でより安定）
            anim = FuncAnimation(
                self.fig, animate, 
                interval=300,  # 300ms間隔（より安定）
                blit=False,  # blitを無効化して安定性向上
                cache_frame_data=False,
                repeat=True
            )
            
            # ウィンドウを最大化して表示
            mng = plt.get_current_fig_manager()
            try:
                mng.window.state('zoomed')  # Windows
            except:
                try:
                    mng.resize(*mng.window.maxsize())
                except:
                    pass
            
            plt.show(block=True)
            
            print("\n\nStopping...")
            
        except KeyboardInterrupt:
            print("\n\nStopping...")
        except Exception as e:
            print(f"Error: {e}")
            import traceback
            traceback.print_exc()
        finally:
            self.running = False
            time.sleep(0.5)  # スレッド終了待ち
            if self.ser and self.ser.is_open:
                self.ser.close()
            self.db.close()
    
    def _process_session(self):
        """セッションデータを処理・保存"""
        print(f"Processing {len(self.data_buffer)} samples...")
        
        # データベースに保存
        self.db.save_raw_data(self.session_id, self.data_buffer)
        
        # HRV解析
        timestamps = np.array([d['timestamp'] for d in self.data_buffer])
        signal_data = np.array([d['filt_ir'] for d in self.data_buffer])
        
        hrv_results = self.analyzer.analyze(signal_data, timestamps)
        
        if 'error' not in hrv_results:
            # R-R間隔データを保存
            rr_data = []
            peaks = hrv_results['peaks']
            rr_intervals = hrv_results['rr_intervals']
            rr_times = hrv_results['rr_times']
            
            for i, (peak, rr_time, rr_int) in enumerate(zip(peaks[1:], rr_times, rr_intervals)):
                rr_data.append({
                    'peak_index': int(peak),
                    'peak_time': float(rr_time),
                    'rr_interval': float(rr_int)
                })
            
            self.db.save_rr_intervals(self.session_id, rr_data)
            
            # HRV指標を保存
            self.db.save_hrv_results(self.session_id, hrv_results)
            
            # 結果表示
            print(f"\n{'='*60}")
            print(f"HRV Analysis Results - Session {self.session_id}")
            print(f"{'='*60}")
            print(f"Heart Rate:      {hrv_results['mean_hr']:.1f} bpm")
            print(f"Mean R-R:        {hrv_results['mean_rr']:.1f} ms")
            print(f"SDNN:            {hrv_results['sdnn']:.2f} ms")
            print(f"RMSSD:           {hrv_results['rmssd']:.2f} ms")
            print(f"pNN50:           {hrv_results['pnn50']:.1f} %")
            print(f"SD1:             {hrv_results['sd1']:.2f} ms")
            print(f"SD2:             {hrv_results['sd2']:.2f} ms")
            
            if 'lf_power' in hrv_results:
                print(f"\nFrequency Domain:")
                print(f"LF Power:        {hrv_results['lf_power']:.2f} ms²")
                print(f"HF Power:        {hrv_results['hf_power']:.2f} ms²")
                print(f"LF/HF Ratio:     {hrv_results['lf_hf_ratio']:.2f}")
                print(f"LF (nu):         {hrv_results['lf_nu']:.1f} %")
                print(f"HF (nu):         {hrv_results['hf_nu']:.1f} %")
            
            print(f"{'='*60}\n")
        else:
            print(f"Error: {hrv_results['error']}")
        
        self.db.close_session(self.session_id, len(self.data_buffer))


class OfflineAnalyzer:
    """過去データの再解析とグラフ化"""
    
    def __init__(self, db_path='ppg_data.db'):
        self.db = PPGDatabase(db_path)
        self.analyzer = HRVAnalyzer()
    
    def list_sessions(self):
        """全セッションをリスト表示"""
        sessions = self.db.get_all_sessions()
        print(f"\n{'='*80}")
        print(f"{'ID':<6} {'Start Time':<20} {'End Time':<20} {'Samples':<10} {'Notes'}")
        print(f"{'='*80}")
        for session in sessions:
            sid, start, end, count, notes = session
            print(f"{sid:<6} {start:<20} {end or 'Running':<20} {count or 0:<10} {notes or ''}")
        print(f"{'='*80}\n")
    
    def visualize_session(self, session_id):
        """セッションデータを可視化"""
        # データ取得
        raw_data = self.db.get_session_data(session_id)
        rr_data = self.db.get_rr_intervals(session_id)
        
        if not raw_data:
            print(f"No data found for session {session_id}")
            return
        
        timestamps, red, ir, filt_red, filt_ir = zip(*raw_data)
        timestamps = np.array(timestamps) / 1000.0  # 秒に変換
        filt_ir = np.array(filt_ir)
        
        # HRV再解析
        hrv_results = self.analyzer.analyze(filt_ir, np.array(timestamps) * 1000)
        
        # グラフ作成
        fig = plt.figure(figsize=(16, 10))
        fig.suptitle(f'PPGclip Analysis - Session {session_id}', fontsize=16)
        
        # 1. 生信号
        ax1 = plt.subplot(3, 3, 1)
        ax1.plot(timestamps, filt_ir, 'b-', linewidth=0.5)
        if 'peaks' in hrv_results:
            peaks = hrv_results['peaks']
            ax1.plot(timestamps[peaks], filt_ir[peaks], 'r.', markersize=8)
        ax1.set_xlabel('Time (s)')
        ax1.set_ylabel('Filtered IR Signal')
        ax1.set_title('PPG Signal with R Peaks')
        ax1.grid(True, alpha=0.3)
        
        # 2. R-R間隔タコグラム
        ax2 = plt.subplot(3, 3, 2)
        if 'rr_intervals' in hrv_results and len(hrv_results['rr_intervals']) > 0:
            rr_times = hrv_results['rr_times'] / 1000.0
            rr_intervals = hrv_results['rr_intervals']
            ax2.plot(rr_times, rr_intervals, 'g-o', markersize=4)
            ax2.set_xlabel('Time (s)')
            ax2.set_ylabel('R-R Interval (ms)')
            ax2.set_title('Tachogram')
            ax2.grid(True, alpha=0.3)
        
        # 3. R-R間隔ヒストグラム
        ax3 = plt.subplot(3, 3, 3)
        if 'rr_clean' in hrv_results and len(hrv_results['rr_clean']) > 0:
            ax3.hist(hrv_results['rr_clean'], bins=30, color='skyblue', edgecolor='black')
            ax3.axvline(np.mean(hrv_results['rr_clean']), color='r', linestyle='--', 
                       label=f'Mean: {np.mean(hrv_results["rr_clean"]):.1f} ms')
            ax3.set_xlabel('R-R Interval (ms)')
            ax3.set_ylabel('Count')
            ax3.set_title('R-R Interval Distribution')
            ax3.legend()
            ax3.grid(True, alpha=0.3)
        
        # 4. ポアンカレプロット
        ax4 = plt.subplot(3, 3, 4)
        if 'rr_clean' in hrv_results and len(hrv_results['rr_clean']) > 1:
            rr = hrv_results['rr_clean']
            ax4.scatter(rr[:-1], rr[1:], alpha=0.5, s=20)
            ax4.plot([rr.min(), rr.max()], [rr.min(), rr.max()], 'r--')
            ax4.set_xlabel('RR(n) [ms]')
            ax4.set_ylabel('RR(n+1) [ms]')
            ax4.set_title(f'Poincaré Plot (SD1={hrv_results.get("sd1", 0):.1f}, SD2={hrv_results.get("sd2", 0):.1f})')
            ax4.grid(True, alpha=0.3)
            ax4.axis('equal')
        
        # 5. パワースペクトル密度
        ax5 = plt.subplot(3, 3, 5)
        if 'freqs' in hrv_results and 'psd' in hrv_results:
            freqs = hrv_results['freqs']
            psd = hrv_results['psd']
            ax5.plot(freqs, psd, 'b-')
            ax5.axvspan(0.04, 0.15, alpha=0.3, color='yellow', label='LF')
            ax5.axvspan(0.15, 0.4, alpha=0.3, color='cyan', label='HF')
            ax5.set_xlabel('Frequency (Hz)')
            ax5.set_ylabel('PSD (ms²/Hz)')
            ax5.set_title('Power Spectral Density')
            ax5.set_xlim(0, 0.5)
            ax5.legend()
            ax5.grid(True, alpha=0.3)
        
        # 6. HRV指標サマリー（テキスト）
        ax6 = plt.subplot(3, 3, 6)
        ax6.axis('off')
        if 'mean_hr' in hrv_results:
            metrics_text = f"""
TIME DOMAIN METRICS:
━━━━━━━━━━━━━━━━━━━━━━━
Heart Rate:     {hrv_results.get('mean_hr', 0):.1f} bpm
Mean R-R:       {hrv_results.get('mean_rr', 0):.1f} ms
SDNN:           {hrv_results.get('sdnn', 0):.2f} ms
RMSSD:          {hrv_results.get('rmssd', 0):.2f} ms
pNN50:          {hrv_results.get('pnn50', 0):.1f} %
SD1:            {hrv_results.get('sd1', 0):.2f} ms
SD2:            {hrv_results.get('sd2', 0):.2f} ms

FREQUENCY DOMAIN METRICS:
━━━━━━━━━━━━━━━━━━━━━━━
LF Power:       {hrv_results.get('lf_power', 0):.2f} ms²
HF Power:       {hrv_results.get('hf_power', 0):.2f} ms²
LF/HF Ratio:    {hrv_results.get('lf_hf_ratio', 0):.2f}
LF (nu):        {hrv_results.get('lf_nu', 0):.1f} %
HF (nu):        {hrv_results.get('hf_nu', 0):.1f} %
            """
            ax6.text(0.1, 0.9, metrics_text, fontsize=10, verticalalignment='top',
                    family='monospace', transform=ax6.transAxes)
        
        # 7. 心拍数推移
        ax7 = plt.subplot(3, 3, 7)
        if 'rr_intervals' in hrv_results and len(hrv_results['rr_intervals']) > 0:
            hr = 60000.0 / np.array(hrv_results['rr_intervals'])
            ax7.plot(hrv_results['rr_times'] / 1000.0, hr, 'm-', linewidth=1.5)
            ax7.set_xlabel('Time (s)')
            ax7.set_ylabel('Heart Rate (bpm)')
            ax7.set_title('Instantaneous Heart Rate')
            ax7.grid(True, alpha=0.3)
        
        # 8. LF/HF比推移（移動窓解析）
        ax8 = plt.subplot(3, 3, 8)
        if 'rr_clean' in hrv_results and len(hrv_results['rr_clean']) > 20:
            window_size = 20
            lf_hf_ratios = []
            window_times = []
            
            for i in range(window_size, len(hrv_results['rr_clean'])):
                window_rr = hrv_results['rr_clean'][i-window_size:i]
                window_times_data = hrv_results['rr_times'][i-window_size:i]
                
                freq_metrics = self.analyzer.calculate_frequency_domain_hrv(
                    window_rr, window_times_data
                )
                
                if 'lf_hf_ratio' in freq_metrics:
                    lf_hf_ratios.append(freq_metrics['lf_hf_ratio'])
                    window_times.append(window_times_data[-1] / 1000.0)
            
            if lf_hf_ratios:
                ax8.plot(window_times, lf_hf_ratios, 'r-', linewidth=1.5)
                ax8.set_xlabel('Time (s)')
                ax8.set_ylabel('LF/HF Ratio')
                ax8.set_title('LF/HF Ratio Trend (20-beat window)')
                ax8.grid(True, alpha=0.3)
        
        # 9. RMSSD推移（移動窓解析）
        ax9 = plt.subplot(3, 3, 9)
        if 'rr_clean' in hrv_results and len(hrv_results['rr_clean']) > 20:
            window_size = 20
            rmssd_values = []
            window_times = []
            
            for i in range(window_size, len(hrv_results['rr_clean'])):
                window_rr = hrv_results['rr_clean'][i-window_size:i]
                rr_diff = np.diff(window_rr)
                rmssd = np.sqrt(np.mean(rr_diff ** 2))
                rmssd_values.append(rmssd)
                window_times.append(hrv_results['rr_times'][i] / 1000.0)
            
            if rmssd_values:
                ax9.plot(window_times, rmssd_values, 'g-', linewidth=1.5)
                ax9.set_xlabel('Time (s)')
                ax9.set_ylabel('RMSSD (ms)')
                ax9.set_title('RMSSD Trend (20-beat window)')
                ax9.grid(True, alpha=0.3)
        
        plt.tight_layout()
        plt.show()
    
    def export_session(self, session_id, output_file):
        """セッションデータをCSVエクスポート"""
        raw_data = self.db.get_session_data(session_id)
        rr_data = self.db.get_rr_intervals(session_id)
        
        with open(output_file, 'w') as f:
            f.write('# PPGclip Data Export\n')
            f.write(f'# Session ID: {session_id}\n')
            f.write('#\n')
            f.write('# Raw Data\n')
            f.write('timestamp,red,ir,filt_red,filt_ir\n')
            for row in raw_data:
                f.write(','.join(map(str, row)) + '\n')
            
            f.write('#\n')
            f.write('# R-R Intervals\n')
            f.write('peak_index,peak_time,rr_interval\n')
            for row in rr_data:
                f.write(','.join(map(str, row)) + '\n')
        
        print(f"Exported to {output_file}")


def main():
    parser = argparse.ArgumentParser(description='PPGclip HRV Analyzer')
    parser.add_argument('--mode', choices=['live', 'analyze', 'list', 'export'],
                       default='live', help='Operating mode')
    parser.add_argument('--port', default='COM3', help='Serial port')
    parser.add_argument('--baudrate', type=int, default=115200, help='Baud rate')
    parser.add_argument('--db', default='ppg_data.db', help='Database file path')
    parser.add_argument('--session', type=int, help='Session ID for analysis')
    parser.add_argument('--output', help='Output file for export')
    parser.add_argument('--plot', action='store_true', help='Show real-time plot (live mode only)')
    
    args = parser.parse_args()
    
    if args.mode == 'live':
        monitor = LiveMonitor(args.port, args.baudrate, args.db, show_plot=args.plot)
        monitor.run()
    
    elif args.mode == 'list':
        analyzer = OfflineAnalyzer(args.db)
        analyzer.list_sessions()
    
    elif args.mode == 'analyze':
        if args.session is None:
            print("Error: --session ID required for analyze mode")
            return
        analyzer = OfflineAnalyzer(args.db)
        analyzer.visualize_session(args.session)
    
    elif args.mode == 'export':
        if args.session is None or args.output is None:
            print("Error: --session ID and --output file required for export mode")
            return
        analyzer = OfflineAnalyzer(args.db)
        analyzer.export_session(args.session, args.output)


if __name__ == '__main__':
    main()
