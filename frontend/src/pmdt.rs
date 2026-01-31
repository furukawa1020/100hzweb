use rand::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Frame {
    Neutral,
    Threat,
    Challenge,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrialState {
    Idle,
    Fixation,
    Stimulus,
    Feedback,
}

pub struct PmdtEngine {
    pub state: TrialState,
    pub frame: Frame,
    pub score: i32,
    pub combo: i32,
    
    // Parameters controlled by r(t)
    pub difficulty: f64, 
    pub visual_opacity: f64,
    pub visual_scale: f64,
    
    // Task Internal
    pub last_correct: bool,
    pub total_trials: usize,
    rng: SmallRng,
    
    // Trial stimuli
    pub left_val: String,
    pub right_val: String,
    pub correct_key: String,
    
    // Timing
    state_start_ts: f64,
}

impl PmdtEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            state: TrialState::Idle,
            frame: Frame::Neutral,
            score: 0,
            combo: 0,
            difficulty: 0.5,
            visual_opacity: 1.0,
            visual_scale: 1.0,
            last_correct: false,
            total_trials: 0,
            rng: SmallRng::seed_from_u64(seed),
            left_val: "".to_string(),
            right_val: "".to_string(),
            correct_key: "".to_string(),
            state_start_ts: 0.0,
        }
    }

    pub fn start_session(&mut self, ts: f64) {
        self.total_trials = 0;
        self.score = 0;
        self.next_state(TrialState::Fixation, ts);
    }

    pub fn update(&mut self, ts: f64) {
        let elapsed = ts - self.state_start_ts;
        
        match self.state {
            TrialState::Fixation => {
                if elapsed > 1.0 { // 1s fixation
                    self.generate_trial();
                    self.next_state(TrialState::Stimulus, ts);
                }
            }
            TrialState::Stimulus => {
                // Timeout logic
                let timeout = 2.0; // 2s default
                if elapsed > timeout {
                    self.conclude_trial(false, ts);
                }
            }
            TrialState::Feedback => {
                if elapsed > 0.5 { // 0.5s feedback
                    self.next_state(TrialState::Fixation, ts);
                }
            }
            _ => {}
        }
    }

    pub fn handle_input(&mut self, key: &str, ts: f64) {
        if self.state != TrialState::Stimulus { return; }
        
        let correct = key.to_uppercase() == self.correct_key;
        self.conclude_trial(correct, ts);
    }

    fn generate_trial(&mut self) {
        // Simple density task: "▲▲▲" vs "▲▲▲▲▲"
        // Difficulty controls how close the densities are
        let base = 5;
        let diff = if self.difficulty > 0.8 { 1 } else if self.difficulty > 0.5 { 2 } else { 3 };
        
        let side = self.rng.gen_bool(0.5);
        let (l, r) = if side { (base + diff, base) } else { (base, base + diff) };
        
        self.left_val = "▲".repeat(l);
        self.right_val = "▲".repeat(r);
        self.correct_key = if side { "A".to_string() } else { "L".to_string() };
    }

    fn conclude_trial(&mut self, correct: bool, ts: f64) {
        self.last_correct = correct;
        if correct {
            self.score += 10;
            self.combo += 1;
        } else {
            self.combo = 0;
        }
        self.total_trials += 1;
        self.next_state(TrialState::Feedback, ts);
    }

    fn next_state(&mut self, next: TrialState, ts: f64) {
        self.state = next;
        self.state_start_ts = ts;
    }

    pub fn update_parameters_based_on_frame(&mut self, arousal: f64) {
        // Lazarus Loop Implementation
        match self.frame {
            Frame::Neutral => {
                self.difficulty = 0.5;
                self.visual_opacity = 1.0;
                self.visual_scale = 1.0;
            }
            Frame::Threat => {
                // Arousal triggers "Hard/Hostile" UI
                self.difficulty = 0.5 + (0.5 * arousal).clamp(0.0, 0.4); 
                self.visual_opacity = (1.0 - arousal).clamp(0.1, 1.0);
                self.visual_scale = (1.0 - 0.2 * arousal).clamp(0.5, 1.0);
            }
            Frame::Challenge => {
                // Arousal triggers "Better/Flow" UI
                self.difficulty = 0.5 - (0.3 * arousal).clamp(0.0, 0.4);
                self.visual_opacity = 1.0;
                self.visual_scale = (1.0 + 0.5 * arousal).clamp(1.0, 2.0);
            }
        }
    }
}
