mod pmdt;
mod signal_processor;

use leptos::*;
use pmdt::{PmdtEngine, TrialState, Frame};
use signal_processor::SignalProcessor;
use web_sys::{window, KeyboardEvent};
use std::cell::RefCell;
use std::rc::Rc;

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

    // Core Engines (Wrapped in Rc<RefCell> for access inside closures)
    let engine = Rc::new(RefCell::new(PmdtEngine::new(42)));
    let processor = Rc::new(RefCell::new(SignalProcessor::new()));

    // --- Key Event Handler ---
    let engine_clone = engine.clone();
    let processor_clone = processor.clone();
    let window_obj = window().unwrap();
    
    let key_handler = Closure::<dyn FnMut(_)>::new(move |ev: KeyboardEvent| {
        let key = ev.key();
        let ts = window().unwrap().performance().unwrap().now() / 1000.0;
        let mut eng = engine_clone.borrow_mut();
        
        if key == " " && eng.state == TrialState::Idle {
            eng.start_session(ts);
        } else if key == "c" || key == "C" {
            processor_clone.borrow_mut().calibrate();
        } else if key == "1" { eng.frame = Frame::Neutral; }
        else if key == "2" { eng.frame = Frame::Threat; }
        else if key == "3" { eng.frame = Frame::Challenge; }
        else {
            eng.handle_input(&key, ts);
        }
    });

    window_obj.add_event_listener_with_callback("keydown", key_handler.as_ref().unchecked_ref()).unwrap();
    key_handler.forget();

    // --- High-Speed Update Loop (100Hz hypothetical or via WebSocket) ---
    // For simplicity, we trigger updates via a timer if no WebSocket yet
    set_interval(move || {
        let ts = window().unwrap().performance().unwrap().now() / 1000.0;
        let mut eng = engine.borrow_mut();
        
        // Hypothetical r(t) injection (will be replaced by WebSocket)
        let arousal = 0.0; // Placeholder
        eng.update_parameters_based_on_frame(arousal);
        eng.update(ts);

        // Sync Signals
        set_score.set(eng.score);
        set_combo.set(eng.combo);
        set_vis_opacity.set(eng.visual_opacity);
        set_vis_scale.set(eng.visual_scale);
        set_current_frame.set(format!("{:?}", eng.frame));

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
    }, std::time::Duration::from_millis(10));

    view! {
        <div style="background: black; color: white; height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; font-family: sans-serif;">
            <div style="position: absolute; top: 10px; left: 10px;">
                "Frame: " {move || current_frame.get()} <br/>
                "Score: " {move || score.get()} " | Combo: " {move || combo.get()}
            </div>
            
            <div style="font-size: 4em;">{move || center_text.get()}</div>
            
            <div style=move || format!("display: flex; gap: 100px; font-size: {}em; opacity: {}; transition: 0.1s;", 3.0 * vis_scale.get(), vis_opacity.get())>
                <div>{move || left_text.get()}</div>
                <div>{move || right_text.get()}</div>
            </div>

            <div style="position: absolute; bottom: 20px; color: #555;">
                "A: Left More | L: Right More | [1,2,3]: Switch Frame | C: Calibrate"
            </div>
        </div>
    }
}
