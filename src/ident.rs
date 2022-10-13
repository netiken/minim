macro_rules! identifier {
    ($name: ident) => {
        #[allow(missing_docs)]
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
            /// Equivalent to Self::new(0).
            pub const ZERO: $name = Self::new(0);
            /// Equivalent to Self::new(1).
            pub const ONE: $name = Self::new(1);
            /// Equivalent to Self::new(usize::MAX).
            pub const MAX: $name = Self::new(usize::MAX);

            /// Create a new ID.
            pub const fn new(value: usize) -> Self {
                Self(value)
            }

            /// Convert the ID into a `usize`.
            pub fn into_usize(self) -> usize {
                self.0 as usize
            }
        }
    };
}
