mod pmdt;
mod signal_processor;
mod audio;

use leptos::*;
use pmdt::{PmdtEngine, TrialState, Frame};
use signal_processor::SignalProcessor;
use audio::AudioManager;
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
    let (vis_jitter, set_vis_jitter) = create_signal(0.0);

    // Core Engines
    let engine = Rc::new(RefCell::new(PmdtEngine::new(42)));
    let processor = Rc::new(RefCell::new(SignalProcessor::new()));
    let audio = Rc::new(RefCell::new(AudioManager::new()));

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
                    set_latest_r.set(r_t);
                    
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

    let (subject_id, set_subject_id) = create_signal("P001".to_string());
    
    // Graph History (Store last 100 points for visualization)
    let (r_history, set_r_history) = create_signal(vec![0.0; 100]);

    // --- Key Events ---
    let engine_kb = engine.clone();
    let proc_kb = processor.clone();
    let ws_kb = ws.clone();
    
    window_event_listener(ev::keydown, move |ev| {
        let key = ev.key();
        let ts = window().unwrap().performance().unwrap().now() / 1000.0;
        let mut eng = engine_kb.borrow_mut();
        
        // Prevent default for Space to avoid scrolling
        if key == " " {
            ev.prevent_default();
        }

        if key == " " && eng.state == TrialState::Idle {
            // Start Session: Send Subject ID
            let sid_msg = serde_json::json!({
                "type": "set_subject_id",
                "subject_id": subject_id.get_untracked()
            });
            let _ = ws_kb.send_with_str(&sid_msg.to_string());
            
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
    let audio_sync = audio.clone();

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

        // Audio Feedback
        let arousal = latest_r.get_untracked();
        if eng.state == TrialState::Stimulus {
             audio_sync.borrow_mut().play_beat(arousal);
        }

        // Jitter effect derived from arousal if in Threat frame
        if eng.frame == Frame::Threat && arousal > 0.5 {
            set_vis_jitter.set((arousal * 8.0).min(15.0));
        } else {
            set_vis_jitter.set(0.0);
        }
        
        // Update Graph History
        set_r_history.update(|hist| {
            hist.push(arousal);
            if hist.len() > 100 { hist.remove(0); }
        });

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
            TrialState::Idle => set_center_text.set("SPACE TO START".to_string()),
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
                set_center_text.set(if eng.last_correct { "CORRECT" } else { "MISS" }.to_string());
                set_left_text.set("".to_string());
                set_right_text.set("".to_string());
            }
        }
    }, std::time::Duration::from_millis(16));

    view! {
        <div style="background: black; color: white; height: 100vh; overflow: hidden; display: flex; flex-direction: column; align-items: center; justify-content: center; font-family: 'Inter', sans-serif; transition: background 0.5s;">
            <div style="position: absolute; top: 20px; left: 20px; font-family: monospace; font-size: 0.9em; opacity: 0.7;">
                "SYSTEM: VASC-LAB v1.2" <br/>
                "STATUS: " {move || status.get()} <br/>
                "FRAME: " <span style=move || if current_frame.get() == "Threat" { "color: #ff3333" } else if current_frame.get() == "Challenge" { "color: #33ff33" } else { "color: white" }>{move || current_frame.get()}</span> <br/>
                "r(t): " {move || format!("{:.3}", latest_r.get())} <br/>
                
                // Subject ID Input
                <div style="margin-top: 10px; display: flex; align-items: center; gap: 5px;">
                    "ID: "
                    <input 
                        type="text" 
                        prop:value=move || subject_id.get()
                        on:input=move |ev| set_subject_id.set(event_target_value(&ev))
                        style="background: #333; color: white; border: 1px solid #555; padding: 2px 5px; width: 60px;"
                    />
                </div>
                
                // Real-time Graph (Simple SVG)
                <svg width="200" height="60" style="background: #111; border: 1px solid #333; margin-top: 5px;">
                    <polyline
                        points=move || {
                            r_history.get().iter().enumerate()
                                .map(|(i, &r)| format!("{},{}", i * 2, 60.0 - (r * 10.0 + 30.0).clamp(0.0, 60.0)))
                                .collect::<Vec<_>>().join(" ")
                        }
                        fill="none"
                        stroke="#00ffcc"
                        stroke-width="2"
                    />
                </svg>
            </div>

            <div style="position: absolute; top: 20px; right: 20px; text-align: right; font-family: monospace;">
                "SCORE: " <span style="font-size: 1.5em; color: #fbff00;">{move || score.get()}</span> <br/>
                "COMBO: " {move || combo.get()} <br/>
                <div style="margin-top: 10px; font-size: 0.8em; color: #aaa;">
                    "ACC: " {move || {
                        let eng = engine_sync.borrow();
                        if eng.total_trials > 0 {
                            format!("{:.1}%", (eng.score as f64 / eng.total_trials as f64) * 100.0)
                        } else {
                            "---".to_string()
                        }
                    }} <br/>
                    "RT: " {move || {
                        let eng = engine_sync.borrow();
                        if eng.last_rt > 0.0 {
                            format!("{:.0} ms", eng.last_rt)
                        } else {
                           "---".to_string()
                        }
                    }}
                </div>
            </div>
            
            <div style=move || {
                let j = vis_jitter.get();
                format!("font-size: 5em; font-weight: bold; transition: 0.1s; transform: translate({}px, {}px); color: {};", 
                    (rand::random::<f64>() - 0.5) * j,
                    (rand::random::<f64>() - 0.5) * j,
                    if center_text.get() == "MISS" { "#ff3333" } else if center_text.get() == "CORRECT" { "#33ff33" } else { "white" }
                )
            }>
                {move || center_text.get()}
            </div>
            
            <div style=move || {
                let j = vis_jitter.get();
                format!(
                    "display: flex; gap: 120px; font-size: {}em; opacity: {}; transform: scale({}) translate({}px, {}px); transition: 0.1s; filter: blur({}px);", 
                    3.0 * vis_scale.get(), 
                    vis_opacity.get(), 
                    vis_scale.get(),
                    (rand::random::<f64>() - 0.5) * j,
                    (rand::random::<f64>() - 0.5) * j,
                    if current_frame.get() == "Threat" { (1.0 - vis_opacity.get()) * 8.0 } else { 0.0 }
                )
            }>
                <div>{move || left_text.get()}</div>
                <div>{move || right_text.get()}</div>
            </div>

            <div style="position: absolute; bottom: 30px; color: #444; font-size: 0.8em; letter-spacing: 0.1em; font-family: monospace;">
                "A: LEFT | L: RIGHT | C: CALIBRATE | SPACE: START"
            </div>
        </div>
    }
}
