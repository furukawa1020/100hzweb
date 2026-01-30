import serial
import time

try:
    print("=== ATOMS3 Debug Monitor ===")
    print("Connecting to COM3...")
    ser = serial.Serial('COM3', 115200, timeout=1)
    time.sleep(2)
    print("Connected!")
    print("\n--- Waiting for data (60 seconds) ---")
    print("PRESS THE ATOMS3 BUTTON NOW!")
    print("=" * 60)
    
    start = time.time()
    line_count = 0
    
    while time.time() - start < 60:
        if ser.in_waiting > 0:
            line = ser.readline().decode('utf-8', errors='ignore').strip()
            if line:
                line_count += 1
                print(f"[{line_count:04d}] {line}")
        else:
            # データがない場合は短く待機
            time.sleep(0.01)
    
    print("=" * 60)
    print(f"\nTotal lines received: {line_count}")
    
    if line_count == 0:
        print("\n⚠️  NO DATA RECEIVED!")
        print("\nTroubleshooting:")
        print("1. Check if ATOMS3 is powered on")
        print("2. Press the ATOMS3 button")
        print("3. Check USB cable connection")
        print("4. Verify firmware was uploaded successfully")
    
    ser.close()
    print("\nDone.")
    
except Exception as e:
    print(f"\n❌ Error: {e}")
    import traceback
    traceback.print_exc()
