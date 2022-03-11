use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::units::Nanosecs;

macro_rules! time_unit {
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
            derive_more::Add,
            derive_more::Display,
            derive_more::FromStr,
            serde::Serialize,
            serde::Deserialize,
        )]
        pub struct $name(u128);

        impl $name {
            pub const ZERO: $name = Self::new(0);
            pub const ONE: $name = Self::new(1);
            pub const MAX: $name = Self::new(u128::MAX);

            pub const fn new(value: u128) -> Self {
                Self(value)
            }

            pub const fn into_u128(self) -> u128 {
                self.0
            }

            pub const fn into_f64(self) -> f64 {
                self.0 as f64
            }

            pub const fn into_usize(self) -> usize {
                self.0 as usize
            }

            pub fn scale_by(self, val: f64) -> Self {
                let inner = self.0 as f64 * val;
                Self(inner.round() as u128)
            }
        }
    };
}

time_unit!(Time);

impl Time {
    pub const fn into_delta(self) -> Delta {
        Delta::new(self.0)
    }

    pub fn into_nanos(self) -> Nanosecs {
        assert!(self.0 <= u128::from(u64::MAX));
        Nanosecs::new(self.0 as u64)
    }
}

time_unit!(Delta);

impl Delta {
    pub const fn into_time(self) -> Time {
        Time::new(self.0)
    }

    pub fn into_nanos(self) -> Nanosecs {
        assert!(self.0 <= u128::from(u64::MAX));
        Nanosecs::new(self.0 as u64)
    }
}

impl From<u128> for Time {
    fn from(val: u128) -> Self {
        Self(val)
    }
}

impl Add<Delta> for Time {
    type Output = Time;

    fn add(self, rhs: Delta) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Delta> for Time {
    type Output = Time;

    fn sub(self, rhs: Delta) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Sub<Time> for Time {
    type Output = Delta;

    fn sub(self, rhs: Time) -> Self::Output {
        Delta::new(self.0 - rhs.0)
    }
}

impl AddAssign<Delta> for Time {
    fn add_assign(&mut self, rhs: Delta) {
        *self = Self(self.0 + rhs.0)
    }
}

impl SubAssign<Delta> for Time {
    fn sub_assign(&mut self, rhs: Delta) {
        *self = Self(self.0 - rhs.0)
    }
}
