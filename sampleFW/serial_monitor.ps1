# シリアルモニタースクリプト
$port = new-Object System.IO.Ports.SerialPort COM3,115200,None,8,one
$port.Open()

Write-Host "=== PPGclip Serial Monitor (COM3 @ 115200) ===" -ForegroundColor Green
Write-Host "Press Ctrl+C to exit" -ForegroundColor Yellow
Write-Host "Press the button on ATOMS3 to export 60 seconds of data" -ForegroundColor Cyan
Write-Host ""

try {
    while ($true) {
        if ($port.BytesToRead -gt 0) {
            $line = $port.ReadLine()
            Write-Host $line
        }
        Start-Sleep -Milliseconds 10
    }
}
finally {
    $port.Close()
    Write-Host "`nPort closed." -ForegroundColor Yellow
}
