use std::time::Duration;

use eframe::emath;

/// Custom wrapper around emath::Numeric that also includes Duration
pub trait Numeric: Clone + Copy + PartialEq + PartialOrd + 'static {
    /// Is this an integer type?
    const INTEGRAL: bool;

    /// Smallest finite value
    const MIN: Self;

    /// Largest finite value
    const MAX: Self;

    fn to_f64(self) -> f64;

    fn from_f64(num: f64) -> Self;
}

impl Numeric for Duration {
    const INTEGRAL: bool = false;

    const MIN: Self = Duration::ZERO;

    const MAX: Self = Duration::MAX;

    fn to_f64(self) -> f64 {
        self.as_secs_f64()
    }

    fn from_f64(num: f64) -> Self {
        Duration::from_secs_f64(num)
    }
}

macro_rules! impl_from_emath {
    ($t: ident) => {
        impl Numeric for $t {
            const INTEGRAL: bool = <Self as emath::Numeric>::INTEGRAL;
            const MIN: Self = <Self as emath::Numeric>::MIN;
            const MAX: Self = <Self as emath::Numeric>::MAX;

            #[inline]
            fn to_f64(self) -> f64 {
                <Self as emath::Numeric>::to_f64(self)
            }

            #[inline]
            fn from_f64(num: f64) -> Self {
                <Self as emath::Numeric>::from_f64(num)
            }
        }
    };
}

impl_from_emath!(f32);
impl_from_emath!(f64);
impl_from_emath!(i8);
impl_from_emath!(u8);
impl_from_emath!(i16);
impl_from_emath!(u16);
impl_from_emath!(i32);
impl_from_emath!(u32);
impl_from_emath!(i64);
impl_from_emath!(u64);
impl_from_emath!(isize);
impl_from_emath!(usize);
