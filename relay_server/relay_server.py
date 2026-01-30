import asyncio
import websockets
import serial
import serial.tools.list_ports
import time
import json
import csv
import os
from datetime import datetime

# Configuration
SERIAL_BAUDRATE = 115200
WS_PORT = 8080
DATA_DIR = "data"

# Global state
connected_clients = set()
current_csv_writer = None
csv_file_handle = None

async def broadcast(message):
    if not connected_clients:
        return
    await asyncio.gather(*[client.send(message) for client in connected_clients], return_exceptions=True)

async def handler(websocket):
    print(f"Client connected: {websocket.remote_address}")
    connected_clients.add(websocket)
    try:
        await websocket.wait_closed()
    finally:
        connected_clients.remove(websocket)
        print(f"Client disconnected: {websocket.remote_address}")

def find_m5atom_port():
    ports = list(serial.tools.list_ports.comports())
    for p in ports:
        # Heuristic: M5Stack often shows up as "USB Serial" or has specific VID/PID
        # For now, just print them and pick the first probable one or let user specify
        print(f"Found port: {p.device} - {p.description}")
        if "USB" in p.description or "Serial" in p.description:
            return p.device
    return None

def setup_logging():
    global current_csv_writer, csv_file_handle
    if not os.path.exists(DATA_DIR):
        os.makedirs(DATA_DIR)
    
    filename = f"{DATA_DIR}/session_{datetime.now().strftime('%Y%m%d_%H%M%S')}.csv"
    csv_file_handle = open(filename, 'w', newline='')
    current_csv_writer = csv.writer(csv_file_handle)
    # Header: ServerTime, Sequence, IR, Red, FiltIR, FiltRed
    current_csv_writer.writerow(["server_ts", "seq", "ir", "red", "filt_ir", "filt_red"])
    print(f"Logging to {filename}")

async def serial_reader(port):
    print(f"Opening serial port {port}...")
    try:
        ser = serial.Serial(port, SERIAL_BAUDRATE, timeout=0.1)
    except Exception as e:
        print(f"Failed to open serial port: {e}")
        return

    print("Serial connected. Listening for data...")
    
    # Clear buffer
    ser.reset_input_buffer()

    while True:
        try:
            if ser.in_waiting:
                line = ser.readline().decode('utf-8', errors='ignore').strip()
                if not line:
                    continue
                
                # Expected format: "S,seq,ir,red,filtIR,filtRed"
                if line.startswith("S,"):
                    parts = line.split(',')
                    if len(parts) >= 6: # S, seq, ir, raw, filtIR, filtRed
                        try:
                            seq = int(parts[1])
                            ir = int(parts[2])
                            red = int(parts[3])
                            filt_ir = float(parts[4])
                            filt_red = float(parts[5])
                            ts = time.time() # High resolution server time
                            
                            # 1. Log to CSV
                            if current_csv_writer:
                                current_csv_writer.writerow([ts, seq, ir, red, filt_ir, filt_red])
                            
                            # 2. Broadcast via WebSocket
                            # Including filtered values for frontend visualization
                            payload = json.dumps({
                                "type": "sample",
                                "ts": ts,
                                "seq": seq,
                                "ir": ir,
                                "red": red,
                                "filt_ir": filt_ir,
                                "filt_red": filt_red
                            })
                            
                            await broadcast(payload)

                        except ValueError:
                            pass # Malformed numbers
            else:
                await asyncio.sleep(0.001) # Yield slightly
                
        except Exception as e:
            print(f"Serial read error: {e}")
            break

async def main():
    port = find_m5atom_port()
    if not port:
        print("No serial port found. Please connect M5Atom.")
        # Fallback for testing?
        # port = "COM3" 
        return

    setup_logging()

    # Start WebSocket Server
    async with websockets.serve(handler, "localhost", WS_PORT):
        print(f"WebSocket server started on ws://localhost:{WS_PORT}")
        await serial_reader(port)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("Stopping...")
        if csv_file_handle:
            csv_file_handle.close()
