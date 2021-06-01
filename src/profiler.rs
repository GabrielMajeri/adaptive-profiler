use splay::{SplayMap, SplaySet};
use string_interner::{symbol::SymbolU32, StringInterner};

use crate::{
    stopwatch::{Statistics, Stopwatch},
    time::TimeCounter,
    FunctionStatistics,
};

/// Current profiler state.
///
/// Should be kept in a thread-local variable.
pub struct Profiler {
    interner: StringInterner,
    blacklist: SplaySet<SymbolU32>,
    stack: Vec<Stopwatch<TimeCounter>>,
    times: SplayMap<SymbolU32, Vec<Statistics<TimeCounter>>>,
}

impl Profiler {
    /// Initializes a new profiler state.
    pub fn new() -> Self {
        Self {
            interner: StringInterner::new(),
            blacklist: SplaySet::new(),
            stack: Vec::with_capacity(1024),
            times: SplayMap::new(),
        }
    }

    /// Resets the profiler's internal data structures.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Called when a function is called.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the called function.
    pub fn on_call(&mut self, name: &str) {
        let sym = self.interner.get_or_intern(name);

        if self.blacklist.contains(&sym) {
            return;
        }

        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.pause();
        }

        self.stack.push(Stopwatch::new(TimeCounter));
        self.stack.last_mut().unwrap().start();
    }

    /// Called when a function returns.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the returning function.
    pub fn on_return(&mut self, name: &str) {
        let sym = self.interner.get_or_intern(name);

        if self.blacklist.contains(&sym) {
            return;
        }

        // If we're not returning from the top-most function
        if let Some(mut stopwatch) = self.stack.pop() {
            // Stop the associated stopwatch
            let duration = stopwatch.stop();

            // Save the execution time
            if !self.times.contains_key(&sym) {
                self.times.insert(sym, Vec::new());
            }

            let times = self.times.get_mut(&sym).unwrap();
            times.push(duration);
        }

        // If we're still have a parent function
        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.unpause();
        }
    }

    /// Updates the function blacklist based on collected data.
    pub fn update(&mut self) {
        // Add all found functions to the blacklist.
        // TODO: use the racing algorithm instead
        let symbols = self.interner.into_iter().map(|(sym, _)| sym);
        self.blacklist.extend(symbols);
    }

    /// Returns a vector of the profiling statistics gathered so far.
    pub fn get_statistics(&self) -> Vec<FunctionStatistics> {
        self.times
            .clone()
            .into_iter()
            .map(|(sym, times)| {
                let name = self.interner.resolve(sym).unwrap().to_owned();
                let total_time = times.iter().map(|d| d.total.as_nanos()).sum();
                let cumulative_time: u128 = times.iter().map(|d| d.cumulative.as_nanos()).sum();
                let num_calls = times.len();
                FunctionStatistics {
                    name,
                    total_time,
                    cumulative_time,
                    num_calls,
                }
            })
            .collect()
    }

    /// Prints useful profiling statistics gathered so far.
    pub fn print_statistics(&self) {
        self.get_statistics().into_iter().for_each(|stats| {
            let average_run_time = stats.cumulative_time / stats.num_calls as u128;
            println!(
                "{}: cumulative {} ns = {} ns (avg) × {} executions",
                stats.name, stats.cumulative_time, average_run_time, stats.num_calls
            );
        })
    }
}
