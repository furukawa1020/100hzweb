use std::collections::VecDeque;

/// VASC-LAB Signal Processor
/// Focuses on 0.05-2Hz oscillation and calculates r(t)
pub struct SignalProcessor {
    buffer: VecDeque<f64>,
    window_size: usize,
    
    // IIR Filter coefficients (0.05-2Hz @ 100Hz)
    // Simplified 2nd order Butterworth for stability in WASM
    // (Actual coefficients would be generated/tuned)
    prev_raw: f64,
    prev_filtered: [f64; 2],
    prev_input: [f64; 2],

    // Session Statistics for Z-score
    session_sum: f64,
    session_sq_sum: f64,
    session_count: usize,
}

impl SignalProcessor {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(501),
            window_size: 500, // 5 seconds at 100Hz
            prev_raw: 0.0,
            prev_filtered: [0.0; 2],
            prev_input: [0.0; 2],
            session_sum: 0.0,
            session_sq_sum: 0.0,
            session_count: 0,
        }
    }

    /// Process raw IR sample and return current r(t)
    pub fn process_sample(&mut self, raw_ir: f64) -> f64 {
        // 1. IIR Bandpass (Dummy coefficients for structure)
        // In real impl, we use specific b, a coefficients
        // y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
        let filtered = self.apply_filter(raw_ir);

        // 2. Sliding Window
        if self.buffer.len() >= self.window_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(filtered);

        if self.buffer.len() < self.window_size {
            return 0.0; // Warming up
        }

        // 3. Extract Features: Sigma and Delta
        let (mean, sigma) = self.compute_stats();
        let delta = self.compute_slope(mean);

        // 4. Calculate raw r_raw(t)
        let r_raw = sigma + delta.abs();

        // 5. Normalization (Running Z-score)
        self.update_session_stats(r_raw);
        self.calculate_zscore(r_raw)
    }

    fn apply_filter(&mut self, x: f64) -> f64 {
        // Simple alpha filter as placeholder for 0.05-2Hz BP
        // (Full IIR implementation would go here)
        let alpha = 0.95;
        let y = alpha * self.prev_filtered[0] + (1.0 - alpha) * x;
        self.prev_filtered[0] = y;
        y
    }

    fn compute_stats(&self) -> (f64, f64) {
        let n = self.buffer.len() as f64;
        let sum: f64 = self.buffer.iter().sum();
        let mean = sum / n;
        let variance: f64 = self.buffer.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        (mean, variance.sqrt())
    }

    fn compute_slope(&self, current_mean: f64) -> f64 {
        // Simplified slope: Difference between current mean and mean of first half of window
        let half = self.window_size / 2;
        let start_mean: f64 = self.buffer.iter().take(half).sum::<f64>() / (half as f64);
        current_mean - start_mean
    }

    fn update_session_stats(&mut self, val: f64) {
        self.session_count += 1;
        self.session_sum += val;
        self.session_sq_sum += val.powi(2);
    }

    fn calculate_zscore(&self, val: f64) -> f64 {
        if self.session_count < 100 { return 0.0; }
        let n = self.session_count as f64;
        let mean = self.session_sum / n;
        let std = ((self.session_sq_sum / n) - mean.powi(2)).sqrt().max(0.0001);
        (val - mean) / std
    }
}
