use std::collections::VecDeque;

use crate::packet::Packet;

#[derive(Debug, Default, derive_new::new)]
pub(crate) struct FifoQ {
    #[new(default)]
    inner: VecDeque<Packet>,
}

impl FifoQ {
    delegate::delegate! {
        to self.inner {
            #[call(push_back)]
            pub(crate) fn enqueue(&mut self, pkt: Packet);

            #[call(pop_front)]
            pub(crate) fn dequeue(&mut self) -> Option<Packet>;
        }
    }
}
