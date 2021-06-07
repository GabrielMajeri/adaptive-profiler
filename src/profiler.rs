use std::{
    fs::File,
    io::{self, BufWriter, Write},
    mem,
};

use splay::{SplayMap, SplaySet};
use string_interner::{symbol::SymbolU32, StringInterner};

use crate::{
    counter::{Counter, IntoU128},
    lifecycle::Lifecycle,
    stopwatch::{Statistics, Stopwatch},
    FunctionStatistics,
};

pub trait AbstractProfiler: Lifecycle {
    /// Updates the function blacklist based on collected data.
    fn update(&mut self);

    /// Called when a function is entered.
    fn on_call(&mut self, name: &str);

    /// Called when a function returns.
    fn on_return(&mut self, name: &str);

    fn get_statistics(&mut self) -> Vec<FunctionStatistics>;
}

/// Current profiler state.
///
/// Should be kept in a thread-local variable.
pub struct Profiler<'a, C: Counter + Lifecycle> {
    counter: C,
    interner: StringInterner,
    blacklist: SplaySet<SymbolU32>,
    stack: Vec<Stopwatch<'a, C>>,
    times: SplayMap<SymbolU32, Vec<Statistics<C>>>,
    previous_times: SplayMap<SymbolU32, Vec<Statistics<C>>>,
}

impl<'a, C: Counter + Lifecycle> Profiler<'a, C> {
    /// Initializes a new profiler state.
    pub fn new(counter: C) -> Box<Self> {
        let profiler = Self {
            counter,
            interner: StringInterner::new(),
            blacklist: SplaySet::new(),
            stack: Vec::with_capacity(1024),
            times: SplayMap::new(),
            previous_times: SplayMap::new(),
        };
        println!("{}", profiler.interner.len());
        Box::new(profiler)
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

    fn record_statistics(&mut self, symbol: SymbolU32, stats: Statistics<C>) {
        if !self.times.contains_key(&symbol) {
            self.times.insert(symbol, Vec::new());
        }

        let times = self.times.get_mut(&symbol).unwrap();
        times.push(stats);
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
                let value: u128 = value.total.into_u128();
                write!(file, "{} ", value)?;
            }
            writeln!(file)?;
        }

        Ok(())
    }
}

impl<C: Counter + Lifecycle> Lifecycle for Profiler<'_, C> {
    fn enable(&self) {
        self.counter.enable();
    }

    fn disable(&self) {
        self.counter.disable();
    }

    fn reset(&self) {
        self.counter.reset();
    }
}

impl<C: Counter + Lifecycle> AbstractProfiler for Profiler<'_, C> {
    fn on_call(&mut self, name: &str) {
        let symbol = self.interner.get_or_intern(name);

        if self.blacklist.contains(&symbol) {
            return;
        }

        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.pause();
        }

        // This is safe because we only use a `Profiler` wrapped in a `Box`.
        let counter = unsafe { mem::transmute(&self.counter) };
        self.stack.push(Stopwatch::new(counter));
        self.stack.last_mut().unwrap().start();
    }

    fn on_return(&mut self, name: &str) {
        let symbol = self.interner.get_or_intern(name);

        if self.blacklist.contains(&symbol) {
            return;
        }

        // If we're not returning from the top-most function
        if let Some(mut stopwatch) = self.stack.pop() {
            // Stop the associated stopwatch
            let stats = stopwatch.stop();

            // Save the execution data
            self.record_statistics(symbol, stats);
        }

        // If we're still have a parent function
        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.unpause();
        }
    }

    fn update(&mut self) {
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
            let mut values: Vec<_> = values.into_iter().map(|v| v.total.into_u128()).collect();

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
    fn get_statistics(&mut self) -> Vec<FunctionStatistics> {
        // Move all values to the `previous_times` map.
        for (key, _) in self.times.clone().into_iter() {
            self.add_to_blacklist(key);
        }

        self.previous_times
            .clone()
            .into_iter()
            .map(|(sym, times)| {
                let name = self.interner.resolve(sym).unwrap().to_owned();
                let total = times.iter().map(|d| d.total.into_u128()).sum();
                let cumulative = times.iter().map(|d| d.cumulative.into_u128()).sum();
                let num_calls = times.len();
                FunctionStatistics {
                    name,
                    num_calls,
                    total,
                    cumulative,
                }
            })
            .collect()
    }
}

#[derive(Debug, Copy, Clone)]
struct FunctionAggregateStatistics {
    symbol: SymbolU32,
    min: u128,
    max: u128,
}
