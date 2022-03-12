use std::collections::VecDeque;

use rustc_hash::FxHashMap;

use crate::{packet::Packet, FlowId};

pub trait QDisc {
    fn enqueue(&mut self, pkt: Packet);
    fn dequeue(&mut self) -> Option<Packet>;
    fn is_empty(&self) -> bool;
}

#[derive(Debug, Default, derive_new::new)]
pub struct FifoQ {
    #[new(default)]
    inner: VecDeque<Packet>,
}

impl QDisc for FifoQ {
    delegate::delegate! {
        to self.inner {
            #[call(push_back)]
            fn enqueue(&mut self, pkt: Packet);

            #[call(pop_front)]
            fn dequeue(&mut self) -> Option<Packet>;

            fn is_empty(&self) -> bool;
        }
    }
}

/// This queue implements round-robin among flows, where each flow is
/// represented by a `VecDeque` of `Packet`s.
#[derive(Debug, Default, derive_new::new)]
pub struct RrQ {
    #[new(default)]
    members: FxHashMap<FlowId, VecDeque<Packet>>,
    #[new(default)]
    order: VecDeque<FlowId>,
}

/// Outside these functions, the `RrQueue` should _never_ contain an empty
/// `VecDeque`. That way, if `dequeue` returns `None`, we can be certain that
/// _no_ flows have any packets to send.
impl QDisc for RrQ {
    fn enqueue(&mut self, pkt: Packet) {
        let flow_id = pkt.flow_id;
        if !self.members.contains_key(&flow_id) {
            self.order.push_back(flow_id);
            self.members.insert(flow_id, VecDeque::new());
        }
        self.members.get_mut(&flow_id).unwrap().push_back(pkt);
    }

    fn dequeue(&mut self) -> Option<Packet> {
        // Move the first class ID, if any, to the back
        if !self.order.is_empty() {
            self.order.rotate_left(1);
        }

        // Get that flow's first packet and other relevant data, if any
        let res = self
            .order
            .back()
            .copied()
            .map(|id| (id, self.members.get_mut(&id).unwrap()))
            .and_then(|(id, queue)| queue.pop_front().map(|pkt| (pkt, id, queue.is_empty())));

        // Delete the flow if removing a packet caused it to become empty
        if let Some((_, id, true)) = res {
            self.order.pop_back();
            self.members.remove(&id);
        }

        res.map(|(pkt, _, _)| pkt)
    }

    delegate::delegate! {
        to self.order {
            fn is_empty(&self) -> bool;
        }
    }
}
