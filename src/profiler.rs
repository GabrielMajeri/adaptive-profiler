use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

use splay::{SplayMap, SplaySet};

pub(crate) type Symbol = string_interner::symbol::SymbolU32;
pub(crate) type StringInterner = string_interner::StringInterner<Symbol>;

type Blacklist = SplaySet<Symbol>;
pub(crate) type SamplesMap<C> = SplayMap<Symbol, Vec<Statistics<C>>>;

use crate::{
    counter::{Counter, IntoU128, Zero},
    lifecycle::Lifecycle,
    stopwatch::{Statistics, Stopwatch},
    update::{create_algorithm, Algorithm, UpdateAlgorithm},
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
    update_algorithm: Box<dyn UpdateAlgorithm<C>>,
    interner: StringInterner,
    blacklist: Blacklist,
    stack: Vec<Stopwatch<C>>,
    samples: SamplesMap<C>,
    previous_samples: SamplesMap<C>,
    c_enter_count: Option<C::ValueType>,
}

impl<C: Counter + Lifecycle> Profiler<C> {
    /// Initializes a new profiler state.
    pub fn new(counter: C) -> Self {
        Self {
            counter,
            update_algorithm: create_algorithm(Algorithm::Racing),
            interner: StringInterner::new(),
            blacklist: SplaySet::new(),
            stack: Vec::with_capacity(1024),
            samples: SplayMap::new(),
            previous_samples: SplayMap::new(),
            c_enter_count: None,
        }
    }

    fn add_to_blacklist(&mut self, symbol: Symbol) {
        let current_samples = self.samples.remove(&symbol).unwrap_or_default();

        if let Some(previous_samples) = self.previous_samples.get_mut(&symbol) {
            previous_samples.extend(current_samples);
        } else {
            self.previous_samples.insert(symbol, current_samples);
        }

        self.blacklist.insert(symbol);
    }

    fn record_statistics(&mut self, symbol: Symbol, stats: Statistics<C>) {
        if !self.samples.contains_key(&symbol) {
            self.samples.insert(symbol, Vec::new());
        }

        self.samples.get_mut(&symbol).unwrap().push(stats);
    }

    #[allow(dead_code)]
    fn dump_times(&self, path: &str) -> io::Result<()> {
        // Open a file for writing
        let file = File::create(path)?;

        // Buffer the output
        let mut file = BufWriter::new(file);

        // Write statistics for each function on a new line
        for (symbol, values) in self.samples.clone().into_iter() {
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

        let now = self.counter.read();
        let cumulative = now - self.c_enter_count.unwrap_or(now);
        let stats = Statistics::<C> {
            total: C::DifferenceType::ZERO,
            cumulative,
        };
        self.previous_samples.insert(symbol, vec![stats]);

        self.c_enter_count = None;

        // C functions blacklisted the first time they're measured
        self.add_to_blacklist(symbol);
    }

    fn update(&mut self) {
        let newly_blacklisted = self.update_algorithm.update(&self.interner, &self.samples);

        for symbol in newly_blacklisted {
            self.add_to_blacklist(symbol);
        }
    }

    /// Returns a vector of the profiling statistics gathered so far.
    fn get_statistics(&mut self) -> Vec<FunctionStatistics> {
        // Move all values to the previous samples map.
        for (key, _) in self.samples.clone().into_iter() {
            self.add_to_blacklist(key);
        }

        self.previous_samples
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
