use std::collections::VecDeque;

use crate::{
    flow::FlowDesc,
    simulation::{event::EventList, Context},
};

use super::source::SourceCmd;

#[derive(Debug, Clone, derive_new::new)]
pub(crate) struct Workload {
    flows: VecDeque<FlowDesc>,
}

impl Workload {
    #[must_use]
    pub(crate) fn step(&mut self, mut ctx: Context) -> EventList {
        if let Some(flow) = self.flows.pop_front() {
            let delta = flow.start.into_time() - ctx.cur_time;
            ctx.schedule(delta, SourceCmd::new_flow_arrive(flow.source, flow));

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
