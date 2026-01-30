# PPGclip Simple HRV Analyzer (PowerShell版)
# Pythonなしで動作する簡易HRV解析ツール

param(
    [string]$Port = "COM3",
    [int]$Baudrate = 115200,
    [string]$OutputFile = "hrv_results.csv"
)

# シリアルポート設定
$serial = New-Object System.IO.Ports.SerialPort $Port, $Baudrate, None, 8, one

Write-Host "=================================" -ForegroundColor Cyan
Write-Host "  PPGclip Simple HRV Analyzer   " -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Port: $Port @ $Baudrate bps" -ForegroundColor Green
Write-Host "Output: $OutputFile" -ForegroundColor Green
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor Yellow
Write-Host "Press the button on ATOMS3 to export data" -ForegroundColor Yellow
Write-Host ""

# データ保存用配列
$timestamps = @()
$redData = @()
$irData = @()
$filtRedData = @()
$filtIRData = @()
$inDataMode = $false
$sessionCount = 0

try {
    $serial.Open()
    Write-Host "Connected! Waiting for data..." -ForegroundColor Green
    Write-Host ""
    
    while ($true) {
        if ($serial.BytesToRead -gt 0) {
            $line = $serial.ReadLine().Trim()
            
            # データモード開始検出
            if ($line -match "^# Index") {
                $inDataMode = $true
                $timestamps = @()
                $redData = @()
                $irData = @()
                $filtRedData = @()
                $filtIRData = @()
                $sessionCount++
                Write-Host "[Session $sessionCount] Recording started..." -ForegroundColor Cyan
                continue
            }
            
            # データモード終了検出
            if ($line -match "Buffer cleared") {
                if ($inDataMode -and $timestamps.Count -gt 0) {
                    Write-Host "[Session $sessionCount] Processing $($timestamps.Count) samples..." -ForegroundColor Yellow
                    
                    # HRV解析実行
                    $hrvResults = Analyze-HRV -FiltIR $filtIRData -Timestamps $timestamps
                    
                    # 結果表示
                    Write-Host ""
                    Write-Host "========================================" -ForegroundColor Green
                    Write-Host "  HRV Analysis Results" -ForegroundColor Green
                    Write-Host "========================================" -ForegroundColor Green
                    Write-Host "Samples:         $($timestamps.Count)" -ForegroundColor White
                    Write-Host "Heart Rate:      $($hrvResults.MeanHR.ToString('F1')) bpm" -ForegroundColor White
                    Write-Host "Mean R-R:        $($hrvResults.MeanRR.ToString('F1')) ms" -ForegroundColor White
                    Write-Host "SDNN:            $($hrvResults.SDNN.ToString('F2')) ms" -ForegroundColor White
                    Write-Host "RMSSD:           $($hrvResults.RMSSD.ToString('F2')) ms" -ForegroundColor White
                    Write-Host "pNN50:           $($hrvResults.pNN50.ToString('F1')) %" -ForegroundColor White
                    Write-Host "Min R-R:         $($hrvResults.MinRR.ToString('F1')) ms" -ForegroundColor White
                    Write-Host "Max R-R:         $($hrvResults.MaxRR.ToString('F1')) ms" -ForegroundColor White
                    Write-Host "Peaks detected:  $($hrvResults.PeakCount)" -ForegroundColor White
                    Write-Host "========================================" -ForegroundColor Green
                    Write-Host ""
                    
                    # CSVに保存
                    Save-Results -Results $hrvResults -Session $sessionCount -OutputFile $OutputFile
                }
                $inDataMode = $false
                continue
            }
            
            # データ行処理
            if ($inDataMode) {
                $parts = $line -split ','
                if ($parts.Count -eq 6) {
                    try {
                        $timestamps += [int]$parts[1]
                        $redData += [int]$parts[2]
                        $irData += [int]$parts[3]
                        $filtRedData += [double]$parts[4]
                        $filtIRData += [double]$parts[5]
                    } catch {
                        # パースエラーは無視
                    }
                }
            }
            
            # 通常の行は表示
            if (-not $inDataMode) {
                Write-Host $line
            }
        }
        Start-Sleep -Milliseconds 10
    }
}
finally {
    if ($serial.IsOpen) {
        $serial.Close()
    }
    Write-Host "`nPort closed." -ForegroundColor Yellow
}

# HRV解析関数
function Analyze-HRV {
    param(
        [double[]]$FiltIR,
        [int[]]$Timestamps
    )
    
    # ピーク検出（簡易版）
    $peaks = @()
    $threshold = ($FiltIR | Measure-Object -Average).Average
    
    for ($i = 2; $i -lt $FiltIR.Count - 2; $i++) {
        # ローカルマキシマム検出
        if ($FiltIR[$i] -gt $FiltIR[$i-1] -and 
            $FiltIR[$i] -gt $FiltIR[$i-2] -and
            $FiltIR[$i] -gt $FiltIR[$i+1] -and
            $FiltIR[$i] -gt $FiltIR[$i+2] -and
            $FiltIR[$i] -gt $threshold) {
            
            # 最小距離チェック（50サンプル = 0.5秒）
            if ($peaks.Count -eq 0 -or ($i - $peaks[-1]) -gt 50) {
                $peaks += $i
            }
        }
    }
    
    if ($peaks.Count -lt 2) {
        return @{
            Error = "Not enough peaks detected"
            PeakCount = $peaks.Count
            MeanHR = 0
            MeanRR = 0
            SDNN = 0
            RMSSD = 0
            pNN50 = 0
            MinRR = 0
            MaxRR = 0
        }
    }
    
    # R-R間隔計算
    $rrIntervals = @()
    for ($i = 0; $i -lt $peaks.Count - 1; $i++) {
        $rr = $Timestamps[$peaks[$i+1]] - $Timestamps[$peaks[$i]]
        
        # 有効範囲チェック（300-2000ms）
        if ($rr -ge 300 -and $rr -le 2000) {
            $rrIntervals += $rr
        }
    }
    
    if ($rrIntervals.Count -lt 2) {
        return @{
            Error = "Not enough valid RR intervals"
            PeakCount = $peaks.Count
            MeanHR = 0
            MeanRR = 0
            SDNN = 0
            RMSSD = 0
            pNN50 = 0
            MinRR = 0
            MaxRR = 0
        }
    }
    
    # 統計計算
    $meanRR = ($rrIntervals | Measure-Object -Average).Average
    $meanHR = 60000.0 / $meanRR
    $minRR = ($rrIntervals | Measure-Object -Minimum).Minimum
    $maxRR = ($rrIntervals | Measure-Object -Maximum).Maximum
    
    # SDNN（標準偏差）
    $variance = 0
    foreach ($rr in $rrIntervals) {
        $variance += [Math]::Pow($rr - $meanRR, 2)
    }
    $sdnn = [Math]::Sqrt($variance / ($rrIntervals.Count - 1))
    
    # RMSSD（連続差の二乗平均平方根）
    $sumSquaredDiff = 0
    $nn50Count = 0
    for ($i = 0; $i -lt $rrIntervals.Count - 1; $i++) {
        $diff = $rrIntervals[$i+1] - $rrIntervals[$i]
        $sumSquaredDiff += $diff * $diff
        
        # pNN50計算
        if ([Math]::Abs($diff) -gt 50) {
            $nn50Count++
        }
    }
    $rmssd = [Math]::Sqrt($sumSquaredDiff / ($rrIntervals.Count - 1))
    $pnn50 = 100.0 * $nn50Count / ($rrIntervals.Count - 1)
    
    return @{
        PeakCount = $peaks.Count
        MeanHR = $meanHR
        MeanRR = $meanRR
        SDNN = $sdnn
        RMSSD = $rmssd
        pNN50 = $pnn50
        MinRR = $minRR
        MaxRR = $maxRR
        RRIntervals = $rrIntervals
    }
}

# 結果保存関数
function Save-Results {
    param(
        [hashtable]$Results,
        [int]$Session,
        [string]$OutputFile
    )
    
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    
    # ヘッダー確認
    if (-not (Test-Path $OutputFile)) {
        "Timestamp,Session,Samples,MeanHR,MeanRR,SDNN,RMSSD,pNN50,MinRR,MaxRR,PeakCount" | Out-File $OutputFile -Encoding UTF8
    }
    
    # データ追加
    $line = "$timestamp,$Session,$($Results.RRIntervals.Count),$($Results.MeanHR.ToString('F2')),$($Results.MeanRR.ToString('F2')),$($Results.SDNN.ToString('F2')),$($Results.RMSSD.ToString('F2')),$($Results.pNN50.ToString('F2')),$($Results.MinRR.ToString('F2')),$($Results.MaxRR.ToString('F2')),$($Results.PeakCount)"
    $line | Out-File $OutputFile -Append -Encoding UTF8
    
    Write-Host "Results saved to $OutputFile" -ForegroundColor Green
}
