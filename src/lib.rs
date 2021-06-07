use std::{cell::RefCell, mem, ptr};

use pyo3::{ffi, prelude::*, types::PyString, FromPyPointer, PyContextProtocol, PyObjectProtocol};

mod lifecycle;

mod counter;

mod stopwatch;

#[cfg(feature = "perfcnt")]
mod perfcnt;

mod time;

mod profiler;
use crate::profiler::{AbstractProfiler, Profiler};

thread_local! {
    static PROFILER: RefCell<Option<Box<dyn AbstractProfiler>>> = RefCell::new(None);
}

fn with_profiler<F, R>(f: F) -> R
where
    F: FnOnce(&mut dyn AbstractProfiler) -> R,
{
    PROFILER.with(|profiler| {
        let mut profiler = profiler.borrow_mut();
        let profiler = profiler.as_mut().unwrap();
        f(profiler.as_mut())
    })
}

#[pyclass]
pub struct FunctionStatistics {
    #[pyo3(get, set)]
    name: String,
    #[pyo3(get, set)]
    num_calls: usize,
    #[pyo3(get, set)]
    total: u128,
    #[pyo3(get, set)]
    cumulative: u128,
}

#[pyproto]
impl PyObjectProtocol for FunctionStatistics {
    fn __repr__(&self) -> String {
        format!(
            "{} ({} calls): {} total / {} cumulative",
            self.name, self.num_calls, self.total, self.cumulative
        )
    }
}

/// An adaptive Python profiler, implemented in Rust.
#[pyclass(unsendable)]
pub struct AdaptiveProfiler {}

#[pymethods]
impl AdaptiveProfiler {
    #[new]
    fn new() -> Self {
        let counter = crate::time::TimeCounter;
        //let counter = crate::perfcnt::HardwarePerformanceCounter::cache_misses();
        //counter.start();
        let profiler = Profiler::new(counter);
        PROFILER.with(|p| p.replace(Some(profiler)));
        Self {}
    }

    /// Starts the profiler for subsequent code.
    fn enable(&self) {
        with_profiler(|profiler| {
            profiler.enable();
            unsafe {
                let profiler_callback = profiler_callback as *const ();
                let profiler_callback = mem::transmute(profiler_callback);
                ffi::PyEval_SetProfile(profiler_callback, ffi::Py_None());
            }
        });
    }

    /// Disables the monitoring of further calls.
    fn disable(&self) {
        unsafe {
            #[allow(invalid_value)]
            let trace_func = mem::transmute(0usize);
            let null_ptr = ptr::null_mut();
            ffi::PyEval_SetProfile(trace_func, null_ptr);
        }

        // TODO: determine why `PyEval_SetProfile` doesn't work
        Python::with_gil(|py| {
            let sys = PyModule::import(py, "sys").unwrap();
            let setprofile = sys.getattr("setprofile").unwrap();
            setprofile.call0().unwrap();
        });

        with_profiler(|profiler| profiler.disable());
    }

    /// Updates the list of functions to be profiled.
    fn update(&mut self) {
        with_profiler(|profiler| profiler.update());
    }

    /// Retrieves statistics for the last profiling run.
    fn get_statistics(&mut self) -> Vec<FunctionStatistics> {
        with_profiler(|profiler| profiler.get_statistics())
    }
}

#[pyproto]
impl PyContextProtocol for AdaptiveProfiler {
    fn __enter__(&mut self) {
        self.enable();
    }

    fn __exit__(
        &mut self,
        _exc_type: Option<&PyAny>,
        _exc_value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) {
        self.disable();
    }
}

const PY_TRACE_CALL: i32 = 0;
const PY_TRACE_RETURN: i32 = 3;
// const PY_TRACE_C_CALL: i32 = 4;
// const PY_TRACE_C_RETURN: i32 = 6;

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
        PY_TRACE_CALL => with_profiler(|profiler| profiler.on_call(name)),
        PY_TRACE_RETURN => with_profiler(|profiler| profiler.on_return(name)),
        // PY_TRACE_C_CALL => with_profiler(|p| p.on_call(name)),
        // PY_TRACE_C_RETURN => with_profiler(|p| p.on_return(name)),
        _ => (),
    }

    0
}

#[pymodule]
fn adaptive_profiler(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<FunctionStatistics>()?;
    m.add_class::<AdaptiveProfiler>()?;

    Ok(())
}
