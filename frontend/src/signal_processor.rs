use std::collections::VecDeque;

// Constants for 100Hz Signal Processing
pub const FS: f64 = 100.0;
pub const WINDOW_SIZE: usize = 500; // 5 seconds @ 100Hz
pub const UPDATE_INTERVAL: usize = 100; // Recalculate every 1s (100 samples)

/// Core Signal SignalProcessor
/// Designed for simplicity and reproducibility as per VASC-LAB specs.
pub struct SignalProcessor {
    // Buffers
    raw_buffer: VecDeque<f64>,
    filtered_buffer: VecDeque<f64>,
    
    // Filter States (Simple IIR)
    // HPF 0.02Hz to remove drift
    // LPF 5.0Hz to remove noise
    // Standard butterworth 2nd order or similar can be used. 
    // Here we use a direct implementation for transparency.
    prev_x: f64,
    prev_y: f64, 
    
    // Baseline statistics
    pub mu_0: f64,
    pub sigma_0: f64,
}

impl SignalProcessor {
    pub fn new() -> Self {
        Self {
            raw_buffer: VecDeque::with_capacity(WINDOW_SIZE),
            filtered_buffer: VecDeque::with_capacity(WINDOW_SIZE),
            prev_x: 0.0,
            prev_y: 0.0,
            mu_0: 1.0, // Prevent div by zero, calibrates later
            sigma_0: 1.0,
        }
    }

    /// Process a single raw sample from the sensor
    pub fn process_sample(&mut self, raw_ir: f64) -> Option<f64> {
        // 1. Preprocessing (Simple Drift Removal / DC blocker)
        // y[n] = x[n] - x[n-1] + R * y[n-1], R ~ 0.99 for ~0.5Hz cutoff equivalent
        // For 0.02Hz at 100Hz, R needs to be very close to 1.
        let alpha = 0.999; 
        let filtered_val = raw_ir - self.prev_x + alpha * self.prev_y;
        
        // Update state
        self.prev_x = raw_ir;
        self.prev_y = filtered_val;

        // 2. Buffer Management
        if self.filtered_buffer.len() >= WINDOW_SIZE {
            self.filtered_buffer.pop_front();
        }
        self.filtered_buffer.push_back(filtered_val);

        // 3. Check if we need to compute r(t)
        // Real-time update: return current r(t) if buffer is full
        if self.filtered_buffer.len() == WINDOW_SIZE {
            return Some(self.compute_rt());
        }

        None
    }

    /// Compute r(t) based on the VASC-LAB definition
    /// r(t) = Relative deviation from baseline.
    /// Uses Z-scores if calibrated.
    fn compute_rt(&self) -> f64 {
        let (_mu, sigma) = self.compute_stats();
        let slope = self.compute_slope();

        // If not calibrated (sigma_0 == 1.0 default), just return raw approximation
        // If calibrated, return (sigma / sigma_0) + impact of slope
        
        let normalized_sigma = if self.sigma_0 > 0.0001 { sigma / self.sigma_0 } else { sigma };
        
        // Slope also needs scaling. Let's assume slope is significant if it's high relative to signal noise.
        // For simple arousal proxy:
        // r(t) = (Current Variance / Baseline Variance) + Weight * |Slope|
        // If r(t) > 1.0, user is more aroused/active than baseline.
        
        let rt = normalized_sigma + (slope.abs() * 100.0); // Heuristic weight for slope
        
        // Log occasionally if needed, or return raw.
        rt
    }

    fn compute_stats(&self) -> (f64, f64) {
        let sum: f64 = self.filtered_buffer.iter().sum();
        let count = self.filtered_buffer.len() as f64;
        let mu = sum / count;

        let variance: f64 = self.filtered_buffer.iter()
            .map(|&x| (x - mu).powi(2))
            .sum::<f64>() / count;
        
        (mu, variance.sqrt())
    }

    fn compute_slope(&self) -> f64 {
        // Simple linear regression slope over the window
        let n = self.filtered_buffer.len() as f64;
        let sum_x: f64 = (0..self.filtered_buffer.len()).map(|i| i as f64).sum();
        let sum_y: f64 = self.filtered_buffer.iter().sum();
        let sum_xy: f64 = self.filtered_buffer.iter().enumerate()
            .map(|(i, &y)| (i as f64) * y)
            .sum();
        let sum_xx: f64 = (0..self.filtered_buffer.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x.powi(2));
        
        // Convert to per-second (sample rate adjustment)
        slope * FS
    }
    
    /// Call this during calibration Phase to set baselines
    pub fn calibrate(&mut self) {
        let (mu, sigma) = self.compute_stats();
            self.mu_0 = mu;
            self.sigma_0 = sigma;
    }
}
