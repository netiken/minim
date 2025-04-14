use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;

use minim::units::{Bytes, Gbps, Kilobytes, Mbps, Nanosecs};
use minim::{run, Config, FlowDesc, FlowId, QIndex, Record, SourceDesc, SourceId};

fn create_flows(num_flows: usize, source_id: SourceId) -> Vec<FlowDesc> {
    let mut rng = StdRng::seed_from_u64(42); // Deterministic seed
    let mut flows = Vec::with_capacity(num_flows);

    // Target 60% utilization on a 10 Gbps link over 10 seconds
    // 10 Gbps * 0.6 * 10 seconds = 60 Gb = 7.5 GB
    let total_bytes = 7_500_000_000; // 7.5 GB in bytes
    let bytes_per_flow = total_bytes / num_flows as u64;

    // Simulation duration - 10 seconds
    let sim_duration_ns = 10_000_000_000;

    for i in 0..num_flows {
        // Flow size varies around the average bytes per flow (u00b150%)
        let size_variation = bytes_per_flow / 2;
        let min_size = bytes_per_flow.saturating_sub(size_variation);
        let max_size = bytes_per_flow.saturating_add(size_variation);
        let size = Bytes::new(rng.gen_range(min_size..=max_size));

        // Start time distributed over the simulation duration (to maintain 60% average load)
        let start = Nanosecs::new(rng.gen_range(0..sim_duration_ns));

        // Create flow
        let flow = FlowDesc {
            id: FlowId::new(i),
            source: source_id,
            qindex: QIndex::new(0), // Only one traffic class
            size,
            start,
            delay2dst: Nanosecs::new(100_000), // 100 microseconds end-to-end delay
        };

        flows.push(flow);
    }

    flows
}

fn run_simulation(num_flows: usize) -> Vec<Record> {
    // Create source
    let source_id = SourceId::new(0);
    let source = SourceDesc::builder()
        .id(source_id)
        .delay2btl(Nanosecs::new(50_000)) // 50 microseconds to bottleneck
        .link_rate(Gbps::new(10).into_bps()) // 10 Gbps link rate
        .build();

    // Create flows
    let flows = create_flows(num_flows, source_id);

    // Create config (using parameters from MinimLink)
    let config = Config::builder()
        .bandwidth(Gbps::new(10).into_bps()) // 10 Gbps bottleneck bandwidth
        .sources(vec![source])
        .flows(flows)
        .quanta(vec![Bytes::new(1024)])
        .windows(vec![Bytes::new(10_000)])
        .dctcp_marking_thresholds(vec![Kilobytes::new(30)])
        .dctcp_gain(0.0625)
        .dctcp_ai(Mbps::new(615).into_bps())
        .sz_pktmax(Bytes::new(1000))
        .sz_pkthdr(Bytes::new(40)) // TCP/IP header size
        .build();

    // Run simulation
    run(config).expect("Failed to run simulation")
}

// This function will be used by Criterion for benchmarking
fn benchmark_simulation(c: &mut Criterion) {
    // Benchmark with 10,000 flows (for speed in CI)
    c.bench_function("simulation_10k", |b| {
        b.iter(|| run_simulation(black_box(10_000)))
    });
}

fn config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group! {
    name = benches;
    config = config();
    targets = benchmark_simulation
}
criterion_main!(benches);
