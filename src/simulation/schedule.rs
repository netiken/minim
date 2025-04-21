//! Event schedule backed by a radix heap.
//!
//! *Invariant:* once an event with timestamp `t_last` has been popped,
//! every future pushed event must satisfy `timestamp ≥ t_last`.
//! (The assert in `push` catches violations in debug builds.)

use super::event::Event;
use radix_heap::RadixHeapMap;

const FLIP_MASK: u64 = u64::MAX;

/// Convert a real timestamp into its inverted “radix key”.
#[inline(always)]
fn key(time: u64) -> u64 {
    FLIP_MASK.wrapping_sub(time)
}

#[derive(Debug)]
pub(crate) struct Schedule {
    /// Timestamp of the most recently popped event.
    now: u64,
    /// Radix heap buckets keyed by timestamp.
    inner: RadixHeapMap<u64, Event>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            now: 0,
            inner: RadixHeapMap::new(),
        }
    }
}

impl Schedule {
    /// Insert a future event.
    /// Panics in debug if the event is scheduled **before** the current time.
    #[inline]
    pub(crate) fn push(&mut self, ev: Event) {
        debug_assert!(
            ev.time().into_u64() >= self.now,
            "attempted to schedule an event in the past ({} < {})",
            ev.time(),
            self.now
        );
        // Key: timestamp, Value: the event itself
        self.inner.push(key(ev.time().into_u64()), ev);
    }

    /// Pop the next event (smallest timestamp) and advance the clock.
    #[inline]
    pub(crate) fn pop(&mut self) -> Option<Event> {
        self.inner.pop().map(|(_, ev)| {
            self.now = ev.time().into_u64();
            ev
        })
    }

    /// Is the schedule empty?
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::{simulation::Command, time::Time};

    use super::*;

    #[test]
    fn event_order() {
        let e1 = Event::new(Time::ZERO, Command::Test);
        let e2 = Event::new(Time::ONE, Command::Test);
        let mut schedule = Schedule::default();
        schedule.push(e1);
        schedule.push(e2);
        assert_eq!(schedule.pop().unwrap().time(), Time::ZERO);
        assert_eq!(schedule.pop().unwrap().time(), Time::ONE);
        assert!(schedule.is_empty());
    }
}
