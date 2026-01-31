mod pmdt;
mod signal_processor;
use pmdt::{PmdtEngine, TrialState, Frame};
use signal_processor::SignalProcessor;

#[component]
pub fn App() -> impl IntoView {
    // Signals
    let (status, set_status) = create_signal("Disconnected".to_string());
    let (monitor_ir, set_monitor_ir) = create_signal(0.0);
    let (latest_r, set_latest_r) = create_signal(0.0);
    
    // PMDT UI Signals
    let (left_text, set_left_text) = create_signal("".to_string());
    let (right_text, set_right_text) = create_signal("".to_string());
    let (center_text, set_center_text) = create_signal("Press SPACE".to_string());
    
    // Visual Parameters
    let (vis_opacity, set_vis_opacity) = create_signal(1.0);
    let (vis_scale, set_vis_scale) = create_signal(1.0);
    
    let (score, set_score) = create_signal(0);
    let (combo, set_combo) = create_signal(0);
    let (debug_diff, set_debug_diff) = create_signal(0.5);
    let (current_frame, set_current_frame) = create_signal("Neutral".to_string());

    // Core Logic (Refs for mutability without re-render loops)
    use std::cell::RefCell;
    use std::rc::Rc;
    let processor = Rc::new(RefCell::new(SignalProcessor::new()));
    let engine = Rc::new(RefCell::new(PmdtEngine::new(12345))); // Fixed seed 12345

    // --- Key Event Listener ---
    let engine_key_input = engine.clone();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    
    let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
        let key = event.key();
        let ts = window.performance().unwrap().now() / 1000.0;
        
        let mut eng = engine_key_input.borrow_mut();
        
        // Debug: Frame Switching with 1, 2, 3
        if key == "1" { eng.frame = Frame::Neutral; }
        else if key == "2" { eng.frame = Frame::Threat; }
        else if key == "3" { eng.frame = Frame::Challenge; }
        else if key == "c" || key == "C" { 
            // Trigger Calibration in Processor
             let mut proc = processor.borrow_mut();
             proc.calibrate();
             web_sys::console::log_1(&"Calibrated Baseline".into());
        }
        
        if eng.state == TrialState::Idle && key == " " {
             eng.start_session(ts);
        } else {
             eng.handle_input(&key, ts);
        }
    });
    document.body().unwrap().add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();

    // --- WebSocket & Loop ---
    let engine_loop = engine.clone();
    
    create_effect(move |_| {
        let ws: WebSocket = WebSocket::new("ws://localhost:8080").unwrap();
        set_status.set("Connecting...".to_string());

        let processor_clone = processor.clone();
        let engine_clone = engine_loop.clone();
        let ws_sender = ws.clone();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                if let Ok(msg) = serde_json::from_str::<ServerMessage>(&txt.into()) {
                    let browser_ts = web_sys::window().unwrap().performance().unwrap().now() / 1000.0;

                    // 1. Signal Processing
                    set_monitor_ir.set(msg.filt_ir.unwrap_or(msg.ir));
                    
                    let mut proc = processor_clone.borrow_mut();
                    
                    let r_val = if let Some(r) = proc.process_sample(msg.ir) {
                        set_latest_r.set(r);
                        r
                    } else {
                        latest_r.get_untracked()
                    };
                    
                    // 2. Game Loop Update (100Hz)
                    let mut eng = engine_clone.borrow_mut();
                    
                    // Capture state before update to detect trial completion
                    let prev_trials = eng.total_trials;

                    // Dynamic Parameter Update based on r(t) and Frame
                    eng.update_parameters_based_on_frame(r_val);
                    eng.update(browser_ts);
                    
                    // Check for trial completion to log data (SCED)
                    if eng.total_trials > prev_trials {
                         // A trial finished. Send log to server.
                         let log_msg = serde_json::json!({
                             "type": "log_event",
                             "event_type": "trial_result",
                             "trial_id": eng.total_trials,
                             "frame": format!("{:?}", eng.frame),
                             "difficulty": eng.difficulty,
                             "correct": eng.last_correct, // Need to expose this in PmdtEngine
                             "rt_ms": eng.last_rt,
                             "physio_r": r_val,
                             "vis_opacity": eng.visual_opacity,
                             "vis_scale": eng.visual_scale,
                             "details": format!("score:{}", eng.score)
                         });
                         // Send via WS
                         let _ = ws_sender.send_with_str(&log_msg.to_string());
                    }
                    
                    // 3. UI Sync
                    set_score.set(eng.score);
                    set_combo.set(eng.combo);
                    set_debug_diff.set(eng.difficulty);
                    set_vis_opacity.set(eng.visual_opacity);
                    set_vis_scale.set(eng.visual_scale);
                    set_current_frame.set(format!("{:?}", eng.frame));

                    match eng.state {
                        TrialState::Fixation => {
                             set_left_text.set("".to_string());
                             set_right_text.set("".to_string());
                             set_center_text.set("+".to_string());
                        },
                        TrialState::Stimulus => {
                            set_center_text.set("".to_string());
                            if let Some(s) = &eng.current_stimulus {
                                set_left_text.set(s.text_left.clone());
                                set_right_text.set(s.text_right.clone());
                            }
                        },
                        TrialState::Feedback => {
                            set_left_text.set("".to_string());
                            set_right_text.set("".to_string());
                            set_center_text.set("".to_string()); // Blink/Clear
                        },
                        _ => {
                             set_center_text.set("Press SPACE".to_string());
                        }
                    }
                }
            }
        });
        
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            set_status.set("Connected".to_string());
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    });

    view! {
        <div style="font-family: monospace; padding: 20px; background: #222; color: #eee; height: 100vh; overflow: hidden;">
            <div style="display: flex; justify-content: space-between;">
                <h1>"VASC-LAB PMDT"</h1>
                <div style="text-align: right;">
                    <div style="font-size: 1.5em; color: yellow;">{move || current_frame.get()}</div>
                    <div>"Score: " {move || score.get()}</div>
                    <div>"Combo: " {move || combo.get()}</div>
                </div>
            </div>

            <!-- Task Area -->
            <div style="
                position: relative;
                border: 2px solid #555; 
                margin: 40px 0; 
                height: 400px; 
                display: flex; 
                align_items: center; 
                justify_content: center;
                background: #000;
                user-select: none;
            ">
                <!-- Fixation / Message -->
                <div style="position: absolute; font-size: 3em; color: white;">
                    {move || center_text.get()}
                </div>

                <!-- Stimulus Container -->
                <div style={move || format!("
                    display: flex; 
                    width: 80%; 
                    justify-content: space-between; 
                    font-size: {}em; 
                    opacity: {};
                    transition: opacity 0.1s, font-size 0.1s;
                ", 3.0 * vis_scale.get(), vis_opacity.get())}>
                    
                    <!-- Left Stimulus -->
                    <div style="color: #0ff;">{move || left_text.get()}</div>
                    
                    <!-- Right Stimulus -->
                    <div style="color: #f0f;">{move || right_text.get()}</div>
                </div>
            </div>
            
            <!-- Key Guide -->
             <div style="display: flex; justify-content: space-between; width: 80%; margin: 0 auto; color: #888;">
                <div>"Press 'A' if Left is More"</div>
                <div>"Press 'L' if Right is More"</div>
            </div>

            <!-- Debug Info -->
            <div style="font-size: 0.8em; color: #aaa; margin-top: 50px; border-top: 1px solid #444; padding-top: 10px;">
                <div>"Sensor IR: " {move || format!("{:.1}", monitor_ir.get())}</div>
                <div>"r(t): " {move || format!("{:.3}", latest_r.get())}</div>
                <div>"Difficulty D(t): " {move || format!("{:.2}", debug_diff.get())}</div>
                <div>"DEBUG Keys: [1] Neutral [2] Threat [3] Challenge"</div>
            </div>
        </div>
    }
}
