use std::path::Path;

use rustc_hash::FxHashMap;

use crate::{
    entities::{bottleneck::Bottleneck, source::Source, workload::Workload},
    queue::QDisc,
    simulation::Simulation,
    units::{BitsPerSec, Bytes, Nanosecs},
    FlowDesc, Record, SourceDesc,
};

/// A simulation configuration.
#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Config<Q: QDisc> {
    /// The bottleneck bandwidth.
    #[builder(setter(into))]
    pub bandwidth: BitsPerSec,
    /// The bottleneck queue.
    pub queue: Q,
    /// The list of sources.
    pub sources: Vec<SourceDesc>,
    /// The list of flows.
    pub flows: Vec<FlowDesc>,

    /// The sending window.
    #[builder(setter(into))]
    pub window: Bytes,
    /// The DCTCP marking threshold.
    #[builder(setter(into))]
    pub dctcp_marking_threshold: Bytes,
    /// The DCTCP gain.
    pub dctcp_gain: f64,
    /// The DCTCP additive increase.
    #[builder(setter(into))]
    pub dctcp_ai: BitsPerSec,

    /// The simulation timeout, if any.
    #[builder(default, setter(into, strip_option))]
    pub timeout: Option<Nanosecs>,
}

/// Runs the simulation specified by `cfg` and returns a list of [records](Record).
pub fn run<Q: QDisc>(mut cfg: Config<Q>) -> Vec<Record> {
    cfg.flows.sort_by_key(|f| f.start);
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

/// Reads a list of [flows](FlowDesc) from `path`.
pub fn read_flows(path: impl AsRef<Path>) -> Result<Vec<FlowDesc>, ReadFlowsError> {
    let s = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&s)?)
}

/// The error type returned by [read_flows].
#[derive(Debug, thiserror::Error)]
pub enum ReadFlowsError {
    /// Serialization/deserialization error.
    #[error("serde error")]
    Serde(#[from] serde_json::Error),

    /// IO error.
    #[error("IO error")]
    Io(#[from] std::io::Error),
}
