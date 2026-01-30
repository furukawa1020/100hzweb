# PPGclip HRV Analyzer - 使用方法

## 📋 概要
PPGclipから取得したデータをデータベースに保存し、詳細なHRV（心拍変動）解析を行うツールです。

## 🚀 セットアップ

### 1. Pythonライブラリのインストール
```powershell
pip install -r requirements.txt
```

### 2. 動作確認
```powershell
python hrv_analyzer.py --help
```

## 💡 使用方法

### モード1: リアルタイムモニタリング
シリアルポートからデータを受信し、自動的にデータベースに保存・HRV解析を実行します。

```powershell
# デフォルト（COM3、115200bps）
python hrv_analyzer.py --mode live

# ポート指定
python hrv_analyzer.py --mode live --port COM5

# データベースファイル指定
python hrv_analyzer.py --mode live --db my_ppg_data.db
```

**使い方:**
1. ATOMS3を接続
2. コマンド実行
3. ATOMS3のボタンを押してデータ転送
4. 自動的にHRV解析結果が表示されます

### モード2: セッション一覧表示
保存されているセッション一覧を表示します。

```powershell
python hrv_analyzer.py --mode list
```

### モード3: 過去データの再解析・グラフ化
保存されたセッションデータを詳細グラフで可視化します。

```powershell
# セッションID 1 のデータを解析
python hrv_analyzer.py --mode analyze --session 1

# 別のデータベースを指定
python hrv_analyzer.py --mode analyze --session 3 --db my_ppg_data.db
```

**表示されるグラフ:**
1. PPG信号とRピーク検出
2. R-R間隔タコグラム
3. R-R間隔ヒストグラム
4. ポアンカレプロット（SD1/SD2）
5. パワースペクトル密度（LF/HF）
6. HRV指標サマリー
7. 瞬時心拍数推移
8. LF/HF比の時間推移
9. RMSSDの時間推移

### モード4: データエクスポート
セッションデータをCSVファイルに出力します。

```powershell
python hrv_analyzer.py --mode export --session 1 --output session1_export.csv
```

## 📊 HRV指標の詳細

### 時間領域指標
- **Mean HR**: 平均心拍数（bpm）
- **Mean R-R**: 平均R-R間隔（ms）
- **SDNN**: 全R-R間隔の標準偏差。全体的な自律神経活動
- **RMSSD**: 連続R-R間隔差の二乗平均平方根。副交感神経活動の指標
- **pNN50**: R-R間隔差が50ms以上の割合（%）
- **SD1**: ポアンカレプロット短軸。短期変動（副交感神経）
- **SD2**: ポアンカレプロット長軸。長期変動（交感神経）

### 周波数領域指標
- **VLF Power**: 超低周波パワー（0.003-0.04 Hz）
- **LF Power**: 低周波パワー（0.04-0.15 Hz）。交感神経+副交感神経
- **HF Power**: 高周波パワー（0.15-0.4 Hz）。副交感神経活動
- **Total Power**: 全周波数帯域のパワー
- **LF/HF Ratio**: 交感神経バランスの指標（高いほど交感神経優位）
- **LF (nu)**: 正規化LFパワー（%）
- **HF (nu)**: 正規化HFパワー（%）

## 🗄️ データベース構造

### テーブル
1. **sessions**: セッション情報
2. **raw_data**: 生データ（Red/IR信号、フィルタ後データ）
3. **rr_intervals**: R-R間隔データ
4. **hrv_results**: HRV解析結果

### データベースファイル
デフォルト: `ppg_data.db`（SQLite形式）

## 🎯 使用例

### 例1: 日常測定ワークフロー
```powershell
# 1. リアルタイムモニタリング開始
python hrv_analyzer.py --mode live

# 2. ATOMS3のボタンを押して60秒測定
# → 自動的にHRV解析結果が表示される

# 3. セッション一覧確認
python hrv_analyzer.py --mode list

# 4. グラフで詳細確認
python hrv_analyzer.py --mode analyze --session 1
```

### 例2: 複数日のデータ比較
```powershell
# 各日の測定
python hrv_analyzer.py --mode live --db week1.db  # 月曜
python hrv_analyzer.py --mode live --db week1.db  # 火曜

# 後日、グラフで比較
python hrv_analyzer.py --mode analyze --session 1 --db week1.db
python hrv_analyzer.py --mode analyze --session 2 --db week1.db
```

## ⚠️ 注意事項

### 測定時の注意
- センサーを指にしっかり装着
- 測定中は動かない
- 最低60秒の安定した測定が推奨

### HRV解析の精度
- 最低10拍以上のR-R間隔が必要
- ノイズや不安定な測定は自動除外
- 周波数解析は長い測定ほど精度向上

## 🔧 トラブルシューティング

### エラー: "Not enough peaks detected"
→ センサーが正しく装着されていない可能性。測定をやり直してください。

### グラフが表示されない
→ matplotlibのバックエンド設定を確認。Windows環境では通常問題なし。

### データベースが見つからない
→ `--db` オプションでパスを明示的に指定してください。

## 📁 ファイル構成
```
sampleFW/
├── hrv_analyzer.py       # メインプログラム（新規追加）
├── requirements.txt      # Pythonライブラリ（新規追加）
├── HRV_ANALYZER_README.md # このファイル（新規追加）
├── ppg_data.db           # データベース（自動生成）
├── main.cpp              # 既存ファームウェア（変更なし）
└── platformio.ini        # 既存設定（変更なし）
```

## 🔄 元に戻す方法
このツールは完全に独立しています。削除する場合：
```powershell
# 新規追加ファイルを削除
Remove-Item hrv_analyzer.py
Remove-Item requirements.txt
Remove-Item HRV_ANALYZER_README.md
Remove-Item ppg_data.db  # データベースファイル
```

既存のファームウェアやPlatformIO設定には一切変更を加えていないため、これらのファイルを削除するだけで元の状態に戻ります。

## 📚 参考文献
- Task Force of the European Society of Cardiology (1996). Heart rate variability: standards of measurement, physiological interpretation and clinical use.
- Shaffer, F., & Ginsberg, J. P. (2017). An overview of heart rate variability metrics and norms. Frontiers in public health, 5, 258.
