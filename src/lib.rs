use std::{cell::RefCell, mem};

use pyo3::{ffi, prelude::*, types::PyString, FromPyPointer, PyObjectProtocol};

mod counter;

mod stopwatch;

#[cfg(feature = "perfcnt")]
mod perfcnt;

mod time;

mod profiler;
use self::profiler::Profiler;

thread_local! {
    static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new());
}

#[pyclass(module = "adaptive_profiler")]
pub struct FunctionStatistics {
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
            ffi::PyEval_SetProfile(trace_func, ffi::Py_None());
        }
    }

    #[pyfn(m, "update")]
    #[text_signature = "(/)"]
    fn update() {
        PROFILER.with(|p| p.borrow_mut().update())
    }

    #[pyfn(m, "get_statistics")]
    #[text_signature = "(/)"]
    fn get_statistics() -> Vec<FunctionStatistics> {
        PROFILER.with(|p| p.borrow_mut().get_statistics())
    }

    #[pyfn(m, "print_statistics")]
    #[text_signature = "(/)"]
    fn print_statistics() {
        PROFILER.with(|p| p.borrow_mut().print_statistics());
    }

    #[pyfn(m, "reset")]
    #[text_signature = "(/)"]
    fn reset() {
        PROFILER.with(|p| p.borrow_mut().reset());
    }

    Ok(())
}
