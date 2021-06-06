use std::cell::UnsafeCell;

use perfcnt::{
    linux::{HardwareEventType, PerfCounterBuilderLinux},
    AbstractPerfCounter, PerfCounter,
};

use crate::counter::{Counter, IntoU128, Zero};

impl Zero for u64 {
    const ZERO: Self = 0u64;
}

impl IntoU128 for u64 {
    fn into_u128(self) -> u128 {
        u128::from(self)
    }
}

pub struct HardwarePerformanceCounter(UnsafeCell<PerfCounter>);

impl HardwarePerformanceCounter {
    pub fn branch_misses() -> Self {
        Self::new(HardwareEventType::BranchMisses)
    }

    pub fn cache_misses() -> Self {
        Self::new(HardwareEventType::CacheMisses)
    }

    pub fn new(event: HardwareEventType) -> Self {
        let pc = PerfCounterBuilderLinux::from_hardware_event(event)
            .finish()
            .expect("Could not create the counter");

        Self(UnsafeCell::new(pc))
    }

    fn get(&self) -> &mut PerfCounter {
        // This is safe because the profiler is thread local
        unsafe { &mut *self.0.get() }
    }

    pub fn start(&self) {
        self.get()
            .start()
            .expect("Failed to start performance counter");
    }

    pub fn reset(&self) {
        self.get()
            .reset()
            .expect("Failed to reset performance counter");
    }

    pub fn stop(&self) {
        self.get()
            .stop()
            .expect("Failed to stop performance counter");
    }
}

impl Drop for HardwarePerformanceCounter {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Counter for HardwarePerformanceCounter {
    type DifferenceType = u64;
    type ValueType = u64;

    fn read(&self) -> Self::ValueType {
        self.get()
            .read()
            .expect("Failed to read performance counter")
    }
}
