#![allow(clippy::non_canonical_partial_ord_impl)]

use smallvec::SmallVec;

use crate::time::Time;

use super::Command;

// Most handlers will not yield very many events
pub(crate) type EventList = SmallVec<[Event; 4]>;

#[derive(Debug)]
pub(crate) struct Event {
    time: Time,
    pub(crate) cmd: Command,
}

impl Event {
    pub(crate) fn new(time: Time, cmd: impl Into<Command>) -> Self {
        Self {
            time,
            cmd: cmd.into(),
        }
    }

    pub(crate) fn time(&self) -> Time {
        self.time
    }
}
