use crate::{
    entities::source::SourceCmd,
    packet::{Ack, Packet},
    port::Port,
    simulation::{event::EventList, Context},
    units::{BitsPerSec, Bytes},
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub(crate) struct Bottleneck {
    #[builder(setter(into))]
    pub(crate) bandwidth: BitsPerSec,
    port: Port,
    #[builder(default, setter(skip))]
    status: Status,

    #[builder(setter(into))]
    marking_threshold: Bytes,
}

impl Bottleneck {
    #[must_use]
    pub(crate) fn receive(&mut self, pkt: Packet, ctx: Context) -> EventList {
        // Enqueue the packet and update state
        self.port[pkt.qindex].enqueue(pkt);
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
        match self.port.pick_dequeue_index() {
            Some(qidx) => {
                let pkt = self.port[qidx].dequeue().expect("unexpected empty queue");
                // Service the packet
                let bw_delta = self.bandwidth.length(pkt.size).into_delta();
                ctx.schedule(bw_delta, BottleneckCmd::new_step());
                // Send an ACK back to the flow
                let prop_delta = (pkt.btl2dst + pkt.hrtt()).into_delta();
                let nr_bytes_to_ack = pkt.size - ctx.sz_pkthdr;
                let marked = self.port[qidx].size() > self.marking_threshold;
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
