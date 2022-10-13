use crate::{
    entities::source::SourceCmd,
    packet::{Ack, Packet},
    queue::QDisc,
    simulation::{event::EventList, Context},
    units::{BitsPerSec, Bytes},
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub(crate) struct Bottleneck<Q: QDisc> {
    #[builder(setter(into))]
    pub(crate) bandwidth: BitsPerSec,
    queue: Q,
    #[builder(default, setter(skip))]
    status: Status,

    // DCTCP
    #[builder(default, setter(skip))]
    qsize: Bytes,
    #[builder(setter(into))]
    marking_threshold: Bytes,
}

impl<Q: QDisc> Bottleneck<Q> {
    fn enqueue(&mut self, pkt: Packet) {
        self.queue.enqueue(pkt);
        self.qsize += pkt.size;
    }

    fn dequeue(&mut self) -> Option<Packet> {
        match self.queue.dequeue() {
            Some(pkt) => {
                self.qsize -= pkt.size;
                Some(pkt)
            }
            None => None,
        }
    }
}

impl<Q: QDisc> Bottleneck<Q> {
    #[must_use]
    pub(crate) fn receive(&mut self, pkt: Packet, ctx: Context) -> EventList {
        // Enqueue the packet and update state
        self.enqueue(pkt);
        match self.status {
            Status::Running => ctx.into_events(),
            Status::Blocked => {
                self.status = Status::new_running();
                self.step(ctx)
            }
        }
    }

    #[must_use]
    pub(crate) fn step(&mut self, mut ctx: Context) -> EventList {
        assert!(self.status == Status::Running);
        match self.dequeue() {
            Some(pkt) => {
                // Service the packet
                let bw_delta = self.bandwidth.length(pkt.size).into_delta();
                ctx.schedule(bw_delta, BottleneckCmd::new_step());
                // Send an ACK back to the flow
                let prop_delta = (pkt.btl2dst + pkt.hrtt()).into_delta();
                let nr_bytes_to_ack = pkt.size - ctx.sz_pkthdr;
                let marked = self.qsize > self.marking_threshold;
                ctx.schedule(
                    bw_delta + prop_delta,
                    SourceCmd::new_rcv_ack(
                        pkt.source_id,
                        pkt.flow_id,
                        Ack::new(nr_bytes_to_ack, marked),
                    ),
                );
                if pkt.is_last {
                    // A flow is defined to be departed when all of its bytes
                    // have been delivered to the destination.
                    ctx.schedule(
                        bw_delta + pkt.btl2dst.into_delta(),
                        SourceCmd::new_flow_depart(pkt.source_id, pkt.flow_id),
                    );
                }
            }
            None => {
                self.status = Status::new_blocked();
            }
        }
        ctx.into_events()
    }
}

#[derive(Debug, Clone, derive_new::new)]
pub(crate) enum BottleneckCmd {
    Receive(Packet),
    Step,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_new::new, derivative::Derivative)]
#[derivative(Default)]
enum Status {
    Running,
    #[derivative(Default)]
    Blocked,
}
