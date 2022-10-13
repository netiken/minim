use std::cmp::Ordering;

use crate::{
    units::{Bytes, Nanosecs},
    FlowId,
};

/// An flow completion time record.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Record {
    /// The flow ID.
    pub id: FlowId,
    /// The flow size.
    pub size: Bytes,
    /// The start time of the flow.
    pub start: Nanosecs,
    /// The flow completion time. A flow is complete when all bytes have been delivered to the
    /// destination.
    pub fct: Nanosecs,
    /// The ideal flow completion time in an unloaded simulation.
    pub ideal: Nanosecs,
}

impl Record {
    /// Computes the delay experienced by the corresponding flow at the bottleneck link, defined as
    /// the measured FCT minus the ideal FCT.
    pub fn delay(&self) -> Nanosecs {
        // Some of these cases are possible because of rounding errors
        match self.fct.cmp(&self.ideal) {
            Ordering::Less | Ordering::Equal => Nanosecs::ZERO,
            Ordering::Greater => self.fct - self.ideal,
        }
    }
}
