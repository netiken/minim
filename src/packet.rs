use typed_builder::TypedBuilder;

use crate::{
    entities::source::SourceId,
    units::{Bytes, Nanosecs},
    FlowId, time::Time,
};

/// A packet of data.
#[derive(Debug, Clone, Copy, TypedBuilder)]
pub struct Packet {
    pub(crate) flow_id: FlowId,
    pub(crate) source_id: SourceId,
    pub(crate) size: Bytes,
    pub(crate) src2btl: Nanosecs,
    pub(crate) btl2dst: Nanosecs,
    pub(crate) is_last: bool,
    
    // Packet tracing
    #[builder(default)]
    pub(crate) t_enq: Option<Time>,
    #[builder(default)]
    pub(crate) t_deq: Option<Time>,
}

// TODO: `SZ_MAX` and `SZ_HDR` should be configurable somehow
impl Packet {
    // /// The maximum packet size.
    // pub const SZ_MAX: Bytes = Bytes::new(1000);
    // /// The size of the packet header.
    // pub const SZ_HDR: Bytes = Bytes::new(48);

    pub(crate) fn hrtt(&self) -> Nanosecs {
        self.src2btl + self.btl2dst
    }

    pub(crate) fn min_count_in(size: Bytes, sz_pktmax: Bytes) -> usize {
        if size == Bytes::ZERO {
            0
        } else {
            size.into_usize() / sz_pktmax.into_usize() + 1
        }
    }
}

#[derive(Debug, Clone, Copy, derive_new::new)]
pub(crate) struct Ack {
    pub(crate) nr_bytes: Bytes,
    pub(crate) marked: bool,
}
