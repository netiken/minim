pub(crate) mod event;
mod schedule;

use std::cmp;

use rustc_hash::FxHashMap;

use crate::{
    data::Record,
    entities::{
        bottleneck::{Bottleneck, BottleneckCmd},
        flow::{Flow, FlowCmd, FlowId},
        workload::{Workload, WorkloadCmd},
    },
    packet::Packet,
    time::{Delta, Time},
    units::{BitsPerSec, Bytes},
};

use self::{
    event::{Event, EventList},
    schedule::Schedule,
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub(crate) struct Simulation {
    // Run-time
    #[builder(default, setter(skip))]
    cur_time: Time,
    #[builder(default, setter(skip))]
    schedule: Schedule,

    // Entities
    workload: Workload,
    #[builder(default, setter(skip))]
    flows: FxHashMap<FlowId, Flow>,
    bottleneck: Bottleneck,

    // Rate control configuration
    #[builder(setter(into))]
    window: Bytes,
    dctcp_gain: f64,
    #[builder(setter(into))]
    dctcp_ai: BitsPerSec,

    // Used for termination
    timeout: Option<Time>,

    // Data accumulator
    #[builder(default, setter(skip))]
    records: Vec<Record>,
}

impl Simulation {
    pub(crate) fn run(mut self) -> Vec<Record> {
        // Kick off the simulation by starting the workload
        let ev = Event::new(Time::ZERO, WorkloadCmd::new_step());
        self.schedule.push(ev);
        // Run the simulation
        while !self.should_stop() {
            self.step();
        }
        // Return the FCT records
        self.records
    }

    fn step(&mut self) {
        let next = self.schedule.pop().unwrap();

        let (time, cmd) = (next.time(), next.cmd);
        assert!(self.cur_time <= time);
        self.cur_time = time;

        let events = self.apply(cmd);
        for ev in events.into_iter() {
            self.schedule.push(ev);
        }
    }

    fn should_stop(&self) -> bool {
        self.schedule.is_empty() || self.cur_time > self.timeout.unwrap_or(Time::MAX)
    }

    fn context(&self) -> Context {
        Context {
            cur_time: self.cur_time,
            events: EventList::new(),
            window: self.window,
            dctcp_gain: self.dctcp_gain,
            dctcp_ai: self.dctcp_ai,
        }
    }

    fn flow_mut(&mut self, id: FlowId) -> Option<&mut Flow> {
        self.flows.get_mut(&id)
    }
}

// Command handlers
impl Simulation {
    fn apply(&mut self, cmd: Command) -> EventList {
        match cmd {
            Command::Simulator(cmd) => self.apply_simulator(cmd),
            Command::Workload(cmd) => self.apply_workload(cmd),
            Command::Flow(cmd) => self.apply_flow(cmd),
            Command::Bottleneck(cmd) => self.apply_bottleneck(cmd),
            Command::Test => unreachable!(),
        }
    }

    fn apply_simulator(&mut self, cmd: SimulatorCmd) -> EventList {
        let mut ctx = self.context();
        match cmd {
            SimulatorCmd::FlowArrive(flow) => {
                // Schedule the flow immediately
                self.flows.insert(flow.id, flow);
                ctx.schedule_now(FlowCmd::new_step(flow.id, u128::default()));
            }
            SimulatorCmd::FlowDepart(fid) => {
                if let Some(flow) = self.flows.remove(&fid) {
                    // Compute the ideal FCT
                    let bw_hop1 = flow.max_rate;
                    let bw_hop2 = self.bottleneck.bandwidth;
                    let sz_head = cmp::min(Packet::SZ_MAX, flow.size);
                    let sz_rest = flow.size - sz_head;
                    let ideal = bw_hop1.length(sz_head)
                        + bw_hop2.length(sz_head)
                        + cmp::min(bw_hop1, bw_hop2).length(sz_rest)
                        + flow.src2btl
                        + flow.btl2dst;
                    // Store the flow's FCT record
                    let record = Record {
                        id: flow.id,
                        size: flow.size,
                        start: flow.start,
                        fct: self.cur_time.into_ns() - flow.start,
                        ideal,
                    };
                    self.records.push(record);
                }
            }
        }
        ctx.into_events()
    }

    fn apply_workload(&mut self, cmd: WorkloadCmd) -> EventList {
        let ctx = self.context();
        match cmd {
            WorkloadCmd::Step => self.workload.step(ctx),
        }
    }

    fn apply_flow(&mut self, cmd: FlowCmd) -> EventList {
        let ctx = self.context();
        match cmd {
            FlowCmd::Step { id, version } => match self.flow_mut(id) {
                Some(flow) => flow.step(version, ctx),
                None => ctx.into_events(),
            },
            FlowCmd::RcvAck { id, ack } => match self.flow_mut(id) {
                Some(flow) => flow.rcv_ack(ack, ctx),
                None => ctx.into_events(),
            },
        }
    }

    fn apply_bottleneck(&mut self, cmd: BottleneckCmd) -> EventList {
        let ctx = self.context();
        match cmd {
            BottleneckCmd::Receive(pkt) => self.bottleneck.receive(pkt, ctx),
            BottleneckCmd::Step => self.bottleneck.step(ctx),
        }
    }
}

#[derive(Debug, Clone, derive_more::From)]
pub(crate) enum Command {
    Simulator(SimulatorCmd),
    Workload(WorkloadCmd),
    Flow(FlowCmd),
    Bottleneck(BottleneckCmd),
    Test,
}

#[derive(Debug, Clone, Copy, derive_new::new)]
pub(crate) enum SimulatorCmd {
    FlowArrive(Flow),
    FlowDepart(FlowId),
}

#[derive(Debug)]
pub(crate) struct Context {
    pub(crate) cur_time: Time,
    events: EventList,

    // Configuration---threaded through `self.workload` to start new flows
    pub(crate) window: Bytes,
    pub(crate) dctcp_gain: f64,
    pub(crate) dctcp_ai: BitsPerSec,
}

impl Context {
    pub(crate) fn schedule(&mut self, delta: Delta, cmd: impl Into<Command>) {
        let time = self.cur_time + delta;
        self.events.push(Event::new(time, cmd.into()));
    }

    pub(crate) fn schedule_now(&mut self, cmd: impl Into<Command>) {
        self.schedule(Delta::ZERO, cmd);
    }

    pub(crate) fn into_events(self) -> EventList {
        self.events
    }
}
