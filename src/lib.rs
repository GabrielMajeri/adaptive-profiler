use std::ptr;
use std::{cell::RefCell, mem};

use splay::SplayMap;

use pyo3::{ffi, prelude::*, types::PyString, FromPyPointer, PyObjectProtocol};

mod counter;

mod stopwatch;
use self::stopwatch::{Statistics, Stopwatch};

#[cfg(feature = "perfcnt")]
mod perfcnt;

mod time;
use self::time::TimeCounter;

thread_local! {
    static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new());
}

#[pyclass(module = "adaptive_profiler")]
struct FunctionStatistics {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    num_calls: usize,
    #[pyo3(get, set)]
    total_time: u128,
    #[pyo3(get, set)]
    cumulative_time: u128,
}

#[pyproto]
impl PyObjectProtocol for FunctionStatistics {
    fn __repr__(&self) -> String {
        format!(
            "{} ({} calls): {} ns / {} ns",
            self.name, self.num_calls, self.total_time, self.cumulative_time
        )
    }
}

/// Current profiler state.
///
/// Should be kept in a thread-local variable.
struct Profiler {
    stack: Vec<Stopwatch<TimeCounter>>,
    times: SplayMap<String, Vec<Statistics<TimeCounter>>>,
}

impl Profiler {
    /// Initializes a new profiler state.
    fn new() -> Self {
        Self {
            stack: Vec::with_capacity(1024),
            times: SplayMap::new(),
        }
    }

    /// Resets the profiler's internal data structures.
    fn reset(&mut self) {
        self.stack = Vec::with_capacity(1024);
        self.times = SplayMap::new();
    }

    /// Called when a function is called.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the called function.
    fn on_call(&mut self, _name: &str) {
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
    fn on_return(&mut self, name: &str) {
        // If we're not returning from the top-most function
        if let Some(mut stopwatch) = self.stack.pop() {
            // Stop the associated stopwatch
            let duration = stopwatch.stop();

            // Save the execution time
            if !self.times.contains_key(name) {
                self.times.insert(name.to_string(), Vec::new());
            }

            let times = self.times.get_mut(name).unwrap();
            times.push(duration);
        }

        // If we're still have a parent function
        if let Some(stopwatch) = self.stack.last_mut() {
            stopwatch.unpause();
        }
    }

    /// Returns a vector of the profiling statistics gathered so far.
    fn get_statistics(&self) -> Vec<FunctionStatistics> {
        self.times
            .clone()
            .into_iter()
            .map(|(name, times)| {
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
    fn print_statistics(&self) {
        self.get_statistics().into_iter().for_each(|stats| {
            let average_run_time = stats.cumulative_time / stats.num_calls as u128;
            println!(
                "{}: cumulative {} ns = {} ns (avg) × {} executions",
                stats.name, stats.cumulative_time, average_run_time, stats.num_calls
            );
        })
    }
}

const PY_TRACE_CALL: i32 = 0;
const PY_TRACE_RETURN: i32 = 3;

/// Function called by the Python interpreter whenever a function
/// is called or returns.
extern "C" fn profiler_callback(
    _obj: *mut ffi::PyObject,
    frame: *mut ffi::PyFrameObject,
    event: i32,
    _arg: *mut ffi::PyObject,
) -> i32 {
    let py = unsafe { Python::assume_gil_acquired() };

    let frame = unsafe { &*frame };
    let code = unsafe { &*frame.f_code };
    let name = unsafe { PyString::from_borrowed_ptr(py, code.co_name) };
    let name = name.to_str().unwrap();

    match event {
        PY_TRACE_CALL => PROFILER.with(|p| p.borrow_mut().on_call(name)),
        PY_TRACE_RETURN => PROFILER.with(|p| p.borrow_mut().on_return(name)),
        _ => (),
    }

    0
}

/// An adaptive Python profiler, implemented in Rust.
#[pymodule]
fn adaptive_profiler(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<FunctionStatistics>()?;

    /// Starts the profiler for subsequent code.
    #[pyfn(m, "enable")]
    #[text_signature = "(/)"]
    fn enable() {
        unsafe {
            ffi::PyEval_SetProfile(profiler_callback, ffi::Py_None());
        }
    }

    #[pyfn(m, "disable")]
    #[text_signature = "(/)"]
    fn disable() {
        unsafe {
            #[allow(invalid_value)]
            let trace_func = mem::transmute(0usize);
            // TODO: this doesn't work!
            ffi::PyEval_SetProfile(trace_func, ptr::null_mut());
        }
    }

    #[pyfn(m, "get_statistics")]
    #[text_signature = "(/)"]
    fn get_statistics() -> Vec<FunctionStatistics> {
        PROFILER.with(|p| p.borrow().get_statistics())
    }

    #[pyfn(m, "print_statistics")]
    #[text_signature = "(/)"]
    fn print_statistics() {
        PROFILER.with(|p| p.borrow().print_statistics());
    }

    #[pyfn(m, "reset")]
    #[text_signature = "(/)"]
    fn reset() {
        PROFILER.with(|p| p.borrow_mut().reset());
    }

    Ok(())
}
