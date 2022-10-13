use std::cmp;

use rustc_hash::FxHashMap;

use crate::{
    flow::{Flow, FlowDesc},
    packet::Ack,
    simulation::{event::EventList, Context},
    time::Time,
    units::{BitsPerSec, Bytes, Nanosecs},
    FlowId, Packet, Record,
};

use super::bottleneck::BottleneckCmd;

identifier!(SourceId);

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub(crate) struct Source {
    pub(crate) id: SourceId,
    #[builder(setter(into))]
    pub(crate) delay2btl: Nanosecs,

    #[builder(setter(into))]
    link_rate: BitsPerSec,
    #[builder(default, setter(skip))]
    earliest_tnext: Time, // the earliest allowable wake-up time under the link rate
    #[builder(default=Time::MAX, setter(skip))]
    tnext: Time, // the actual time this source will wake up, or Time::MAX

    #[builder(default, setter(skip))]
    flow_queue: FlowQ,
    #[builder(default, setter(skip))]
    flow_info: FxHashMap<FlowId, FlowInfo>,

    #[builder(default, setter(skip))]
    version: u128,

    #[builder(default, setter(skip))]
    pub(crate) records: Vec<Record>,
}

impl Source {
    #[must_use]
    pub(crate) fn try_send(&mut self, version: u128, mut ctx: Context) -> EventList {
        if version != self.version {
            return ctx.into_events();
        }
        match self.flow_queue.next_packet(ctx.cur_time) {
            FlowQResult::Found { pkt } => {
                // Send the packet to the bottleneck
                let bw_delta = self.link_rate.length(pkt.size).into_delta();
                ctx.schedule(
                    self.delay2btl.into_delta() + bw_delta,
                    BottleneckCmd::new_receive(pkt),
                );
                // Reschedule the source to send again
                ctx.schedule(bw_delta, SourceCmd::new_try_send(self.id, self.version));
                self.earliest_tnext = ctx.cur_time + bw_delta;
                self.tnext = ctx.cur_time + bw_delta;
            }
            FlowQResult::RateBound { tnext } => {
                let delta = tnext - ctx.cur_time;
                ctx.schedule(delta, SourceCmd::new_try_send(self.id, self.version));
                self.tnext = ctx.cur_time + delta;
            }
            FlowQResult::WinBound | FlowQResult::Empty => {
                self.tnext = Time::MAX;
            }
        }
        ctx.into_events()
    }

    #[must_use]
    pub(crate) fn rcv_ack(&mut self, flow_id: FlowId, ack: Ack, mut ctx: Context) -> EventList {
        if let Some(flow) = self.flow_queue.get_flow_mut(flow_id) {
            flow.rcv_ack(ack);
            if !flow.is_win_bound() && flow.tnext < self.tnext {
                let tnext = cmp::max(self.earliest_tnext, flow.tnext);
                self.version += 1;
                ctx.schedule(
                    tnext.saturating_sub(ctx.cur_time),
                    SourceCmd::new_try_send(self.id, self.version),
                );
                self.tnext = tnext;
            }
        }
        ctx.into_events()
    }

    #[must_use]
    pub(crate) fn flow_arrive(&mut self, desc: FlowDesc, ctx: Context) -> EventList {
        let btl2dst = desc.delay2dst - self.delay2btl;
        let info = FlowInfo {
            id: desc.id,
            size: desc.size,
            start: desc.start,
            src2btl: self.delay2btl,
            btl2dst: desc.delay2dst - self.delay2btl,
            max_rate: self.link_rate,
        };
        self.flow_info.insert(info.id, info);
        let flow = Flow::builder()
            .id(desc.id)
            .source(desc.source)
            .size(desc.size)
            .rate(self.link_rate)
            .max_rate(self.link_rate)
            .tnext(ctx.cur_time)
            .src2btl(self.delay2btl)
            .btl2dst(btl2dst)
            .window(ctx.window)
            .gain(ctx.dctcp_gain)
            .additive_inc(ctx.dctcp_ai)
            .build();
        self.flow_queue.add_flow(flow);
        if self.earliest_tnext <= ctx.cur_time && ctx.cur_time < self.tnext {
            self.version += 1;
            self.try_send(self.version, ctx)
        } else {
            ctx.into_events()
        }
    }

    #[must_use]
    pub(crate) fn flow_depart(&mut self, flow_id: FlowId, ctx: Context) -> EventList {
        let flow = self
            .flow_info
            .remove(&flow_id)
            .expect("missing flow record");
        // Compute the ideal FCT
        let bw_hop1 = flow.max_rate;
        let bw_hop2 = ctx.btl_bandwidth;
        let bw_min = cmp::min(bw_hop1, bw_hop2);
        let sz_head_ = cmp::min(Packet::SZ_MAX, flow.size);
        let sz_head = (sz_head_ != Bytes::ZERO)
            .then(|| sz_head_ + Packet::SZ_HDR)
            .unwrap_or(Bytes::ZERO);
        let sz_rest_ = flow.size - sz_head_;
        let head_delay = bw_hop1.length(sz_head) + bw_hop2.length(sz_head);
        let rest_delay = {
            let nr_full_pkts = sz_rest_.into_usize() / Packet::SZ_MAX.into_usize();
            let sz_full_pkt = Packet::SZ_MAX + Packet::SZ_HDR;
            let sz_partial_pkt_ = Bytes::new(sz_rest_.into_u64() % Packet::SZ_MAX.into_u64());
            let sz_partial_pkt = (sz_partial_pkt_ != Bytes::ZERO)
                .then(|| sz_partial_pkt_ + Packet::SZ_HDR)
                .unwrap_or(Bytes::ZERO);
            bw_min.length(sz_full_pkt).scale_by(nr_full_pkts as f64) + bw_min.length(sz_partial_pkt)
        };
        let prop_delay = flow.src2btl + flow.btl2dst;
        let ideal = head_delay + rest_delay + prop_delay;

        // Store the flow's FCT record
        let record = Record {
            id: flow.id,
            size: flow.size,
            start: flow.start,
            fct: ctx.cur_time.into_ns() - flow.start,
            ideal,
        };
        self.records.push(record);
        ctx.into_events()
    }
}

#[derive(Debug, Copy, Clone, derive_new::new)]
pub(crate) enum SourceCmd {
    TrySend {
        id: SourceId,
        version: u128,
    },
    RcvAck {
        source: SourceId,
        flow: FlowId,
        ack: Ack,
    },
    FlowArrive {
        source: SourceId,
        desc: FlowDesc,
    },
    FlowDepart {
        source: SourceId,
        flow: FlowId,
    },
}

#[derive(Debug, Default, Clone, derive_new::new)]
struct FlowQ {
    #[new(default)]
    members: FxHashMap<FlowId, Flow>,

    #[new(default)]
    order: Vec<FlowId>,
    rr_next: usize,
}

impl FlowQ {
    fn next_packet(&mut self, now: Time) -> FlowQResult {
        if self.order.is_empty() {
            return FlowQResult::Empty;
        }
        let mut min_viable_tnext = None;
        let nr_flows = self.order.len();
        for i in 0..nr_flows {
            let idx = (i + self.rr_next) % nr_flows;
            let id = self.order[idx];
            let flow = self.members.get_mut(&id).unwrap();
            match (flow.is_rate_bound(now), flow.is_win_bound()) {
                (false, false) => {
                    // This flow can send, so there's nothing left to do but update the order.
                    let pkt = flow.next_packet(now);
                    let id = flow.id;
                    if flow.bytes_left() == Bytes::ZERO {
                        self.order.remove(idx);
                        self.members.remove(&id);
                    }
                    self.rr_next = idx + 1;
                    return FlowQResult::Found { pkt };
                }
                (true, false) => {
                    // This flow can definitely send later because it isn't window-bound, and it
                    // won't become window-bound unless it sends again, since all the ACKs that
                    // come in will only increase the window.
                    min_viable_tnext =
                        Some(cmp::min(flow.tnext, min_viable_tnext.unwrap_or(Time::MAX)));
                }
                // The flow is window-bound, so it isn't a candidate for scheduling
                _ => continue,
            }
        }
        match min_viable_tnext {
            Some(tnext) => {
                assert!(tnext > now); // otherwise, we would've sent it
                FlowQResult::RateBound { tnext }
            }
            None => FlowQResult::WinBound,
        }
    }

    fn add_flow(&mut self, flow: Flow) {
        let id = flow.id;
        assert!(!self.members.contains_key(&id));
        self.order.push(id);
        self.members.insert(id, flow);
    }

    fn get_flow_mut(&mut self, flow_id: FlowId) -> Option<&mut Flow> {
        self.members.get_mut(&flow_id)
    }
}

#[derive(Debug)]
enum FlowQResult {
    // The next packet to send
    Found { pkt: Packet },
    // Rate-bound, but not window-bound
    RateBound { tnext: Time },
    // Window-bound
    WinBound,
    // No flows in the flow queue
    Empty,
}

#[derive(Debug, Clone, Copy)]
struct FlowInfo {
    id: FlowId,
    size: Bytes,
    start: Nanosecs,
    src2btl: Nanosecs,
    btl2dst: Nanosecs,
    max_rate: BitsPerSec,
}

/// A source configuration.
#[derive(Debug, Clone, Copy, typed_builder::TypedBuilder, serde::Serialize, serde::Deserialize)]
pub struct SourceDesc {
    /// The source ID.
    pub id: SourceId,
    /// The propagation delay from the source to the bottleneck link.
    #[builder(setter(into))]
    pub delay2btl: Nanosecs,
    /// The rate of the link connecting the source to the bottleneck.
    #[builder(setter(into))]
    pub link_rate: BitsPerSec,
}
