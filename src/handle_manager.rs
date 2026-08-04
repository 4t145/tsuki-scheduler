use crate::TaskRun;

/// A trait for processing handles produced by the runtime
pub trait HandleManager<H> {
    fn manage(&mut self, task_run: &TaskRun, handle: H);
}

impl<H> HandleManager<H> for Vec<H> {
    fn manage(&mut self, _: &TaskRun, handle: H) {
        self.push(handle);
    }
}

/// An empty handle manager that does nothing
impl<T> HandleManager<T> for () {
    fn manage(&mut self, _: &TaskRun, _: T) {}
}
