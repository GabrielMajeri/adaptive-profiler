use std::fmt::Debug;
use std::ops::{Add, Sub};

/// Trait implemented by types which can be initialized with a zero value.
pub trait Zero {
    const ZERO: Self;
}

pub trait IntoU128 {
    fn into_u128(self) -> u128;
}

/// Trait implemented by resources which can be measured by counting.
pub trait Counter {
    type DifferenceType: Debug
        + Copy
        + Clone
        + Zero
        + Add<Self::DifferenceType, Output = Self::DifferenceType>
        + IntoU128;
    type ValueType: Debug + Copy + Clone + Sub<Self::ValueType, Output = Self::DifferenceType>;

    fn read(&self) -> Self::ValueType;
}
