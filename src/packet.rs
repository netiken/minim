use typed_builder::TypedBuilder;

use crate::{
    entities::source::SourceId,
    port::QIndex,
    units::{Bytes, Nanosecs},
    FlowId,
};

/// A packet of data.
#[derive(Debug, Default, Clone, Copy, TypedBuilder)]
pub struct Packet {
    pub(crate) flow_id: FlowId,
    pub(crate) source_id: SourceId,
    pub(crate) qindex: QIndex,
    pub(crate) size: Bytes,
    pub(crate) src2btl: Nanosecs,
    pub(crate) btl2dst: Nanosecs,
    pub(crate) is_last: bool,
}

impl Packet {
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
