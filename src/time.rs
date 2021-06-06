use std::time::{Duration, Instant};

use crate::counter::{Counter, IntoU128, Zero};

impl Zero for Duration {
    const ZERO: Self = Duration::ZERO;
}

impl IntoU128 for Duration {
    fn into_u128(self) -> u128 {
        self.as_nanos()
    }
}

pub struct TimeCounter;

impl Counter for TimeCounter {
    type DifferenceType = Duration;
    type ValueType = Instant;

    fn read(&self) -> Self::ValueType {
        Instant::now()
    }
}
