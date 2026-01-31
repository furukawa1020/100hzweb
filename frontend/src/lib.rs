mod pmdt;
mod signal_processor;

use leptos::*;
use pmdt::{PmdtEngine, TrialState, Frame};
use signal_processor::SignalProcessor;
use web_sys::{window, MessageEvent, WebSocket};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // Signals for UI updates
    let (score, set_score) = create_signal(0);
    let (combo, set_combo) = create_signal(0);
    let (center_text, set_center_text) = create_signal("Press SPACE to Start".to_string());
    let (left_text, set_left_text) = create_signal("".to_string());
    let (right_text, set_right_text) = create_signal("".to_string());
    let (vis_opacity, set_vis_opacity) = create_signal(1.0);
    let (vis_scale, set_vis_scale) = create_signal(1.0);
    let (current_frame, set_current_frame) = create_signal("Neutral".to_string());
    let (status, set_status) = create_signal("Disconnected".to_string());
    let (latest_r, set_latest_r) = create_signal(0.0);

    // Core Engines
    let engine = Rc::new(RefCell::new(PmdtEngine::new(42)));
    let processor = Rc::new(RefCell::new(SignalProcessor::new()));

    // Shared WebSocket (wrapped in Rc for closure access)
    let ws_url = "ws://localhost:8080";
    let ws = Rc::new(WebSocket::new(ws_url).unwrap());
    
    // --- WebSocket Incoming (Data Loop) ---
    let engine_ws = engine.clone();
    let processor_ws = processor.clone();
    let ws_recv = ws.clone();
    
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |ev: MessageEvent| {
        if let Ok(txt) = ev.data().as_string() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&txt) {
                if msg["type"] == "sample" {
                    let ir = msg["ir"].as_f64().unwrap_or(0.0);
                    let mut proc = processor_ws.borrow_mut();
                    let r_t = proc.process_sample(ir);
                    
                    let mut eng = engine_ws.borrow_mut();
                    eng.update_parameters_based_on_frame(r_t);
                }
            }
        }
    });
    ws_recv.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let onopen = Closure::<dyn FnMut()>::new(move || set_status.set("Connected".to_string()));
    ws_recv.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // --- Key Events ---
    let engine_kb = engine.clone();
    let proc_kb = processor.clone();
    window_event_listener(ev::keydown, move |ev| {
        let key = ev.key();
        let ts = window().unwrap().performance().unwrap().now() / 1000.0;
        let mut eng = engine_kb.borrow_mut();
        
        if key == " " && eng.state == TrialState::Idle {
            eng.start_session(ts);
        } else if key == "c" || key == "C" {
            proc_kb.borrow_mut().calibrate();
        } else if key == "1" { eng.frame = Frame::Neutral; }
        else if key == "2" { eng.frame = Frame::Threat; }
        else if key == "3" { eng.frame = Frame::Challenge; }
        else {
            eng.handle_input(&key, ts);
        }
    });

    // --- High-Speed UI & Logging Sync ---
    let engine_sync = engine.clone();
    let ws_sync = ws.clone();
    set_interval(move || {
        let ts = window().unwrap().performance().unwrap().now() / 1000.0;
        let mut eng = engine_sync.borrow_mut();
        let prev_trials = eng.total_trials;
        
        eng.update(ts);

        // Sync Signals
        set_score.set(eng.score);
        set_combo.set(eng.combo);
        set_vis_opacity.set(eng.visual_opacity);
        set_vis_scale.set(eng.visual_scale);
        set_current_frame.set(format!("{:?}", eng.frame));

        // SCED Logging: Check if a trial just finished
        if eng.total_trials > prev_trials {
             let r_val = latest_r.get_untracked();
             let log_msg = serde_json::json!({
                 "type": "log_event",
                 "event_type": "trial_result",
                 "trial_id": eng.total_trials,
                 "frame": format!("{:?}", eng.frame),
                 "difficulty": eng.difficulty,
                 "correct": eng.last_correct,
                 "rt_ms": eng.last_rt,
                 "physio_r": r_val,
                 "vis_opacity": eng.visual_opacity,
                 "vis_scale": eng.visual_scale
             });
             let _ = ws_sync.send_with_str(&log_msg.to_string());
        }

        match eng.state {
            TrialState::Idle => set_center_text.set("Press SPACE to Start".to_string()),
            TrialState::Fixation => {
                set_center_text.set("+".to_string());
                set_left_text.set("".to_string());
                set_right_text.set("".to_string());
            }
            TrialState::Stimulus => {
                set_center_text.set("".to_string());
                set_left_text.set(eng.left_val.clone());
                set_right_text.set(eng.right_val.clone());
            }
            TrialState::Feedback => {
                set_center_text.set(if eng.last_correct { "O" } else { "X" }.to_string());
                set_left_text.set("".to_string());
                set_right_text.set("".to_string());
            }
        }
    }, std::time::Duration::from_millis(16));

    view! { ... }
}

    view! {
        <div style="background: black; color: white; height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; font-family: sans-serif;">
            <div style="position: absolute; top: 10px; left: 10px; font-family: monospace;">
                "Status: " {move || status.get()} <br/>
                "Frame: " {move || current_frame.get()} <br/>
                "r(t): " {move || format!("{:.3}", latest_r.get())} <br/>
                "Score: " {move || score.get()} " | Combo: " {move || combo.get()}
            </div>
            
            <div style="font-size: 4em;">{move || center_text.get()}</div>
            
            <div style=move || format!("display: flex; gap: 100px; font-size: {}em; opacity: {};", 3.0 * vis_scale.get(), vis_opacity.get())>
                <div>{move || left_text.get()}</div>
                <div>{move || right_text.get()}</div>
            </div>

            <div style="position: absolute; bottom: 20px; color: #555;">
                "A: Left More | L: Right More | [1,2,3]: Switch Frame | C: Calibrate"
            </div>
        </div>
    }
}
