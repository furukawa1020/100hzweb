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
current_event_writer = None
event_file_handle = None

async def broadcast(message):
    if not connected_clients:
        return
    await asyncio.gather(*[client.send(message) for client in connected_clients], return_exceptions=True)

async def handler(websocket):
    global current_event_writer, event_file_handle
    print(f"Client connected: {websocket.remote_address}")
    connected_clients.add(websocket)
    try:
        async for message in websocket:
            try:
                data = json.loads(message)
                
                # 1. Subject ID Setup
                if data.get("type") == "set_subject_id":
                    sid = data.get("subject_id", "test")
                    print(f"Setting Subject ID: {sid}")
                    # Re-initialize logging with new filename
                    setup_logging(sid)
                    setup_event_logging(sid)
                
                # 2. Event Logging
                elif data.get("type") == "log_event":
                    if current_event_writer:
                        ts = time.time()
                        current_event_writer.writerow([
                            ts,
                            data.get("event_type", "unknown"),
                            data.get("trial_id", ""),
                            data.get("frame", ""),
                            data.get("difficulty", ""),
                            data.get("correct", ""),
                            data.get("rt_ms", ""),
                            data.get("physio_r", ""),
                            data.get("vis_opacity", ""),
                            data.get("vis_scale", ""),
                            data.get("details", "")
                        ])
                        event_file_handle.flush()
            except Exception as e:
                print(f"Error handling message: {e}")
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

def setup_logging(subject_id="default"):
    global current_csv_writer, csv_file_handle
    
    if csv_file_handle:
        csv_file_handle.close()

    if not os.path.exists(DATA_DIR):
        os.makedirs(DATA_DIR)
    
    filename = f"{DATA_DIR}/{subject_id}_raw_{datetime.now().strftime('%Y%m%d_%H%M%S')}.csv"
    csv_file_handle = open(filename, 'w', newline='')
    current_csv_writer = csv.writer(csv_file_handle)
    # Header: ServerTime, Sequence, IR, Red, FiltIR, FiltRed
    current_csv_writer.writerow(["server_ts", "seq", "ir", "red", "filt_ir", "filt_red"])
    print(f"Raw logging to {filename}")

def setup_event_logging(subject_id="default"):
    global current_event_writer, event_file_handle
    
    if event_file_handle:
        event_file_handle.close()

    if not os.path.exists(DATA_DIR):
        os.makedirs(DATA_DIR)
    
    filename = f"{DATA_DIR}/{subject_id}_events_{datetime.now().strftime('%Y%m%d_%H%M%S')}.csv"
    event_file_handle = open(filename, 'w', newline='')
    current_event_writer = csv.writer(event_file_handle)
    # Header for SCED analysis
    current_event_writer.writerow([
        "server_ts", "event_type", "trial_id", "frame", 
        "difficulty", "correct", "rt_ms", "physio_r", 
        "vis_opacity", "vis_scale", "details"
    ])
    print(f"Event logging to {filename}")

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
                            payload_dict = {
                                "type": "sample",
                                "ts": ts,
                                "seq": seq,
                                "ir": ir,
                                "red": red,
                                "filt_ir": filt_ir,
                                "filt_red": filt_red
                            }
                            await broadcast(json.dumps(payload_dict))

                        except (ValueError, IndexError):
                            pass # Malformed packet
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
    setup_event_logging()

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
