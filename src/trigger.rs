use std::collections::VecDeque;

const SAMPLES_PER_SECOND: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerEdge {
    Rising,
    Falling,
    Both,
}

#[derive(Debug, Clone)]
pub struct TriggerConfig {
    pub threshold_ua: f64,
    pub edge: TriggerEdge,
    pub pre_trigger_ms: u64,
    pub post_trigger_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerState {
    Armed,
    #[allow(dead_code)]
    Fired,
    Collecting,
    Done,
}

pub struct TriggerEngine {
    config: TriggerConfig,
    state: TriggerState,
    pre_trigger: VecDeque<(f64, u8)>,
    captured: Vec<(f64, u8)>,
    prev_above: Option<bool>,
    post_samples_remaining: u64,
    sample_count: u64,
    fired_at: Option<u64>,
}

impl TriggerEngine {
    pub fn new(config: TriggerConfig) -> Self {
        let pre_samples =
            (config.pre_trigger_ms as f64 / 1000.0 * SAMPLES_PER_SECOND as f64) as usize;
        Self {
            config,
            state: TriggerState::Armed,
            pre_trigger: VecDeque::with_capacity(pre_samples.max(1)),
            captured: Vec::new(),
            prev_above: None,
            post_samples_remaining: 0,
            sample_count: 0,
            fired_at: None,
        }
    }

    pub fn feed(&mut self, ua: f64, logic: u8) {
        if self.state == TriggerState::Done {
            return;
        }

        self.sample_count += 1;

        let pre_cap =
            (self.config.pre_trigger_ms as f64 / 1000.0 * SAMPLES_PER_SECOND as f64) as usize;
        let pre_cap = pre_cap.max(1);

        match self.state {
            TriggerState::Armed => {
                let above = ua >= self.config.threshold_ua;

                if let Some(prev) = self.prev_above {
                    let edge_triggered = match self.config.edge {
                        TriggerEdge::Rising => !prev && above,
                        TriggerEdge::Falling => prev && !above,
                        TriggerEdge::Both => prev != above,
                    };
                    if edge_triggered {
                        self.fired_at = Some(self.sample_count);
                        self.captured = self.pre_trigger.iter().copied().collect();
                        self.post_samples_remaining = (self.config.post_trigger_ms as f64 / 1000.0
                            * SAMPLES_PER_SECOND as f64)
                            as u64;
                        self.captured.push((ua, logic));
                        self.pre_trigger.clear();
                        self.prev_above = Some(above);

                        if self.post_samples_remaining > 0 {
                            self.state = TriggerState::Collecting;
                        } else {
                            self.state = TriggerState::Done;
                        }
                        return;
                    }
                }

                self.prev_above = Some(above);
                while self.pre_trigger.len() >= pre_cap {
                    self.pre_trigger.pop_front();
                }
                self.pre_trigger.push_back((ua, logic));
            }
            TriggerState::Fired => {
                self.state = TriggerState::Done;
            }
            TriggerState::Collecting => {
                self.captured.push((ua, logic));
                self.post_samples_remaining -= 1;
                if self.post_samples_remaining == 0 {
                    self.state = TriggerState::Done;
                }
            }
            TriggerState::Done => {}
        }
    }

    pub fn state(&self) -> TriggerState {
        self.state
    }

    pub fn captured(&self) -> &[(f64, u8)] {
        &self.captured
    }

    pub fn fired_at(&self) -> Option<u64> {
        self.fired_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rising_edge_triggers_when_crossing_threshold() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Rising,
            pre_trigger_ms: 10,
            post_trigger_ms: 0,
        };
        let mut engine = TriggerEngine::new(config);

        engine.feed(500.0, 0);
        assert_eq!(engine.state(), TriggerState::Armed);

        engine.feed(1200.0, 0);
        assert_eq!(engine.state(), TriggerState::Done);
    }

    #[test]
    fn falling_edge_triggers_when_dropping_below_threshold() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Falling,
            pre_trigger_ms: 10,
            post_trigger_ms: 0,
        };
        let mut engine = TriggerEngine::new(config);

        engine.feed(1200.0, 0);
        assert_eq!(engine.state(), TriggerState::Armed);

        engine.feed(500.0, 0);
        assert_eq!(engine.state(), TriggerState::Done);
    }

    #[test]
    fn both_edge_triggers_on_any_crossing() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Both,
            pre_trigger_ms: 10,
            post_trigger_ms: 0,
        };
        let mut engine = TriggerEngine::new(config);

        engine.feed(500.0, 0);
        assert_eq!(engine.state(), TriggerState::Armed);

        engine.feed(1200.0, 0);
        assert_eq!(engine.state(), TriggerState::Done);
    }

    #[test]
    fn no_trigger_without_crossing() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Rising,
            pre_trigger_ms: 10,
            post_trigger_ms: 10,
        };
        let mut engine = TriggerEngine::new(config);

        for _ in 0..100 {
            engine.feed(500.0, 0);
        }
        assert_eq!(engine.state(), TriggerState::Armed);
    }

    #[test]
    fn no_trigger_when_staying_above() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Rising,
            pre_trigger_ms: 10,
            post_trigger_ms: 0,
        };
        let mut engine = TriggerEngine::new(config);

        engine.feed(1200.0, 0);
        engine.feed(1300.0, 0);
        engine.feed(1400.0, 0);
        assert_eq!(engine.state(), TriggerState::Armed);
    }

    #[test]
    fn pre_trigger_buffer_captured() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Rising,
            pre_trigger_ms: 10,
            post_trigger_ms: 0,
        };
        let mut engine = TriggerEngine::new(config);

        for i in 0..500 {
            engine.feed(i as f64, 0);
        }
        engine.feed(500.0, 0);
        engine.feed(1200.0, 0);

        assert_eq!(engine.state(), TriggerState::Done);
        let captured = engine.captured();
        assert!(!captured.is_empty());
        assert!(captured.last().unwrap().0 >= 1000.0);
    }

    #[test]
    fn post_trigger_samples_collected() {
        let config = TriggerConfig {
            threshold_ua: 1000.0,
            edge: TriggerEdge::Rising,
            pre_trigger_ms: 0,
            post_trigger_ms: 10,
        };
        let mut engine = TriggerEngine::new(config);

        engine.feed(500.0, 0);
        engine.feed(1200.0, 0);
        assert_eq!(engine.state(), TriggerState::Collecting);

        for _ in 0..2000 {
            if engine.state() == TriggerState::Done {
                break;
            }
            engine.feed(800.0, 0);
        }
        assert_eq!(engine.state(), TriggerState::Done);
        let captured = engine.captured();
        assert!(captured.len() > 1);
    }
}
