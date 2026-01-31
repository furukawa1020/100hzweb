use std::collections::VecDeque;

pub struct SignalProcessor {
    buffer: VecDeque<f64>,
    window_size: usize,
    
    // For Z-score normalization
    session_r_values: Vec<f64>, 
    mu_0: f64,
    sigma_0: f64,
}

impl SignalProcessor {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(501),
            window_size: 500,
            session_r_values: Vec::new(),
            mu_0: 0.0,
            sigma_0: 1.0,
        }
    }

    pub fn process_sample(&mut self, raw_ir: f64) -> f64 {
        // 1. Sliding Window (5s @ 100Hz = 500 samples)
        if self.buffer.len() >= self.window_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(raw_ir);

        if self.buffer.len() < self.window_size { return 0.0; }

        // 2. Features: sigma(t) and delta(t)
        let mean: f64 = self.buffer.iter().sum::<f64>() / self.window_size as f64;
        let variance: f64 = self.buffer.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / self.window_size as f64;
        let sigma = variance.sqrt();
        
        let delta = mean - (self.buffer.iter().take(250).sum::<f64>() / 250.0);

        // 3. r(t) = sigma + |delta|
        let r_raw = sigma + delta.abs();

        // 4. Normalized r(t) (Z-score relative to baseline/session)
        (r_raw - self.mu_0) / self.sigma_0
    }

    pub fn calibrate(&mut self) {
        // Simple baseline calibration: compute mu/sigma of current window
        let n = self.buffer.len() as f64;
        if n == 0.0 { return; }
        
        // We use the last 5s as baseline
        let mean: f64 = self.buffer.iter().sum::<f64>() / n;
        let variance: f64 = self.buffer.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        
        self.mu_0 = mean; // This should actually be r_raw baseline, but simplifying for now
        self.sigma_0 = variance.sqrt().max(0.0001);
    }
}
