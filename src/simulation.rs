pub(crate) mod event;
mod schedule;

use rustc_hash::FxHashMap;

use crate::{
    data::Record,
    entities::{
        bottleneck::{Bottleneck, BottleneckCmd},
        source::{Source, SourceCmd, SourceId},
        workload::{Workload, WorkloadCmd},
    },
    queue::QDisc,
    time::{Delta, Time},
    units::{BitsPerSec, Bytes},
};

use self::{
    event::{Event, EventList},
    schedule::Schedule,
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub(crate) struct Simulation<Q: QDisc> {
    // Run-time
    #[builder(default, setter(skip))]
    cur_time: Time,
    #[builder(default, setter(skip))]
    schedule: Schedule,

    // Entities
    workload: Workload,
    sources: FxHashMap<SourceId, Source>,
    bottleneck: Bottleneck<Q>,

    // Rate control configuration
    #[builder(setter(into))]
    window: Bytes,
    dctcp_gain: f64,
    #[builder(setter(into))]
    dctcp_ai: BitsPerSec,

    // Used for termination
    timeout: Option<Time>,
}

impl<Q: QDisc> Simulation<Q> {
    pub(crate) fn run(mut self) -> Vec<Record> {
        // Kick off the simulation by starting the workload
        let ev = Event::new(Time::ZERO, WorkloadCmd::new_step());
        self.schedule.push(ev);
        // Run the simulation
        while !self.should_stop() {
            self.step();
        }
        // Return the FCT records
        self.finish()
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
            btl_bandwidth: self.bottleneck.bandwidth,
            window: self.window,
            dctcp_gain: self.dctcp_gain,
            dctcp_ai: self.dctcp_ai,
        }
    }

    fn finish(self) -> Vec<Record> {
        self.sources
            .into_iter()
            .flat_map(|(_, source)| source.records.into_iter())
            .collect()
    }
}

// Command handlers
impl<Q: QDisc> Simulation<Q> {
    fn apply(&mut self, cmd: Command) -> EventList {
        match cmd {
            Command::Workload(cmd) => self.apply_workload(cmd),
            Command::Source(cmd) => self.apply_source(cmd),
            Command::Bottleneck(cmd) => self.apply_bottleneck(cmd),
            Command::Test => unreachable!(),
        }
    }

    fn apply_workload(&mut self, cmd: WorkloadCmd) -> EventList {
        let ctx = self.context();
        match cmd {
            WorkloadCmd::Step => self.workload.step(ctx),
        }
    }

    fn apply_source(&mut self, cmd: SourceCmd) -> EventList {
        let ctx = self.context();
        match cmd {
            SourceCmd::TrySend { id, version } => {
                let source = self.sources.get_mut(&id).expect("invalid source ID");
                source.try_send(version, ctx)
            }
            SourceCmd::RcvAck { source, flow, ack } => {
                let source = self.sources.get_mut(&source).expect("invalid source ID");
                source.rcv_ack(flow, ack, ctx)
            }
            SourceCmd::FlowArrive { source, desc } => {
                let source = self.sources.get_mut(&source).expect("invalid source ID");
                source.flow_arrive(desc, ctx)
            }
            SourceCmd::FlowDepart { source, flow } => {
                let source = self.sources.get_mut(&source).expect("invalid source ID");
                source.flow_depart(flow, ctx)
            }
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
    Workload(WorkloadCmd),
    Source(SourceCmd),
    Bottleneck(BottleneckCmd),
    Test,
}

#[derive(Debug)]
pub(crate) struct Context {
    pub(crate) cur_time: Time,
    events: EventList,

    // Configuration
    pub(crate) btl_bandwidth: BitsPerSec,
    pub(crate) window: Bytes,
    pub(crate) dctcp_gain: f64,
    pub(crate) dctcp_ai: BitsPerSec,
}

impl Context {
    pub(crate) fn schedule(&mut self, delta: Delta, cmd: impl Into<Command>) {
        let time = self.cur_time + delta;
        self.events.push(Event::new(time, cmd.into()));
    }

    #[allow(unused)]
    pub(crate) fn schedule_now(&mut self, cmd: impl Into<Command>) {
        self.schedule(Delta::ZERO, cmd);
    }

    pub(crate) fn into_events(self) -> EventList {
        self.events
    }
}
