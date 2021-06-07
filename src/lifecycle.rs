/// Trait implemented by resources managed under a profiler's lifecycle.
pub trait Lifecycle {
    fn enable(&self) {}
    fn disable(&self) {}
    fn reset(&self) {}
}
