use crate::{
    units::{Bytes, Nanosecs},
    FlowId,
};

#[derive(Debug)]
pub struct Record {
    pub id: FlowId,
    pub size: Bytes,
    pub start: Nanosecs,
    pub fct: Nanosecs,
    pub ideal: Nanosecs,
}
