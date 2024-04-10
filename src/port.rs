use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
};

use crate::{packet::Packet, units::Bytes};

#[derive(Debug, Clone)]
pub(crate) struct Port {
    queues: Vec<Queue>,
    quanta: Vec<Bytes>,
    deficits: Vec<Bytes>,
    counter: usize,
    should_bump: bool,
}

impl Port {
    pub(crate) fn new(quanta: &[Bytes]) -> Self {
        let nr_queues = quanta.len();
        Self {
            queues: (0..nr_queues).map(|_| Queue::default()).collect(),
            quanta: Vec::from(quanta),
            deficits: vec![Bytes::ZERO; nr_queues],
            counter: 0,
            should_bump: true,
        }
    }

    // PRECONDITION: All quanta must be nonzero
    // This routine returns `None` iff all queues are empty. Otherwise, queue indices are returned
    // in deficit round-robin order according to the configured quanta.
    #[must_use]
    pub(crate) fn pick_dequeue_index(&mut self) -> Option<QIndex> {
        let n = self.queues.len();
        let start = self.counter;
        loop {
            if self.counter - start == n {
                // All queues are empty
                return None;
            }
            let idx = self.counter % n;
            if self.queues[idx].is_empty() {
                self.deficits[idx] = Bytes::ZERO;
                self.counter += 1;
                self.should_bump = true;
            } else {
                break;
            }
        }
        // The previous step guarantees that there exists a nonempty queue at this point. Since we
        // assume all quanta are nonzero and positive, we are guaranteed to find some queue that
        // has accumulated enough deficit to send.
        loop {
            let idx = self.counter % n;
            if self.queues[idx].is_empty() {
                self.deficits[idx] = Bytes::ZERO;
                self.counter += 1;
                self.should_bump = true;
                continue;
            }
            // `self.queues[idx]` is now guaranteed to be nonempty
            if self.should_bump {
                self.deficits[idx] += self.quanta[idx];
                self.should_bump = false;
            }
            let cost = self.queues[idx].peek().unwrap().size;
            if self.deficits[idx] >= cost {
                self.deficits[idx] -= cost;
                break Some(QIndex::new(idx));
            } else {
                self.counter += 1;
                self.should_bump = true;
                continue;
            }
        }
    }
}

/// A queue index for a port.
///
/// Each queue has its own scheduling weight.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct QIndex(usize);

impl QIndex {
    /// Queue index zero.
    pub const ZERO: QIndex = QIndex::new(0);
    /// Queue index one.
    pub const ONE: QIndex = QIndex::new(1);

    /// Create a new queue index.
    pub const fn new(val: usize) -> Self {
        Self(val)
    }

    /// Get the inner value of the queue index.
    pub const fn inner(&self) -> usize {
        self.0
    }
}

impl Index<QIndex> for Port {
    type Output = Queue;

    fn index(&self, index: QIndex) -> &Self::Output {
        &self.queues[index.inner()]
    }
}

impl IndexMut<QIndex> for Port {
    fn index_mut(&mut self, index: QIndex) -> &mut Self::Output {
        &mut self.queues[index.inner()]
    }
}

#[derive(Debug, Default, Clone, derive_new::new)]
pub(crate) struct Queue {
    inner: VecDeque<Packet>,
    qsize: Bytes,
}

impl Queue {
    pub(crate) fn enqueue(&mut self, pkt: Packet) {
        self.qsize += pkt.size;
        self.inner.push_back(pkt);
    }

    pub(crate) fn dequeue(&mut self) -> Option<Packet> {
        match self.inner.pop_front() {
            r @ Some(pkt) => {
                self.qsize -= pkt.size;
                r
            }
            None => None,
        }
    }

    pub(crate) fn size(&self) -> Bytes {
        self.qsize
    }

    delegate::delegate! {
        to self.inner {
            #[call(front)]
            pub(crate) fn peek(&self) -> Option<&Packet>;
            pub(crate) fn is_empty(&self) -> bool;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::FlowId;

    use super::*;

    fn mk_pkt(flow_id: FlowId, qindex: QIndex, size: Bytes) -> Packet {
        Packet {
            flow_id,
            qindex,
            size,
            ..Packet::default()
        }
    }

    fn check_drr_sequence(port: &mut Port, sequence: &[usize]) {
        for &idx in sequence {
            let expected = QIndex::new(idx);
            let actual = port.pick_dequeue_index().expect("all queues empty");
            // We have to call dequeue every time we call `pick_dequeue_index`
            assert_eq!(port[actual].dequeue().unwrap().qindex, expected);
        }
    }

    #[test]
    fn drr_empty_none() -> anyhow::Result<()> {
        let mut port = Port::new(&[Bytes::new(1); 8]);
        assert!(port.pick_dequeue_index().is_none());
        Ok(())
    }

    #[test]
    fn drr_nonempty_some() -> anyhow::Result<()> {
        let mut port = Port::new(&[Bytes::new(1); 8]);
        let pkt = mk_pkt(FlowId::ZERO, QIndex::ZERO, Bytes::new(1_000));
        port[pkt.qindex].enqueue(pkt);
        assert_eq!(port.pick_dequeue_index(), Some(QIndex::ZERO));
        Ok(())
    }

    #[test]
    fn drr_empty_resets_deficit() -> anyhow::Result<()> {
        let mut port = Port::new(&[Bytes::new(1); 2]);

        // One packet in queue 0
        let pkt = mk_pkt(FlowId::ZERO, QIndex::ZERO, Bytes::new(1_000));
        port[pkt.qindex].enqueue(pkt);

        // 20 packets in queue 1
        for _ in 0..20 {
            let pkt = mk_pkt(FlowId::ONE, QIndex::ONE, Bytes::new(1_000));
            port[QIndex::ONE].enqueue(pkt);
        }

        // First dequeue should be from queue 0, after which its deficit should be reset
        check_drr_sequence(&mut port, &[0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

        // 10 more packets in queue 0
        for _ in 0..10 {
            let pkt = mk_pkt(FlowId::ZERO, QIndex::ZERO, Bytes::new(1_000));
            port[pkt.qindex].enqueue(pkt);
        }

        // Queue 0 should not have accumulated deficit while it was empty
        check_drr_sequence(
            &mut port,
            &[0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1],
        );

        Ok(())
    }

    #[test]
    fn drr_respects_weights() -> anyhow::Result<()> {
        let mut port = Port::new(&[Bytes::new(1), Bytes::new(3)]);

        let pkt1 = mk_pkt(FlowId::ZERO, QIndex::ZERO, Bytes::ONE);
        let pkt2 = mk_pkt(FlowId::ONE, QIndex::ONE, Bytes::ONE);
        for _ in 0..6 {
            port[pkt1.qindex].enqueue(pkt1);
            port[pkt2.qindex].enqueue(pkt2);
        }

        check_drr_sequence(&mut port, &[0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0]);
        assert!(port.pick_dequeue_index().is_none());
        Ok(())
    }
}
