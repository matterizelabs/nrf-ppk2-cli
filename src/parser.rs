use crate::types::Sample;

const MAX_GAP_BUFFER: usize = 4;

pub struct SampleParser {
    expected_counter: u8,
    lost_samples: u64,
    gap_buffer: Vec<Sample>,
    synced: bool,
}

impl SampleParser {
    pub fn new() -> Self {
        Self {
            expected_counter: 0,
            lost_samples: 0,
            gap_buffer: Vec::with_capacity(MAX_GAP_BUFFER),
            synced: false,
        }
    }

    pub fn feed(&mut self, raw: &[u32]) -> Vec<Sample> {
        let mut output = Vec::with_capacity(raw.len());

        for &word in raw {
            let sample = Sample::from_raw(word);

            if !self.synced {
                self.expected_counter = sample.counter.wrapping_add(1) & 0x3F;
                self.synced = true;
                output.push(sample);
                continue;
            }

            if sample.counter != self.expected_counter {
                // Gap detected
                let gap = if sample.counter > self.expected_counter {
                    (sample.counter - self.expected_counter) as u64
                } else {
                    (64 - self.expected_counter + sample.counter) as u64
                };
                self.lost_samples += gap;

                // Buffer bad frames; if too many, flush
                if self.gap_buffer.len() >= MAX_GAP_BUFFER {
                    self.gap_buffer.clear();
                    output.push(sample);
                    self.expected_counter = sample.counter.wrapping_add(1) & 0x3F;
                    continue;
                }
                self.gap_buffer.push(sample);
            } else {
                output.push(sample);
                self.expected_counter = self.expected_counter.wrapping_add(1) & 0x3F;
            }
        }

        output
    }

    pub fn lost_samples(&self) -> u64 {
        self.lost_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(counter: u8) -> u32 {
        // adc=0, range=0, logic=1, counter set
        (1u32 << 24) | ((counter as u32) << 18)
    }

    #[test]
    fn sequential_samples_no_gaps() {
        let mut parser = SampleParser::new();
        let raw = [make_sample(0), make_sample(1), make_sample(2)];
        let samples = parser.feed(&raw);
        assert_eq!(samples.len(), 3);
        assert_eq!(parser.lost_samples(), 0);
    }

    #[test]
    fn counter_gap_detected() {
        let mut parser = SampleParser::new();
        let raw = [make_sample(0), make_sample(5)];
        let samples = parser.feed(&raw);
        assert_eq!(samples.len(), 1); // first sample accepted
        assert!(parser.lost_samples() > 0);
    }

    #[test]
    fn counter_wrap_handled() {
        let mut parser = SampleParser::new();
        let raw = [make_sample(63), make_sample(0)];
        let samples = parser.feed(&raw);
        assert_eq!(samples.len(), 2);
        assert_eq!(parser.lost_samples(), 0);
    }

    #[test]
    fn multiple_bad_frames_buffered() {
        let mut parser = SampleParser::new();
        let raw = [
            make_sample(0),  // good, counter=0
            make_sample(10), // bad, counter=0→10 gap
            make_sample(11), // still bad
            make_sample(12), // still bad
            make_sample(13), // still bad
            make_sample(14), // accumulated 4 gap-buffered, now flush
        ];
        let samples = parser.feed(&raw);
        assert!(samples.len() >= 2); // at least first + flushed
    }
}
