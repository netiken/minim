use std::path::Path;

use rustc_hash::FxHashMap;

use crate::{
    entities::{bottleneck::Bottleneck, source::Source, workload::Workload},
    queue::QDisc,
    simulation::Simulation,
    units::{BitsPerSec, Bytes, Nanosecs},
    FlowDesc, Record, SourceDesc,
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Config<Q: QDisc> {
    #[builder(setter(into))]
    bandwidth: BitsPerSec,
    queue: Q,
    sources: Vec<SourceDesc>,
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
    let sources = cfg
        .sources
        .into_iter()
        .map(|s| {
            let source = Source::builder()
                .id(s.id)
                .delay2btl(s.delay2btl)
                .link_rate(s.link_rate)
                .build();
            (s.id, source)
        })
        .collect::<FxHashMap<_, _>>();
    let bottleneck = Bottleneck::builder()
        .bandwidth(cfg.bandwidth)
        .queue(cfg.queue)
        .marking_threshold(cfg.dctcp_marking_threshold)
        .build();
    let sim = Simulation::builder()
        .workload(workload)
        .sources(sources)
        .bottleneck(bottleneck)
        .window(cfg.window)
        .dctcp_gain(cfg.dctcp_gain)
        .dctcp_ai(cfg.dctcp_ai)
        .timeout(cfg.timeout.map(|v| v.into_time()))
        .build();
    sim.run()
}

pub fn read_flows(path: impl AsRef<Path>) -> Result<Vec<FlowDesc>, Error> {
    let s = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&s)?)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("serde error")]
    Serde(#[from] serde_json::Error),

    #[error("IO error")]
    Io(#[from] std::io::Error),
}
