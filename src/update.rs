use crate::{
    counter::{Counter, IntoU128},
    profiler::{SamplesMap, StringInterner, Symbol},
};

#[allow(dead_code)]
pub enum Algorithm {
    Noop,
    Racing,
}

pub fn create_algorithm<C: Counter>(algorithm: Algorithm) -> Box<dyn UpdateAlgorithm<C>> {
    match algorithm {
        Algorithm::Noop => Box::new(NoopAlgorithm),
        Algorithm::Racing => Box::new(RacingAlgorithm::new()),
    }
}

pub trait UpdateAlgorithm<C: Counter> {
    fn update(&mut self, interner: &StringInterner, samples: &SamplesMap<C>) -> Vec<Symbol>;
}

#[derive(Debug, Copy, Clone)]
struct FunctionAggregateStatistics {
    symbol: Symbol,
    min: u128,
    max: u128,
    mean: u128,
}

#[non_exhaustive]
pub struct NoopAlgorithm;

impl<C: Counter> UpdateAlgorithm<C> for NoopAlgorithm {
    fn update(&mut self, _: &StringInterner, _: &SamplesMap<C>) -> Vec<Symbol> {
        Vec::new()
    }
}

#[non_exhaustive]
pub struct RacingAlgorithm {
    disable_updates: bool,
}

impl RacingAlgorithm {
    pub fn new() -> Self {
        RacingAlgorithm {
            disable_updates: false,
        }
    }
}

impl<C: Counter> UpdateAlgorithm<C> for RacingAlgorithm {
    fn update(&mut self, interner: &StringInterner, samples: &SamplesMap<C>) -> Vec<Symbol> {
        let mut blacklist = Vec::new();

        // Algorithm disengaged itself
        if self.disable_updates {
            return blacklist;
        }

        // No function calls were recorded
        if samples.is_empty() {
            return blacklist;
        }

        let times = samples.clone();

        // There is only one function left
        if samples.len() == 1 {
            let entry = times.into_iter().next().unwrap();
            let symbol = entry.0;
            blacklist.push(symbol);
            println!(
                "Blacklisting last remaining function: {}",
                interner.resolve(symbol).unwrap()
            );
            return blacklist;
        }

        let mut stats = Vec::with_capacity(interner.len());

        for (symbol, values) in samples.clone().into_iter() {
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
            let symbol = smallest_runtime.symbol;
            blacklist.push(symbol);

            let fn_name = interner.resolve(symbol).unwrap();
            println!("Blacklisting {}", fn_name);
        } else {
            // Not blacklisting anything means we can't reduce profiling overhead any further
            self.disable_updates = true;
        }

        return blacklist;
    }
}
