#![allow(clippy::non_canonical_partial_ord_impl)]

use std::cmp::Reverse;

use smallvec::SmallVec;

use crate::time::Time;

use super::Command;

// Most handlers will not yield very many events
pub(crate) type EventList = SmallVec<[Event; 4]>;

#[derive(Debug, derivative::Derivative)]
#[derivative(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Event {
    time: Reverse<Time>,
    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    pub(crate) cmd: Command,
}

impl Event {
    pub(crate) fn new(time: Time, cmd: impl Into<Command>) -> Self {
        Self {
            time: Reverse(time),
            cmd: cmd.into(),
        }
    }

    pub(crate) fn time(&self) -> Time {
        self.time.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_order() {
        let e1 = Event::new(Time::ZERO, Command::Test);
        let e2 = Event::new(Time::ONE, Command::Test);
        assert!(e1 > e2);
    }
}
