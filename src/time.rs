use std::time::{Duration, Instant};

use crate::counter::{Counter, Zero};

impl Zero for Duration {
    const ZERO: Self = Duration::ZERO;
}

pub struct TimeCounter;

impl Counter for TimeCounter {
    type DifferenceType = Duration;
    type ValueType = Instant;

    fn read(&mut self) -> Self::ValueType {
        Instant::now()
    }
}
