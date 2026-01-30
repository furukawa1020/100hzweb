import serial
import time

try:
    print("Connecting to COM3...")
    ser = serial.Serial('COM3', 115200, timeout=2)
    time.sleep(2)
    print("Connected! Reading data for 10 seconds...")
    print("=" * 50)
    
    start = time.time()
    while time.time() - start < 10:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        if line:
            print(line)
    
    print("=" * 50)
    print("Done")
    ser.close()
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()
