//! Rate limit for progress reports: long jobs report after every item, but
//! the UI only needs an update every so often.

use std::time::{Duration, Instant};

/// Says when at least `every` has passed since it last said so.
#[derive(Debug)]
pub struct Throttle {
    every: Duration,
    last: Instant,
}

impl Throttle {
    /// A throttle that is first due `every` after `start`.
    #[must_use]
    pub fn new(every: Duration, start: Instant) -> Self {
        Self { every, last: start }
    }

    /// Whether `every` has passed since the start or the last `true`.
    pub fn is_due(&mut self, now: Instant) -> bool {
        let due = now.duration_since(self.last) >= self.every;
        if due {
            self.last = now;
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_throttle_is_due_once_per_period() {
        let start = Instant::now();
        let mut throttle = Throttle::new(Duration::from_secs(10), start);

        assert!(!throttle.is_due(start + Duration::from_secs(9)));
        assert!(throttle.is_due(start + Duration::from_secs(10)));
        assert!(!throttle.is_due(start + Duration::from_secs(19)));
        assert!(throttle.is_due(start + Duration::from_secs(21)));
    }
}
