use std::{cell::RefCell, mem};

use pyo3::{ffi, prelude::*, types::PyString, FromPyPointer, PyObjectProtocol};

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
        PY_TRACE_CALL => with_profiler(|p| p.on_call(name)),
        PY_TRACE_RETURN => with_profiler(|p| p.on_return(name)),
        // PY_TRACE_C_CALL => with_profiler(|p| p.on_call(name)),
        // PY_TRACE_C_RETURN => with_profiler(|p| p.on_return(name)),
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
        PROFILER.with(|profiler| {
            if profiler.borrow().is_some() {
                panic!("Profiler has already been enabled");
            }
            let counter = crate::time::TimeCounter;
            //let counter = crate::perfcnt::HardwarePerformanceCounter::cache_misses();
            //counter.start();
            profiler.replace(Some(Profiler::new(counter)));
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
        PROFILER.with(|p| {
            p.replace(None);
        });
    }

    #[pyfn(m, "update")]
    #[text_signature = "(/)"]
    fn update() {
        with_profiler(|p| p.update())
    }

    #[pyfn(m, "get_statistics")]
    #[text_signature = "(/)"]
    fn get_statistics() -> Vec<FunctionStatistics> {
        with_profiler(|p| p.get_statistics())
    }

    Ok(())
}
