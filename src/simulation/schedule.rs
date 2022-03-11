use std::collections::BinaryHeap;

use delegate::delegate;

use super::event::Event;

#[derive(Debug, Default)]
pub(crate) struct Schedule {
    inner: BinaryHeap<Event>,
}

impl Schedule {
    delegate! {
        to self.inner {
            pub(crate) fn push(&mut self, ev: Event);
            pub(crate) fn pop(&mut self) -> Option<Event>;
            pub(crate) fn is_empty(&self) -> bool;
        }
    }
}
