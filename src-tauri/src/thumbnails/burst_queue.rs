//! The order in which queued thumbnails are rendered: the newest burst of
//! requests first, each burst from its oldest request on.
//!
//! The webview requests the thumbnails of one render (a page just opened, or
//! where a scroll stopped) top to bottom within a few dozen milliseconds,
//! visible rows before the rows rendered ahead below them. Bursts further
//! back are photos already scrolled past.

use std::time::{Duration, Instant};

/// Requests closer together than this belong to the same burst. Measured on
/// 2026-10-29: the rows of one render come 20–50 ms apart, separate scroll
/// steps at least 360 ms apart.
pub const BURST_GAP: Duration = Duration::from_millis(100);

/// A burst reaches back at most this far from its newest request, so a long
/// continuous scroll does not chain back to the photos it scrolled past.
pub const BURST_SPAN: Duration = Duration::from_millis(300);

/// Queued items with the time each was queued.
#[derive(Debug)]
pub struct BurstQueue<T> {
    items: Vec<(Instant, T)>,
}

impl<T> Default for BurstQueue<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> BurstQueue<T> {
    /// Queues `item`, queued at `at`; times never go backwards.
    pub fn push(&mut self, item: T, at: Instant) {
        self.items.push((at, item));
    }

    /// Takes the oldest item of the newest burst, if any is queued.
    pub fn pop(&mut self) -> Option<T> {
        let start = self.newest_burst_start()?;
        Some(self.items.remove(start).1)
    }

    /// Index of the first item of the newest burst.
    fn newest_burst_start(&self) -> Option<usize> {
        let (newest, _) = self.items.last()?;
        let earliest = newest.checked_sub(BURST_SPAN);
        let start = self
            .items
            .windows(2)
            .rposition(|pair| match pair {
                [(before, _), (after, _)] => {
                    after.duration_since(*before) >= BURST_GAP
                        || earliest.is_some_and(|earliest| *before < earliest)
                }
                _ => false,
            })
            .map_or(0, |gap| gap + 1);
        Some(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    fn drain(queue: &mut BurstQueue<u32>) -> Vec<u32> {
        std::iter::from_fn(|| queue.pop()).collect()
    }

    #[test]
    fn one_burst_is_served_in_the_order_it_was_requested() {
        let start = Instant::now();
        let mut queue = BurstQueue::default();
        // Three rows of one render, 25 ms apart.
        for (n, at) in [(1, 0), (2, 1), (3, 25), (4, 26), (5, 50)] {
            queue.push(n, ms(start, at));
        }

        assert_eq!(drain(&mut queue), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn the_newest_burst_comes_before_older_ones() {
        let start = Instant::now();
        let mut queue = BurstQueue::default();
        for (n, at) in [(1, 0), (2, 10), (3, 400), (4, 410), (5, 800), (6, 810)] {
            queue.push(n, ms(start, at));
        }

        assert_eq!(drain(&mut queue), [5, 6, 3, 4, 1, 2]);
    }

    #[test]
    fn a_long_continuous_burst_starts_at_its_last_300_ms() {
        let start = Instant::now();
        let mut queue = BurstQueue::default();
        // Rows every 50 ms for a second, as while scrolling without a pause.
        for n in 0..=20 {
            queue.push(n, ms(start, u64::from(n) * 50));
        }

        let first: Vec<u32> = std::iter::from_fn(|| queue.pop()).take(7).collect();

        assert_eq!(first, [14, 15, 16, 17, 18, 19, 20]);
    }

    #[test]
    fn items_queued_while_serving_a_burst_come_first() {
        let start = Instant::now();
        let mut queue = BurstQueue::default();
        queue.push(1, ms(start, 0));
        queue.push(2, ms(start, 10));
        assert_eq!(queue.pop(), Some(1));

        queue.push(3, ms(start, 500));

        assert_eq!(drain(&mut queue), [3, 2]);
    }

    #[test]
    fn an_empty_queue_has_nothing_to_serve() {
        assert_eq!(BurstQueue::<u32>::default().pop(), None);
    }
}
