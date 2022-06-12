use std::cmp::Ordering;

use crate::{
    units::{Bytes, Nanosecs},
    FlowId,
};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub id: FlowId,
    pub size: Bytes,
    pub start: Nanosecs,
    pub fct: Nanosecs,
    pub ideal: Nanosecs,
}

impl Record {
    pub fn delay(&self) -> Nanosecs {
        // Some of these cases are possible because of rounding errors
        match self.fct.cmp(&self.ideal) {
            Ordering::Less | Ordering::Equal => Nanosecs::ZERO,
            Ordering::Greater => self.fct - self.ideal,
        }
    }
}
