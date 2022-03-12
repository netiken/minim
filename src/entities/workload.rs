use std::collections::VecDeque;

use crate::{
    simulation::{event::EventList, Context, SimulatorCmd},
    units::{BitsPerSec, Bytes, Nanosecs},
};

use super::flow::{Flow, FlowId};

#[derive(Debug, Clone, derive_new::new)]
pub(crate) struct Workload {
    flows: VecDeque<FlowDesc>,
}

impl Workload {
    #[must_use]
    pub(crate) fn step(&mut self, mut ctx: Context) -> EventList {
        if let Some(flow) = self.flows.pop_front() {
            let flow = Flow::builder()
                .id(flow.id)
                .size(flow.size)
                .start(flow.start)
                .rate(flow.max_rate)
                .max_rate(flow.max_rate)
                .src2btl(flow.src2btl)
                .btl2dst(flow.btl2dst)
                .window(ctx.window)
                .gain(ctx.dctcp_gain)
                .additive_inc(ctx.dctcp_ai)
                .build();
            let delta = flow.start.into_time() - ctx.cur_time;
            ctx.schedule(delta, SimulatorCmd::new_flow_arrive(flow));

            // Reschedule the next flow arrival
            if let Some(&FlowDesc {
                start: next_start, ..
            }) = self.flows.front()
            {
                let delta = next_start.into_time() - ctx.cur_time;
                ctx.schedule(delta, WorkloadCmd::new_step());
            }
        }
        ctx.into_events()
    }
}

#[derive(Debug, Copy, Clone, derive_new::new)]
pub(crate) enum WorkloadCmd {
    Step,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowDesc {
    pub id: FlowId,
    pub size: Bytes,
    pub start: Nanosecs,
    pub max_rate: BitsPerSec,
    pub src2btl: Nanosecs, // propagation delay to bottleneck
    pub btl2dst: Nanosecs, // propagation delay to destination
}
