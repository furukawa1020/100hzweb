use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};
use serde::{Deserialize, Serialize};

mod signal_processor;
use signal_processor::SignalProcessor;

#[derive(Debug, Deserialize, Clone)]
struct ServerMessage {
    ts: f64,
    seq: u32,
    ir: f64,
    red: f64,
    #[serde(default)] // Handle optional fields if legacy firmware connects
    filt_ir: Option<f64>, 
    #[serde(default)]
    filt_red: Option<f64>,
}

#[component]
pub fn App() -> impl IntoView {
    // Signals for UI
    let (status, set_status) = create_signal("Disconnected".to_string());
    let (monitor_ir, set_monitor_ir) = create_signal(0.0); // Display value (Teacher's Waveform)
    let (latest_r, set_latest_r) = create_signal(0.0);
    
    // Processor State (Mutable inside a RefCell/Signal wrapper?)
    // In Leptos, we can use a stored value or effect.
    // Since we need to mutate it on every WS message, we'll keep it simple.
    // WARNING: Thread safety in WASM is single-threaded, so RefCell is fine.
    // However, sticking to functional updates where possible.
    
    // Using a RefCell to hold the processor state across renders is efficient here.
    use std::cell::RefCell;
    use std::rc::Rc;
    let processor = Rc::new(RefCell::new(SignalProcessor::new()));

    // WebSocket Setup
    create_effect(move |_| {
        let ws = WebSocket::new("ws://localhost:8080").unwrap();
        set_status.set("Connecting...".to_string());

        let processor_clone = processor.clone();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let txt: String = txt.into();
                if let Ok(msg) = serde_json::from_str::<ServerMessage>(&txt) {
                    // Display: Prioritize Teacher's Waveform (filt_ir), fallback to Raw
                    let display_val = msg.filt_ir.unwrap_or(msg.ir);
                    set_monitor_ir.set(display_val);
                    
                    // Logic: Use Raw IR for strict r(t) calculation (VASC-LAB Spec)
                    // (Or should we use filt_ir? Spec 6.1 says Input: IR_raw. Stick to Raw.)
                    let mut proc = processor_clone.borrow_mut();
                    if let Some(r_val) = proc.process_sample(msg.ir) {
                        set_latest_r.set(r_val);
                    }
                }
            }
        });
        
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget(); // Leak memory intentionally for life of app

        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            set_status.set("Connected (100Hz Stream)".to_string());
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    });

    view! {
        <div style="font-family: sans-serif; padding: 20px;">
            <h1>"VASC-LAB Monitor"</h1>
            <div style="margin-bottom: 20px; padding: 10px; border: 1px solid #ccc;">
                <strong>"Status: "</strong> {move || status.get()} <br/>
                <strong>"Waveform (Teacher's): "</strong> {move || format!("{:.2}", monitor_ir.get())} <br/>
                <strong>"r(t) [Calc from Raw]: "</strong> {move || format!("{:.4}", latest_r.get())}
            </div>
            
            <div style="border: 1px solid #333; height: 200px; position: relative;">
                <p style="text-align: center; color: #888; padding-top: 80px;">
                    "Canvas Visualization would go here"
                </p>
            </div>
            
            <p style="font-size: 0.8em; color: #666;">
                "Processing running entirely in WebAssembly."
            </p>
        </div>
    }
}
