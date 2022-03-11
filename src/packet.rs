use typed_builder::TypedBuilder;

use crate::{
    entities::flow::FlowId,
    units::{Bytes, Nanosecs},
};

#[derive(Debug, Clone, Copy, TypedBuilder)]
pub(crate) struct Packet {
    pub(crate) flow_id: FlowId,
    pub(crate) size: Bytes,
    pub(crate) src2btl: Nanosecs,
    pub(crate) btl2dst: Nanosecs,
    pub(crate) is_last: bool,
}

impl Packet {
    pub(crate) const SZ_MIN: Bytes = Bytes::new(64);
    pub(crate) const SZ_MAX: Bytes = Bytes::new(9000);

    pub(crate) fn hrtt(&self) -> Nanosecs {
        self.src2btl + self.btl2dst
    }

    pub(crate) fn max_count_in(size: Bytes) -> usize {
        size.into_usize() / Self::SZ_MAX.into_usize() + 1
    }
}

#[derive(Debug, Clone, Copy, derive_new::new)]
pub(crate) struct Ack {
    pub(crate) nr_bytes: Bytes,
    pub(crate) marked: bool,
}
