#[cfg(feature = "async-std")]
mod async_std;
use std::{future::Future, time::Duration};

#[cfg(feature = "async-std")]
pub use async_std::*;
#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::*;
#[cfg(feature = "thread")]
mod thread;
#[cfg(feature = "thread")]
pub use thread::*;
#[cfg(feature = "promise")]
mod promise;
#[cfg(feature = "promise")]
pub use promise::*;

mod local;
pub use local::*;

use crate::{Dtu, Task, TaskRun, TaskUid, prelude::IntoSchedule};

pub trait Runtime {
    type Handle;
}

pub trait AsyncRuntime: Runtime + Send + Sync {
    /// wake runner after a duration
    fn wake_after(&self, duration: Duration, ctx: &mut std::task::Context<'_>);
    fn spawn<F>(task: F) -> Self::Handle
    where
        F: Future<Output = ()> + Send + 'static;
}

impl<R: Runtime> Task<R> {
    /// Create a new task.
    ///
    ///
    /// The run function could have the following signature:
    ///
    /// `Fn(<&mut Runtime>?, ..Args)`
    ///
    /// the Args could be any type that implements `TaskRunArg`.
    ///
    /// For now, these types are supported:
    /// * [`TaskRun`] : information about this run.
    /// * [`TaskUid`] : the task unique identifier.
    /// * [`Dtu`] : the time when this task is scheduled to run.
    ///
    /// For example, you can call this function like this:
    ///
    ///
    /// ```rust
    /// # use tsuki_scheduler::prelude::*;
    /// let task = Task::<Local>::new(now(), |time: Dtu, id: TaskUid| {
    ///    println!("Time: {time}, Uid: {id}");
    /// });
    /// ```
    ///
    ///
    pub fn new<S, F, A>(schedule: S, run: F) -> Self
    where
        S: IntoSchedule,
        F: IntoRunTaskFn<R, A>,
    {
        Self {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(run.convert()),
        }
    }
}
impl<R: AsyncRuntime> Task<R> {
    /// This is basically the same as [`Task::new`] but with more hint for compiler that this is an async function.
    #[allow(private_bounds)]
    pub fn new_async<S, F, A, Fut>(schedule: S, run: F) -> Self
    where
        S: IntoSchedule,
        F: IntoRunTaskFn<R, Async<A, Fut>>,
    {
        Self {
            schedule: Box::new(schedule.into_schedule()),
            run: Box::new(run.convert()),
        }
    }
}

pub trait IntoRunTaskFn<R: Runtime, A> {
    fn convert(self) -> impl Fn(&mut R, &TaskRun) -> <R as Runtime>::Handle + Send + 'static;
}

pub trait AsyncTaskFn<R: Runtime, A> {
    type Future: std::future::Future<Output = ()> + Send + 'static;
    fn convert(self) -> impl Fn(&mut R, &TaskRun) -> Self::Future;
}

pub trait TaskRunArg: Sized {
    fn extract(task_run: &TaskRun) -> Self;
}

impl TaskRunArg for TaskRun {
    fn extract(task_run: &TaskRun) -> Self {
        task_run.clone()
    }
}

impl TaskRunArg for TaskUid {
    fn extract(task_run: &TaskRun) -> Self {
        task_run.key
    }
}

impl TaskRunArg for Dtu {
    fn extract(task_run: &TaskRun) -> Self {
        task_run.time
    }
}

struct WithRuntime<A>(A);
struct Async<A, F>(A, F);

macro_rules! impl_for {
    ($($T: ident)*) => {
        impl_for!([][$($T)*]);
    };
    ([$($T: ident)*][$TN: ident $($Rest: ident)*]) => {
        impl_for!(@impl [$($T)*]);
        impl_for!([$($T)* $TN][$($Rest)*]);
    };
    ([$($T: ident)*][]) => {
        impl_for!(@impl [$($T)*]);
    };
    (@impl [$($T: ident)*]) => {
        impl<R, F, $($T,)*> IntoRunTaskFn<R, ($($T,)*)> for F
        where
            R: Runtime,
            F: FnOnce($($T,)*) -> R::Handle + Send + 'static + Clone,
            $($T: TaskRunArg,)*
        {
            #[allow(unused_variables)]
            fn convert(
                self,
            ) -> impl Fn(&mut R, &TaskRun) -> <R as Runtime>::Handle + Send + 'static {
                move |_, task_run| (self.clone())($($T::extract(task_run),)*)
            }
        }
        impl<R, F, $($T,)*> IntoRunTaskFn<R, WithRuntime<($($T,)*)>> for F
        where
            R: Runtime,
            F: FnOnce(&mut R, $($T,)*) -> R::Handle + Send + Sync + 'static + Clone,
            $($T: TaskRunArg,)*
        {
            #[allow(unused_variables)]
            fn convert(
                self,
            ) -> impl Fn(&mut R, &TaskRun) -> <R as Runtime>::Handle + Send+ 'static {
                move |run_time, task_run| (self.clone())(run_time, $($T::extract(task_run),)*)
            }
        }
        impl<R, F, Fut, $($T,)*> IntoRunTaskFn<R, Async<($($T,)*), Fut>> for F
        where
            R: AsyncRuntime,
            F: FnOnce($($T,)*) -> Fut + Send + 'static + Clone,
            Fut: std::future::Future<Output = ()> + Send + 'static ,
            $($T: TaskRunArg,)*
        {
            #[allow(unused_variables)]
            fn convert(
                self,
            ) -> impl Fn(&mut R, &TaskRun) -> <R as Runtime>::Handle + Send + 'static {
                move |_, task_run| R::spawn((self.clone())($($T::extract(task_run),)*))
            }
        }
    }
}

impl_for!(
    T0 T1 T2 T3
);
