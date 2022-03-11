macro_rules! entity_id {
    ($name: ident) => {
        #[derive(
            Debug,
            Default,
            Copy,
            Clone,
            PartialOrd,
            Ord,
            PartialEq,
            Eq,
            Hash,
            derive_more::Display,
            derive_more::Add,
            derive_more::Sub,
            derive_more::AddAssign,
            derive_more::SubAssign,
            serde::Serialize,
            serde::Deserialize,
        )]
        pub struct $name(u64);

        impl $name {
            pub const ZERO: $name = Self::new(0);
            pub const ONE: $name = Self::new(1);
            pub const MAX: $name = Self::new(u64::MAX);

            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            pub fn from_usize(val: usize) -> Self {
                Self(val as u64)
            }

            pub fn into_usize(self) -> usize {
                self.0 as usize
            }
        }
    };
}

pub(crate) mod bottleneck;
pub(crate) mod flow;
pub(crate) mod workload;
