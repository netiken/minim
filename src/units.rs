use crate::time::{Delta, Time};

macro_rules! unit {
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
            derive_more::Sub,
            derive_more::AddAssign,
            derive_more::SubAssign,
            derive_more::Sum,
            derive_more::Display,
            derive_more::FromStr,
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

            pub const fn into_u64(self) -> u64 {
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
                Self(inner.round() as u64)
            }

            pub const fn checked_div(self, rhs: u64) -> Option<Self> {
                if rhs == 0 {
                    None
                } else {
                    Some(Self::new(self.0 / rhs))
                }
            }

            pub const fn saturating_sub(self, rhs: Self) -> Self {
                Self::new(self.0.saturating_sub(rhs.0))
            }
        }
    };
}

unit!(Nanosecs);
unit!(Microsecs);
unit!(Millisecs);
unit!(Secs);

impl Nanosecs {
    pub fn into_time(self) -> Time {
        Time::new(u128::from(self.0))
    }

    pub fn into_delta(self) -> Delta {
        Delta::new(u128::from(self.0))
    }
}

impl Microsecs {
    pub const fn into_ns(self) -> Nanosecs {
        Nanosecs::new(self.0 * 1_000)
    }

    pub fn into_time(self) -> Time {
        self.into_ns().into_time()
    }

    pub fn into_delta(self) -> Delta {
        self.into_ns().into_delta()
    }
}

impl Millisecs {
    pub const fn into_us(self) -> Microsecs {
        Microsecs::new(self.0 * 1_000)
    }

    pub fn into_time(self) -> Time {
        self.into_us().into_time()
    }

    pub fn into_delta(self) -> Delta {
        self.into_us().into_delta()
    }
}

impl Secs {
    pub const fn into_ms(self) -> Millisecs {
        Millisecs::new(self.0 * 1_000)
    }

    pub fn into_time(self) -> Time {
        self.into_ms().into_time()
    }

    pub fn into_delta(self) -> Delta {
        self.into_ms().into_delta()
    }
}

impl From<Nanosecs> for Time {
    fn from(ns: Nanosecs) -> Self {
        ns.into_time()
    }
}

impl From<Microsecs> for Time {
    fn from(us: Microsecs) -> Self {
        us.into_time()
    }
}

impl From<Millisecs> for Time {
    fn from(ms: Millisecs) -> Self {
        ms.into_time()
    }
}

impl From<Secs> for Time {
    fn from(s: Secs) -> Self {
        s.into_time()
    }
}

unit!(Bits);
unit!(Bytes);
unit!(Kilobytes);

impl Bytes {
    pub fn into_bits(self) -> Bits {
        Bits::new(self.0 * 8)
    }
}

impl Kilobytes {
    pub fn into_bytes(self) -> Bytes {
        Bytes::new(self.0 * 1_000)
    }

    pub fn into_bits(self) -> Bits {
        self.into_bytes().into_bits()
    }
}

impl From<Bytes> for Bits {
    fn from(val: Bytes) -> Self {
        val.into_bits()
    }
}

impl From<Kilobytes> for Bytes {
    fn from(val: Kilobytes) -> Self {
        val.into_bytes()
    }
}

unit!(BitsPerSec);
unit!(Mbps);
unit!(Gbps);

impl BitsPerSec {
    pub fn into_gbps(self) -> Gbps {
        let val = self.0 as f64 / 1_000_000_000_f64;
        Gbps::new(val.round() as u64)
    }

    #[allow(non_snake_case)]
    pub fn length(&self, size: Bytes) -> Nanosecs {
        assert!(*self != BitsPerSec::ZERO);
        if size == Bytes::ZERO {
            return Nanosecs::ZERO;
        }
        let bytes = size.into_f64();
        let bps = self.into_f64();
        let delta = (bytes * 1e9 * 8.0) / bps;
        let delta = delta.round() as u64;
        Nanosecs::new(delta)
    }

    #[allow(non_snake_case)]
    pub fn width(&self, delta: Nanosecs) -> Bytes {
        if delta == Nanosecs::ZERO {
            return Bytes::ZERO;
        }
        let delta = delta.into_f64();
        let bps = self.into_f64();
        let size = (bps * delta) / (1e9 * 8.0);
        let size = size.round() as u64;
        Bytes::new(size)
    }
}

impl Gbps {
    pub const fn into_bps(self) -> BitsPerSec {
        let val = self.0 * 1_000_000_000;
        BitsPerSec::new(val)
    }

    pub const fn into_mbps(self) -> Mbps {
        let val = self.0 * 1_000;
        Mbps::new(val)
    }

    pub fn length(&self, size: Bytes) -> Nanosecs {
        self.into_bps().length(size)
    }

    pub fn width(&self, delta: Nanosecs) -> Bytes {
        self.into_bps().width(delta)
    }
}

impl From<Gbps> for BitsPerSec {
    fn from(val: Gbps) -> Self {
        val.into_bps()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_length() {
        let rate = Gbps::new(100);
        let size = Bytes::new(64);
        assert_eq!(rate.length(size), Nanosecs::new(5));
    }

    #[test]
    fn rate_width() {
        let rate = Gbps::new(100);
        let delta = Nanosecs::new(5);
        assert_eq!(rate.width(delta), Bytes::new(63));
    }
}
