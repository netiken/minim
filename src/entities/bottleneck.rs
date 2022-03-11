use crate::{
    entities::flow::FlowCmd,
    packet::{Ack, Packet},
    queue::FifoQ,
    simulation::{event::EventList, Context, SimulatorCmd},
    units::{BitsPerSec, Bytes},
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub(crate) struct Bottleneck {
    #[builder(setter(into))]
    pub(crate) bandwidth: BitsPerSec,
    queue: FifoQ,
    #[builder(default, setter(skip))]
    status: Status,

    // DCTCP
    #[builder(default, setter(skip))]
    qsize: Bytes,
    #[builder(setter(into))]
    marking_threshold: Bytes,
}

impl Bottleneck {
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

impl Bottleneck {
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
                let delta = self.bandwidth.length(pkt.size).into_delta();
                ctx.schedule(delta, BottleneckCmd::new_step());
                // Signal the flow to increase its window
                let delay = (pkt.btl2dst + pkt.hrtt()).into_delta();
                let marked = self.qsize > self.marking_threshold;
                ctx.schedule(
                    delta + delay,
                    FlowCmd::new_rcv_ack(pkt.flow_id, Ack::new(pkt.size, marked)),
                );
                if pkt.is_last {
                    // A flow is defined to be departed when all of its bytes
                    // have been delivered to the destination.
                    ctx.schedule(
                        pkt.btl2dst.into_delta(),
                        SimulatorCmd::new_flow_depart(pkt.flow_id),
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
