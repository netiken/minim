//! Minim is a custom single-link simulator built for the Parsimon project. Given a
//! [bottleneck link configuration](Config), [congestion control parameters](Config)
//! (only DCTCP is currently supported), a list of [sources](SourceDesc), and a list of
//! [flows](FlowDesc), Minim will output a list of [records](Record) via the [run] function.

#![warn(unreachable_pub, missing_debug_implementations, missing_docs)]

#[macro_use]
mod ident;

pub mod time;
pub mod units;

pub(crate) mod data;
pub(crate) mod driver;
pub(crate) mod entities;
pub(crate) mod flow;
pub(crate) mod packet;
pub(crate) mod port;
pub(crate) mod simulation;

pub use data::Record;
pub use driver::{read_flows, run, Config, ConfigBuilder, ReadFlowsError};
pub use entities::source::{SourceDesc, SourceId};
pub use flow::{FlowDesc, FlowId};
pub use packet::Packet;
pub use port::QIndex;
