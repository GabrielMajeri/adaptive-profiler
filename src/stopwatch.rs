use crate::counter::{Counter, Zero};

#[derive(Debug)]
pub struct Statistics<C: Counter> {
    /// Counter spent only in the function.
    pub total: C::DifferenceType,
    /// Counter spent between entry into the function until final return.
    pub cumulative: C::DifferenceType,
}

impl<C: Counter> Copy for Statistics<C> {}

impl<C: Counter> Clone for Statistics<C> {
    fn clone(&self) -> Self {
        Self {
            total: self.total,
            cumulative: self.cumulative,
        }
    }
}

pub struct Stopwatch<C: Counter> {
    elapsed: C::DifferenceType,
    start: C::ValueType,
    last: C::ValueType,
}

impl<C: Counter> Stopwatch<C> {
    /// Creates a new stopwatch and immediately starts it.
    #[inline]
    pub fn new(value: C::ValueType) -> Self {
        Self {
            elapsed: C::DifferenceType::ZERO,
            start: value,
            last: value,
        }
    }

    /// Temporarily pauses the stopwatch.
    #[inline]
    pub fn pause(&mut self, value: C::ValueType) {
        self.elapsed = self.elapsed + (value - self.last);
    }

    /// Resumses the stopwatch.
    #[inline]
    pub fn unpause(&mut self, value: C::ValueType) {
        self.last = value;
    }

    /// Stops the stopwatch.
    #[inline]
    pub fn stop(&mut self, value: C::ValueType) -> Statistics<C> {
        let cumulative = value - self.start;
        let total = self.elapsed + (value - self.last);
        Statistics { total, cumulative }
    }
}
