use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

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
    previous_times: SplayMap<SymbolU32, Vec<Statistics<TimeCounter>>>,
}

impl Profiler {
    /// Initializes a new profiler state.
    pub fn new() -> Self {
        Self {
            interner: StringInterner::new(),
            blacklist: SplaySet::new(),
            stack: Vec::with_capacity(1024),
            times: SplayMap::new(),
            previous_times: SplayMap::new(),
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
        // No function calls were recorded
        if self.times.is_empty() {
            return;
        }

        let times = self.times.clone();

        // There is only one function left
        if self.times.len() == 1 {
            let entry = times.into_iter().next().unwrap();
            let symbol = entry.0;
            self.add_to_blacklist(symbol);
            println!(
                "Blacklisting last remaining function: {}",
                self.interner.resolve(symbol).unwrap()
            );
            return;
        }

        let mut stats = Vec::with_capacity(self.interner.len());

        for (symbol, values) in self.times.clone().into_iter() {
            let mut values: Vec<_> = values.into_iter().map(|v| v.total.as_nanos()).collect();

            let mut min = u128::MAX;
            let mut max = u128::MIN;
            if values.len() < 10 {
                // If we don't have enough samples to extract a confidence interval,
                // just use any value available.
                let value = values.first().unwrap().clone();
                min = value;
                max = value;
            } else {
                // Sort the running times
                values.sort_unstable();

                // Drop the first and last 5%
                let percent = 0.05;
                let n = values.len() as f64;
                let start = (percent * n) as usize;
                let end = ((1.0 - percent) * n) as usize;

                let values = &values[start..end];

                for value in values.into_iter().copied() {
                    if value > max {
                        max = value;
                    }
                    if value < min {
                        min = value;
                    }
                }
            }

            stats.push(FunctionAggregateStatistics { symbol, min, max })
        }
        let stats = stats;

        let mut ordered_by_max = stats.clone();
        ordered_by_max.sort_unstable_by_key(|s| s.max);
        let ordered_by_max = ordered_by_max;

        let mut ordered_by_min = stats.clone();
        ordered_by_min.sort_unstable_by_key(|s| s.min);
        let ordered_by_min = ordered_by_min;

        // for fn_stats in ordered_by_max.iter() {
        //     let fn_name = self.interner.resolve(fn_stats.symbol).unwrap();
        //     println!("{} - min: {}, max: {}", fn_name, fn_stats.min, fn_stats.max);
        // }
        // for fn_stats in ordered_by_min.iter() {
        //     let fn_name = self.interner.resolve(fn_stats.symbol).unwrap();
        //     println!("{} - min: {}, max: {}", fn_name, fn_stats.min, fn_stats.max);
        // }

        let mut added_to_blacklist = Vec::new();
        for fn_stats in ordered_by_max.iter() {
            let first = ordered_by_min
                .iter()
                .skip_while(|s| s.symbol == fn_stats.symbol)
                .next()
                .unwrap();

            if fn_stats.max <= first.min {
                self.add_to_blacklist(fn_stats.symbol);
                added_to_blacklist.push(fn_stats.symbol);
            } else {
                break;
            }
        }

        let added_to_blacklist: Vec<_> = added_to_blacklist
            .into_iter()
            .map(|s| self.interner.resolve(s).unwrap())
            .collect();

        println!(
            "Newly blacklisted functions: {}",
            added_to_blacklist.join(", ")
        );
    }

    /// Returns a vector of the profiling statistics gathered so far.
    pub fn get_statistics(&mut self) -> Vec<FunctionStatistics> {
        for (key, _) in self.times.clone().into_iter() {
            self.add_to_blacklist(key);
        }

        self.previous_times
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
    pub fn print_statistics(&mut self) {
        self.get_statistics().into_iter().for_each(|stats| {
            let average_run_time = stats.cumulative_time / stats.num_calls as u128;
            println!(
                "{}: cumulative {} ns = {} ns (avg) × {} executions",
                stats.name, stats.cumulative_time, average_run_time, stats.num_calls
            );
        })
    }

    fn add_to_blacklist(&mut self, symbol: SymbolU32) {
        let current_times = self.times.remove(&symbol).unwrap_or_default();

        if let Some(previous_times) = self.previous_times.get_mut(&symbol) {
            previous_times.extend(current_times);
        } else {
            self.previous_times.insert(symbol, current_times);
        }

        self.blacklist.insert(symbol);
    }

    #[allow(dead_code)]
    fn dump_times(&self, path: &str) -> io::Result<()> {
        // Open a file for writing
        let file = File::create(path)?;

        // Buffer the output
        let mut file = BufWriter::new(file);

        // Write statistics for each function on a new line
        for (symbol, values) in self.times.clone().into_iter() {
            let fn_name = self.interner.resolve(symbol).unwrap();

            writeln!(file, "{}", fn_name)?;
            for value in values {
                let value = value.total.as_nanos();
                write!(file, "{} ", value)?;
            }
            writeln!(file)?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
struct FunctionAggregateStatistics {
    symbol: SymbolU32,
    min: u128,
    max: u128,
}
