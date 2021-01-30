use std::time::{Duration, Instant};
use std::{cell::RefCell, collections::HashMap};

use pyo3::prelude::*;

thread_local! {
    static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new());
}

/// Current profiler state.
///
/// Should be kept in a thread-local variable.
struct Profiler {
    start_times: HashMap<String, Instant>,
    run_times: HashMap<String, Vec<Duration>>,
}

impl Profiler {
    /// Initializes a new profiler state.
    fn new() -> Self {
        Self {
            start_times: HashMap::new(),
            run_times: HashMap::new(),
        }
    }

    /// Called when a function is called.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the called function.
    fn on_call(&mut self, name: &str) {
        let name = name.to_string();
        self.start_times.insert(name, Instant::now());
    }

    /// Called when a function returns.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the returning function.
    fn on_return(&mut self, name: &str) {
        let name = name.to_string();
        let start_time = self.start_times.get(&name);

        let start_time = match start_time {
            Some(time) => time,
            // Can happen if we get an early return from the top-level function
            // which is being profiled.
            None => return,
        };

        let run_time = Instant::now().duration_since(*start_time);
        self.run_times
            .entry(name)
            .or_insert_with(Vec::new)
            .push(run_time);
    }

    /// Prints useful profiling statistics gathered so far.
    fn print_statistics(&self) {
        for (name, run_times) in self.run_times.iter() {
            let total_run_time: u128 = run_times.iter().map(Duration::as_nanos).sum();
            let number_of_calls = run_times.len();
            let average_run_time = total_run_time / number_of_calls as u128;
            println!(
                "{}: {} nanoseconds (avg) × {} executions",
                name, average_run_time, number_of_calls
            );
        }
    }
}

/// An adaptive Python profiler, implemented in Rust.
#[pymodule]
fn adaptive_profiler(_py: Python, m: &PyModule) -> PyResult<()> {
    /// Function called by the Python interpreter whenever a function
    /// is called or returns.
    ///
    /// Must be of the type of the parameter passed to [`sys.setprofile`](https://docs.python.org/3/library/sys.html#sys.setprofile).
    #[pyfn(m, "profiler_callback")]
    #[text_signature = "(frame, event, arg, /)"]
    fn profiler_callback(frame: &PyAny, event: &str, _arg: &PyAny) -> PyResult<()> {
        let code = frame.getattr("f_code")?;
        let name: &str = code.getattr("co_name")?.extract()?;

        match event {
            "call" => PROFILER.with(|p| p.borrow_mut().on_call(name)),
            "return" => PROFILER.with(|p| p.borrow_mut().on_return(name)),
            _ => (),
        }

        Ok(())
    }

    #[pyfn(m, "print_statistics")]
    #[text_signature = "(/)"]
    fn print_statistics() {
        PROFILER.with(|p| p.borrow().print_statistics());
    }

    Ok(())
}
