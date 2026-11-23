//! "One at a time" that can't stick: a send or a news check marks itself
//! busy until it ends (also when it fails or panics), and a mark older than
//! the limit counts as gone, so one hung task can't block every later one
//! until NGM Desktop restarts.

use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Busy(Mutex<Option<Instant>>);

/// Held while the task runs; dropping it clears the mark.
pub struct Held<'a>(&'a Busy);

impl Busy {
    /// The mark, unless something younger than `stale` holds it.
    pub fn take(&self, stale: Duration) -> Option<Held<'_>> {
        let mut at = self.0.lock().unwrap();
        if matches!(*at, Some(t) if t.elapsed() < stale) {
            return None;
        }
        *at = Some(Instant::now());
        Some(Held(self))
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        *self.0 .0.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_at_a_time_until_dropped() {
        let b = Busy::default();
        let first = b.take(Duration::from_secs(60));
        assert!(first.is_some());
        assert!(b.take(Duration::from_secs(60)).is_none());
        drop(first);
        assert!(b.take(Duration::from_secs(60)).is_some());
    }

    #[test]
    fn a_stale_mark_counts_as_gone() {
        let b = Busy::default();
        let stuck = b.take(Duration::from_secs(60));
        std::mem::forget(stuck); // a task that never ends
        std::thread::sleep(Duration::from_millis(20));
        assert!(b.take(Duration::from_millis(10)).is_some());
    }
}
