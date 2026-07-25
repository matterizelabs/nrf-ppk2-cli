const HW_RATE: u32 = 100_000;

pub struct Downsampler {
    factor: u32,
    count: u32,
    sum: f64,
    bits: u8,
    actual_rate: u32,
}

impl Downsampler {
    pub fn new(target_rate: u32) -> Self {
        let clamped = target_rate.clamp(1, HW_RATE);
        let factor = ((HW_RATE as f64) / (clamped as f64)).round() as u32;
        let factor = factor.clamp(1, HW_RATE);
        let actual_rate = HW_RATE / factor;
        if actual_rate != clamped {
            eprintln!(
                "note: using {} sps (factor {}), requested {}",
                actual_rate, factor, target_rate
            );
        }
        Self {
            factor,
            count: 0,
            sum: 0.0,
            bits: 0,
            actual_rate,
        }
    }

    pub fn feed(&mut self, ua: f64, logic: u8) -> Option<(f64, u8)> {
        self.count += 1;
        self.sum += ua;
        self.bits |= logic;
        if self.count >= self.factor {
            let avg = self.sum / self.factor as f64;
            let bits = self.bits;
            self.count = 0;
            self.sum = 0.0;
            self.bits = 0;
            Some((avg, bits))
        } else {
            None
        }
    }

    pub fn flush(&self) -> Option<(f64, u8)> {
        if self.count > 0 && self.count < self.factor {
            Some((self.sum / self.count as f64, self.bits))
        } else {
            None
        }
    }

    pub fn actual_rate(&self) -> u32 {
        self.actual_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_10_averages_correctly() {
        let mut ds = Downsampler::new(10000);
        assert_eq!(ds.actual_rate(), 10000);
        let mut emitted = Vec::new();
        for i in 1..=10 {
            if let Some(v) = ds.feed(i as f64, 0) {
                emitted.push(v);
            }
        }
        assert_eq!(emitted.len(), 1);
        assert!((emitted[0].0 - 5.5).abs() < 0.001);
    }

    #[test]
    fn factor_1_is_pass_through() {
        let mut ds = Downsampler::new(100000);
        assert_eq!(ds.actual_rate(), 100000);
        let out = ds.feed(42.0, 0x03);
        assert!(out.is_some());
        let (ua, bits) = out.unwrap();
        assert!((ua - 42.0).abs() < 0.001);
        assert_eq!(bits, 0x03);
    }

    #[test]
    fn rate_1_is_max_downsample() {
        let ds = Downsampler::new(1);
        assert_eq!(ds.actual_rate(), 1);
    }

    #[test]
    fn rate_0_clamps_to_1() {
        let ds = Downsampler::new(0);
        assert_eq!(ds.actual_rate(), 1);
    }

    #[test]
    fn rate_over_max_clamps() {
        let ds = Downsampler::new(200000);
        assert_eq!(ds.actual_rate(), 100000);
    }

    #[test]
    fn non_divisible_rate_rounds() {
        let ds = Downsampler::new(3000);
        assert_eq!(ds.actual_rate(), 3030);
    }

    #[test]
    fn logic_bits_ored_across_group() {
        let mut ds = Downsampler::new(50000);
        assert!(ds.feed(1.0, 0x01).is_none());
        let out = ds.feed(2.0, 0x02);
        assert!(out.is_some());
        let (_, bits) = out.unwrap();
        assert_eq!(bits, 0x03);
    }

    #[test]
    fn flush_emits_partial_group() {
        let mut ds = Downsampler::new(25000);
        assert!(ds.feed(1.0, 0).is_none());
        assert!(ds.feed(2.0, 0).is_none());
        let out = ds.flush();
        assert!(out.is_some());
        let (ua, _) = out.unwrap();
        assert!((ua - 1.5).abs() < 0.001);
    }

    #[test]
    fn flush_on_full_group_is_none() {
        let mut ds = Downsampler::new(100000);
        let _ = ds.feed(1.0, 0);
        assert!(ds.flush().is_none());
    }
}
