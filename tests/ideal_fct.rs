use minim::{
    queue::FifoQ,
    units::{Bytes, Gbps, Kilobytes, Mbps, Nanosecs, Secs},
    Config, FlowDesc, FlowId, Packet, SourceDesc, SourceId,
};

// Make sure FCTs match up for short flows and long flows. For long flows, there may be some minor
// rounding errors.
#[test]
fn ideal_fct() {
    let source = SourceDesc::builder()
        .id(SourceId::ZERO)
        .delay2btl(Nanosecs::new(1_000))
        .link_rate(Gbps::new(10))
        .build();
    let flows = vec![
        FlowDesc {
            id: FlowId::new(0),
            source: SourceId::ZERO,
            size: Bytes::new(100),
            start: Secs::new(1).into_ns(),
            delay2dst: Nanosecs::new(2_000),
        },
        FlowDesc {
            id: FlowId::new(1),
            source: SourceId::ZERO,
            size: Packet::SZ_MAX.scale_by(1_000.0),
            start: Secs::new(2).into_ns(),
            delay2dst: Nanosecs::new(2_000),
        },
    ];
    let cfg = Config::builder()
        .bandwidth(Gbps::new(40))
        .queue(FifoQ::new())
        .sources(vec![source])
        .flows(flows)
        .window(Kilobytes::new(100))
        .dctcp_marking_threshold(Kilobytes::new(300))
        .dctcp_gain(0.0625)
        .dctcp_ai(Mbps::new(615))
        .build();
    let records = minim::run(cfg);
    for record in records {
        assert_eq!(record.fct, record.ideal);
    }
}
