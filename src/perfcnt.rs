use perfcnt::{
    linux::{HardwareEventType, PerfCounterBuilderLinux},
    AbstractPerfCounter, PerfCounter,
};

use crate::counter::{Counter, Zero};

impl Zero for u64 {
    const ZERO: Self = 0u64;
}

pub struct HardwarePerformanceCounter(PerfCounter);

impl HardwarePerformanceCounter {
    pub fn new(event: HardwareEventType) -> Self {
        let pc = PerfCounterBuilderLinux::from_hardware_event(event)
            .finish()
            .expect("Could not create the counter");

        Self(pc)
    }

    pub fn start(&self) {
        self.0.start().expect("Failed to start performance counter");
    }

    pub fn reset(&self) {
        self.0.reset().expect("Failed to reset performance counter");
    }

    pub fn stop(&self) {
        self.0.stop().expect("Failed to stop performance counter");
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

    fn read(&mut self) -> Self::ValueType {
        self.0.read().expect("Failed to read performance counter")
    }
}
