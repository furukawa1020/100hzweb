use web_sys::{AudioContext, OscillatorNode, GainNode};
use wasm_bindgen::prelude::*;

pub struct AudioManager {
    ctx: AudioContext,
    osc: Option<OscillatorNode>,
    gain: Option<GainNode>,
    next_beat_time: f64,
}

impl AudioManager {
    pub fn new() -> Self {
        let ctx = AudioContext::new().unwrap();
        Self {
            ctx,
            osc: None,
            gain: None,
            next_beat_time: 0.0,
        }
    }

    pub fn play_beat(&mut self, arousal: f64) {
        let now = self.ctx.current_time();
        
        // Simple heartbeat logic using existing arousal r(t)
        // Base rate 60 BPM (1.0s interval) + arousal factor
        // arousal is Z-score (~ -2.0 to +4.0). 
        // We map r=0 -> 60 BPM, r=3 -> 120 BPM
        let interval = 60.0 / (60.0 + (arousal * 20.0).max(0.0));
        
        if now >= self.next_beat_time {
            self.trigger_sound(440.0, 0.1); // "Low" beat
            self.trigger_sound(300.0, 0.1); // "High" beat (double beat effect)
            self.next_beat_time = now + interval;
        }
    }

    fn trigger_sound(&self, freq: f32, duration: f64) {
        let osc = self.ctx.create_oscillator().unwrap();
        let gain = self.ctx.create_gain().unwrap();
        
        osc.frequency().set_value(freq);
        osc.connect_with_audio_node(&gain).unwrap();
        gain.connect_with_audio_node(&self.ctx.destination()).unwrap();
        
        osc.start().unwrap();
        let now = self.ctx.current_time();
        gain.gain().set_value_at_time(0.1, now).unwrap();
        gain.gain().exponential_ramp_to_value_at_time(0.001, now + duration).unwrap();
        osc.stop_with_when(now + duration + 0.1).unwrap();
    }
}
