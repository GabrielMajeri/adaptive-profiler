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
    counter: C,
    elapsed: C::DifferenceType,
    start: C::ValueType,
    last: C::ValueType,
}

impl<C: Counter> Stopwatch<C> {
    /// Creates a new stopwatch using the given counter.
    pub fn new(mut counter: C) -> Self {
        let start = counter.read();
        Self {
            counter,
            elapsed: C::DifferenceType::ZERO,
            start,
            last: start,
        }
    }

    /// Starts the stopwatch.
    pub fn start(&mut self) {
        self.elapsed = C::DifferenceType::ZERO;
        self.start = self.counter.read();
        self.last = self.start;
    }

    /// Temporarily pauses the stopwatch.
    pub fn pause(&mut self) {
        self.elapsed = self.elapsed + (self.counter.read() - self.last);
    }

    /// Resumses the stopwatch.
    pub fn unpause(&mut self) {
        self.last = self.counter.read();
    }

    /// Stops the stopwatch.
    pub fn stop(&mut self) -> Statistics<C> {
        let cumulative = self.counter.read() - self.start;
        let total = self.elapsed + (self.counter.read() - self.last);
        Statistics { total, cumulative }
    }
}
