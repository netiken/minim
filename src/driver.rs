use crate::{
    entities::{
        bottleneck::Bottleneck,
        workload::{FlowDesc, Workload},
    },
    queue::QDisc,
    simulation::Simulation,
    units::{BitsPerSec, Bytes, Nanosecs},
    Record,
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Config<Q: QDisc> {
    #[builder(setter(into))]
    bandwidth: BitsPerSec,
    queue: Q,
    flows: Vec<FlowDesc>,

    // Rate control configuration
    #[builder(setter(into))]
    window: Bytes,
    #[builder(setter(into))]
    dctcp_marking_threshold: Bytes,
    dctcp_gain: f64,
    #[builder(setter(into))]
    dctcp_ai: BitsPerSec,

    #[builder(default, setter(into, strip_option))]
    timeout: Option<Nanosecs>,
}

pub fn run<Q: QDisc>(cfg: Config<Q>) -> Vec<Record> {
    let workload = Workload::new(cfg.flows.into());
    let bottleneck = Bottleneck::builder()
        .bandwidth(cfg.bandwidth)
        .queue(cfg.queue)
        .marking_threshold(cfg.dctcp_marking_threshold)
        .build();
    let sim = Simulation::builder()
        .workload(workload)
        .bottleneck(bottleneck)
        .window(cfg.window)
        .dctcp_gain(cfg.dctcp_gain)
        .dctcp_ai(cfg.dctcp_ai)
        .timeout(cfg.timeout.map(|v| v.into_time()))
        .build();
    sim.run()
}
