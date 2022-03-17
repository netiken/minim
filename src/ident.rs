macro_rules! identifier {
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
        pub struct $name(usize);

        impl $name {
            pub const ZERO: $name = Self::new(0);
            pub const ONE: $name = Self::new(1);
            pub const MAX: $name = Self::new(usize::MAX);

            pub const fn new(value: usize) -> Self {
                Self(value)
            }

            pub fn from_usize(val: usize) -> Self {
                Self(val)
            }

            pub fn into_usize(self) -> usize {
                self.0 as usize
            }
        }
    };
}
