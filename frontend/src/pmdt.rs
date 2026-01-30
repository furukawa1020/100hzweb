use rand::prelude::*;
use rand::rngs::SmallRng;
use std::time::Duration;

// --- Constants ---
const FIXATION_MS: u64 = 200;
const FEEDBACK_MS: u64 = 500;
const TRIAL_TIMEOUT_MS: u64 = 2000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrialState {
    Idle,       // Waiting to start
    Fixation,   // Cross hair
    Stimulus,   // Task active
    Feedback,   // Result display
    BlockBreak, // Rest between blocks
}

#[derive(Clone, Debug)]
pub struct Stimulus {
    pub left_count: u32,
    pub right_count: u32,
    pub is_left_correct: bool,
    pub text_left: String,
    pub text_right: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Frame {
    Neutral,
    Threat,
    Challenge,
}

pub struct PmdtEngine {
    // State
    pub state: TrialState,
    pub frame: Frame,
    
    // Timing
    state_start_ts: f64,
    pub current_timeout: f64, // ms
    
    // Task Parameters (Controlled by Frame + r(t))
    pub difficulty: f64, // 0.0 (Easy) to 1.0 (Hard)
    pub visual_opacity: f64, // 1.0 (Clear) to 0.1 (Faint)
    pub visual_scale: f64,   // 1.0 (Normal) to 0.5 (Small/Far)
    
    // Game State
    pub score: i32,
    pub combo: u32,
    
    // Last Trial Stats (for Logging)
    pub last_correct: bool,
    pub last_rt: f64,
    
    // Current Trial Data
    pub current_stimulus: Option<Stimulus>,
    rng: SmallRng,
    
    // Stats
    pub total_trials: u32,
    pub correct_count: u32,
}

impl PmdtEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            state: TrialState::Idle,
            frame: Frame::Neutral, 
            state_start_ts: 0.0,
            current_timeout: 2000.0,
            
            difficulty: 0.5,
            visual_opacity: 1.0,
            visual_scale: 1.0,
            
            score: 0,
            combo: 0,
            last_correct: false,
            last_rt: 0.0,
            current_stimulus: None,
            rng: SmallRng::seed_from_u64(seed),
            total_trials: 0,
            correct_count: 0,
        }
    }

    pub fn start_session(&mut self, ts: f64) {
        // Reset metrics but keep Seed?
        // For strict experiments, maybe reset seed too or log it.
        self.transition_to(TrialState::Fixation, ts);
    }
    
    /// Update logic driven by r(t)
    /// r_val: The vascular tone metric (approx 0.0 - 5.0 range usually, normalized here implied)
    pub fn update_parameters_based_on_frame(&mut self, r_val: f64) {
        // Normalize r_val typical usage: 0 (Relax) to 10+ (Stress/Arousal)
        // Let's assume input r is somewhat scaled or we scale it here. 
        // User spec: High r(t) -> Effect
        
        let arousal = r_val.max(0.0).min(5.0) / 5.0; // 0.0 - 1.0 normalized for logic

        match self.frame {
            Frame::Neutral => {
                self.difficulty = 0.5;
                self.visual_opacity = 1.0;
                self.visual_scale = 1.0;
                self.current_timeout = 2000.0;
            },
            Frame::Threat => {
                // Threat: Arousal -> "Bad" things happen
                // Difficulty UP, Opacity DOWN, Time DOWN
                self.difficulty = 0.3 + (0.7 * arousal); // Harder
                self.visual_opacity = 1.0 - (0.8 * arousal); // Fades out
                self.visual_scale = 1.0 - (0.5 * arousal);   // Shrinks (Distances)
                self.current_timeout = 2000.0 - (1500.0 * arousal); // 2000ms -> 500ms
            },
            Frame::Challenge => {
                // Challenge: Arousal -> "Good" things happen (Flow state support)
                // Difficulty DOWN (or optimal), Opacity HIGH, Time UP
                // Actually Flow requires balancing difficulty, but spec says "Discrimination easier"
                self.difficulty = 0.5 - (0.3 * arousal); // Easier
                self.visual_opacity = 1.0; // Stays clear or gets "Sharper" (CSS glow?)
                self.visual_scale = 1.0 + (0.2 * arousal); // Zooms in
                self.current_timeout = 2000.0 + (1000.0 * arousal); // More time
            }
        }
    }

    pub fn update(&mut self, current_ts: f64) {
        let elapsed = (current_ts - self.state_start_ts) * 1000.0; // ms

        match self.state {
            TrialState::Fixation => {
                if elapsed >= FIXATION_MS as f64 {
                    self.generate_stimulus();
                    self.transition_to(TrialState::Stimulus, current_ts);
                }
            }
            TrialState::Stimulus => {
                if elapsed >= self.current_timeout {
                    self.handle_timeout(current_ts);
                }
            }
            TrialState::Feedback => {
                if elapsed >= FEEDBACK_MS as f64 {
                    self.transition_to(TrialState::Fixation, current_ts);
                }
            }
            _ => {}
        }
    }

    pub fn handle_input(&mut self, key: &str, current_ts: f64) {
        if self.state != TrialState::Stimulus { return; }

        if let Some(stim) = &self.current_stimulus {
            // A (Left) or L (Right) - Case insensitive
            let is_a = key.eq_ignore_ascii_case("a");
            let is_l = key.eq_ignore_ascii_case("l");
            
            if !is_a && !is_l { return; }

            let user_chose_left = is_a;
            let correct = if user_chose_left { stim.is_left_correct } else { !stim.is_left_correct };
            
            self.conclude_trial(correct, current_ts);
        }
    }

    fn generate_stimulus(&mut self) {
        // Visual Density Task
        // Base density: ~10 chars
        // Difficulty controls the DELTA between Left and Right.
        // Diff 0.0 -> Large Delta (Easy) e.g. 10 vs 5
        // Diff 1.0 -> Small Delta (Hard) e.g. 10 vs 9
        
        let base_count = 12; // Base number of items
        
        // Map difficulty 0.0-1.0 to delta range [8..1]
        // 0.0 -> delta 6
        // 1.0 -> delta 1
        let max_delta = 6.0;
        let delta = (max_delta * (1.0 - self.difficulty)).ceil() as u32;
        let delta = delta.max(1); 
        
        let target = base_count;
        let distractor = base_count - delta;
        
        let is_left_correct = self.rng.gen_bool(0.5);
        let (l, r) = if is_left_correct { (target, distractor) } else { (distractor, target) };

        // Generate Strings (Visual Pattern)
        // Using "▲" or "<" / ">" based on direction?
        // User example: "<<<<< >>>>" or "▲▲▲ ▲▲"
        // Let's use a solid shape for density: "■" or "●" or "▲"
        let shape = "▲"; 
        
        // Spacer for visuals? No, just the string length.
        let text_left = shape.repeat(l as usize);
        let text_right = shape.repeat(r as usize);

        self.current_stimulus = Some(Stimulus {
            left_count: l,
            right_count: r,
            is_left_correct,
            text_left,
            text_right,
        });
    }

    fn handle_timeout(&mut self, ts: f64) {
        self.conclude_trial(false, ts);
    }

    fn conclude_trial(&mut self, correct: bool, ts: f64) {
        // Calculate RT
        let rt = (ts - self.state_start_ts) * 1000.0;
        self.last_rt = rt;
        self.last_correct = correct;

        self.total_trials += 1;
        if correct {
            self.correct_count += 1;
            self.combo += 1;
            self.score += 10 + (self.combo as i32);
        } else {
            self.combo = 0;
            // Penalty in Threat frame?
            if self.frame == Frame::Threat {
                self.score -= 20;
            }
        }
        self.transition_to(TrialState::Feedback, ts);
    }

    fn transition_to(&mut self, new_state: TrialState, ts: f64) {
        self.state = new_state;
        self.state_start_ts = ts;
    }
}
