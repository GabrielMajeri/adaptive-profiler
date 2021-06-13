use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

use splay::{SplayMap, SplaySet};

type Symbol = string_interner::symbol::SymbolU32;
type StringInterner = string_interner::StringInterner<Symbol>;

use crate::{
    counter::{Counter, IntoU128, Zero},
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

    /// Called when a C function is entered.
    fn on_c_call(&mut self, name: &str);

    /// Called when control is gained back from C code.
    fn on_c_return(&mut self, name: &str);

    fn get_statistics(&mut self) -> Vec<FunctionStatistics>;
}

/// Current profiler state.
///
/// Should be kept in a thread-local variable.
pub struct Profiler<C: Counter + Lifecycle> {
    counter: C,
    interner: StringInterner,
    blacklist: SplaySet<Symbol>,
    stack: Vec<Stopwatch<C>>,
    times: SplayMap<Symbol, Vec<Statistics<C>>>,
    previous_times: SplayMap<Symbol, Vec<Statistics<C>>>,
    c_enter_count: Option<C::ValueType>,
}

impl<C: Counter + Lifecycle> Profiler<C> {
    /// Initializes a new profiler state.
    pub fn new(counter: C) -> Self {
        Self {
            counter,
            interner: StringInterner::new(),
            blacklist: SplaySet::new(),
            stack: Vec::with_capacity(1024),
            times: SplayMap::new(),
            previous_times: SplayMap::new(),
            c_enter_count: None,
        }
    }

    fn add_to_blacklist(&mut self, symbol: Symbol) {
        let current_times = self.times.remove(&symbol).unwrap_or_default();

        if let Some(previous_times) = self.previous_times.get_mut(&symbol) {
            previous_times.extend(current_times);
        } else {
            self.previous_times.insert(symbol, current_times);
        }

        self.blacklist.insert(symbol);
    }

    fn record_statistics(&mut self, symbol: Symbol, stats: Statistics<C>) {
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

impl<C: Counter + Lifecycle> Lifecycle for Profiler<C> {
    fn enable(&self) {
        self.counter.enable();
    }

    fn disable(&self) {
        self.counter.disable();
    }
}

impl<C: Counter + Lifecycle> AbstractProfiler for Profiler<C> {
    fn on_call(&mut self, name: &str) {
        let symbol = self.interner.get_or_intern(name);

        if self.blacklist.contains(&symbol) {
            return;
        }

        let value = self.counter.read();

        // Pause the previous function's stopwatch
        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.pause(value);
        }

        // Start a new stopwatch for the function we just entered
        self.stack.push(Stopwatch::new(value));
    }

    fn on_return(&mut self, name: &str) {
        let symbol = self.interner.get_or_intern(name);

        if self.blacklist.contains(&symbol) {
            return;
        }

        let value = self.counter.read();

        // If we're not returning from the top-most function
        if let Some(mut stopwatch) = self.stack.pop() {
            // Stop the associated stopwatch
            let stats = stopwatch.stop(value);

            // Save the execution data
            self.record_statistics(symbol, stats);
        }

        // If we're still have a parent function
        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.unpause(value);
        }
    }

    fn on_c_call(&mut self, name: &str) {
        let symbol = self.interner.get_or_intern(name);

        if self.blacklist.contains(&symbol) {
            return;
        }

        self.c_enter_count = Some(self.counter.read());
    }

    fn on_c_return(&mut self, name: &str) {
        let symbol = self.interner.get_or_intern(name);

        if self.blacklist.contains(&symbol) {
            return;
        }

        let cumulative = self.counter.read() - self.c_enter_count.unwrap();
        let stats = Statistics::<C> {
            total: C::DifferenceType::ZERO,
            cumulative,
        };
        self.previous_times.insert(symbol, vec![stats]);

        self.c_enter_count = None;

        // C functions blacklisted the first time they're measured
        self.add_to_blacklist(symbol);
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
            let mut values: Vec<_> = values
                .into_iter()
                .map(|v| v.cumulative.into_u128())
                .collect();

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

            let mean = min + (max - min) / 2;
            stats.push(FunctionAggregateStatistics {
                symbol,
                min,
                max,
                mean,
            })
        }
        let stats = stats;

        // Find the function with the smallest average runtime
        let smallest_runtime = stats.iter().min_by_key(|stats| stats.mean).unwrap();

        let should_blacklist = stats
            .iter()
            .filter(|stats| stats.symbol != smallest_runtime.symbol)
            .all(|stats| smallest_runtime.max < stats.min);

        if should_blacklist {
            self.add_to_blacklist(smallest_runtime.symbol);
            let fn_name = self.interner.resolve(smallest_runtime.symbol).unwrap();
            println!("Blacklisting {}", fn_name);
        }
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
    symbol: Symbol,
    min: u128,
    max: u128,
    mean: u128,
}
