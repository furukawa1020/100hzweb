use std::collections::VecDeque;

pub struct SignalProcessor {
    // Buffer for raw data (for visualization or complex analysis if needed)
    raw_buffer: VecDeque<f64>,
    
    // Filtering
    filter_buffer: VecDeque<f64>,
    
    // Peak Detection
    last_peak_ts: f64,
    current_sample_ts: f64,
    peak_threshold: f64,
    sample_count: u64,
    
    // IBI (Inter-Beat Interval) History for HRV
    ibi_history: VecDeque<f64>,
    
    // Baseline for Normalization (Z-score equivalent logic)
    baseline_rmssd: f64,
}

impl SignalProcessor {
    pub fn new() -> Self {
        Self {
            raw_buffer: VecDeque::with_capacity(500),
            filter_buffer: VecDeque::with_capacity(10),
            last_peak_ts: 0.0,
            current_sample_ts: 0.0,
            peak_threshold: 1000.0, // Initial guess, adaptable?
            sample_count: 0,
            ibi_history: VecDeque::with_capacity(30), // Last ~30 beats
            baseline_rmssd: 50.0, // Standard resting RMSSD (ms)
        }
    }

    pub fn process_sample(&mut self, sample: f64) -> f64 {
        self.sample_count += 1;
        self.current_sample_ts = self.sample_count as f64 * 0.01; // 100Hz = 10ms per sample

        // 1. Simple High-pass / Band-pass filter (Placeholder: Delta)
        // A real bandpass is better, but for now using immediate slope + threshold
        // to detect systolic upstroke.
        self.filter_buffer.push_back(sample);
        if self.filter_buffer.len() > 5 {
            self.filter_buffer.pop_front();
        }
        
        // Simple Peak Detector
        // Look for local maximum within the small buffer that exceeds threshold
        let is_peak = self.detect_peak();

        if is_peak {
            let now = self.current_sample_ts;
            let time_since_last = now - self.last_peak_ts;
            
            // Refractory period: 250ms (Max HR ~240) to 2.0s (Min HR ~30)
            if time_since_last > 0.25 && time_since_last < 2.0 {
                let ibi_ms = time_since_last * 1000.0;
                self.ibi_history.push_back(ibi_ms);
                if self.ibi_history.len() > 20 {
                    self.ibi_history.pop_front();
                }
                self.last_peak_ts = now;
            } else if time_since_last >= 2.0 {
                 // Too long, reset
                 self.last_peak_ts = now;
            }
        }

        // 2. Calculate RMSSD
        let rmssd = self.calculate_rmssd();
        
        // 3. Map to Arousal r(t)
        // Lower RMSSD = Higher Stress.
        // We want r(t) to go UP when Stress goes UP.
        // Formula: r(t) = sigmoid( (Baseline - Current) / Scale )
        // Using simple inverse scaling for now.
        if rmssd > 0.0 {
            // Normalized roughly: resting RMSSD ~50ms. Stressed ~20ms.
            // (50 - rmssd) / 20 -> 0 at 50, +1.5 at 20.
            let r = (self.baseline_rmssd - rmssd) / 20.0; 
            return r.clamp(-2.0, 4.0); // Z-score like clamp
        }

        0.0 // Default / Neutral
    }

    // Very simple peak detector: returns true if the middle of filter_buffer is max
    // and exceeds dynamic threshold logic (simplified here to static for robustness as user requested "raw is fine")
    fn detect_peak(&self) -> bool {
        if self.filter_buffer.len() < 5 { return false; }
        
        let center = self.filter_buffer[2];
        let left = self.filter_buffer[1];
        let right = self.filter_buffer[3];
        
        // Local Maxima
        if center > left && center > right && center > 500.0 { // 500.0 is arbitrary min amplitude
            return true;
        }
        false
    }

    fn calculate_rmssd(&self) -> f64 {
        if self.ibi_history.len() < 2 {
            return self.baseline_rmssd; // Return baseline if not enough data
        }

        let mut sum_sq_diff = 0.0;
        let mut count = 0;
        
        for i in 0..self.ibi_history.len()-1 {
            let diff = self.ibi_history[i+1] - self.ibi_history[i];
            sum_sq_diff += diff * diff;
            count += 1;
        }

        if count == 0 { return self.baseline_rmssd; }
        
        (sum_sq_diff / count as f64).sqrt()
    }

    pub fn calibrate(&mut self) {
        // Set current RMSSD as baseline
        let current = self.calculate_rmssd();
        if current > 0.0 {
            self.baseline_rmssd = current;
        }
    }
}
