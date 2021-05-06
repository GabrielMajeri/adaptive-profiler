use std::time::{Duration, Instant};
use std::{cell::RefCell, mem};

use splay::SplayMap;

use pyo3::{ffi, prelude::*, types::PyString, FromPyPointer, PyObjectProtocol};

thread_local! {
    static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new());
}
use perfcnt::linux::{HardwareEventType, PerfCounterBuilderLinux};
use perfcnt::{AbstractPerfCounter, PerfCounter};

#[pyclass(module = "adaptive_profiler")]
struct FunctionStatistics {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    num_calls: usize,
    #[pyo3(get, set)]
    cumulative_time: u128,
}

#[pyproto]
impl PyObjectProtocol for FunctionStatistics {
    fn __repr__(&self) -> String {
        format!(
            "{} ({} calls): {} ns",
            self.name, self.num_calls, self.cumulative_time
        )
    }
}

/// Current profiler state.
///
/// Should be kept in a thread-local variable.
struct Profiler {
    start_times: SplayMap<String, Instant>,
    run_times: SplayMap<String, Vec<Duration>>,
}

impl Profiler {
    /// Initializes a new profiler state.
    fn new() -> Self {
        Self {
            start_times: SplayMap::new(),
            run_times: SplayMap::new(),
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

        let entry = self.run_times.get_mut(&name);
        if let Some(fn_run_times) = entry {
            fn_run_times.push(run_time);
        } else {
            self.run_times.insert(name, vec![run_time]);
        }
    }

    /// Returns a vector of the profiling statistics gathered so far.
    fn get_statistics(&self) -> Vec<FunctionStatistics> {
        self.run_times
            .clone()
            .into_iter()
            .map(|(name, run_times)| {
                let cumulative_time: u128 = run_times.iter().map(Duration::as_nanos).sum();
                let num_calls = run_times.len();
                FunctionStatistics {
                    name,
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
    let gil = Python::acquire_gil();
    let py = gil.python();

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

thread_local! {
    static CACHE_MISSES_PERFORMANCE_COUNTER: RefCell<PerfCounter> = RefCell::new(
        PerfCounterBuilderLinux::from_hardware_event(HardwareEventType::CacheMisses)
            .finish()
            .expect("Could not create the counter")
    );
}

/// An adaptive Python profiler, implemented in Rust.
#[pymodule]
fn adaptive_profiler(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<FunctionStatistics>()?;

    /// Starts the profiler for subsequent code.
    #[pyfn(m, "enable")]
    #[text_signature = "(/)"]
    fn enable() {
        CACHE_MISSES_PERFORMANCE_COUNTER.with(|pc| {
            let pc = pc.borrow();
            pc.start().expect("Can not start the counter");
        });

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
            ffi::PyEval_SetProfile(trace_func, ffi::Py_None());
        }

        CACHE_MISSES_PERFORMANCE_COUNTER.with(|pc| {
            let mut pc = pc.borrow_mut();
            pc.stop().expect("Can not stop the counter");
            let res = pc.read().expect("Can not read the counter");
            println!("Measured {} cache misses.", res);
        });
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

    Ok(())
}
