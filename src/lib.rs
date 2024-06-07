use std::{
    collections::{BinaryHeap, HashMap},
    hash::Hash,
};
pub type Dtu = chrono::DateTime<chrono::Utc>;
pub mod runtime;
pub mod schedule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskUid(pub(crate) u64);

pub trait Schedule {
    fn peek_next(&mut self) -> Option<Dtu>;
    fn next(&mut self) -> Option<Dtu>;
    fn forward(&mut self, dtu: Dtu);
}

pub fn forward_default<S: Schedule>(schedule: &mut S, dtu: Dtu) {
    while let Some(next) = schedule.peek_next() {
        if next > dtu {
            break;
        }
        schedule.next();
    }
}

pub trait IntoSchedule {
    type Output: Schedule;
    fn into_schedule(self) -> Self::Output;
}

impl<S: Schedule> IntoSchedule for S {
    type Output = S;

    fn into_schedule(self) -> Self::Output {
        self
    }
}

pub struct Task<R> {
    schedule: Box<dyn Schedule + Send>,
    run: Box<dyn Fn(&R) + Send>,
}

impl<R> std::fmt::Debug for Task<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct Scheduler<R> {
    pub(crate) next_up_heap: BinaryHeap<NextUp>,
    pub(crate) task_map: HashMap<TaskUid, Task<R>>,
    pub(crate) runtime: R,
}

#[derive(Debug)]
pub struct NextUp {
    key: TaskUid,
    time: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for NextUp {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl Eq for NextUp {}

impl PartialOrd for NextUp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NextUp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // the task with earlier next time has higher priority
        self.time.cmp(&other.time).reverse()
    }
}

impl<R> Scheduler<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            next_up_heap: BinaryHeap::new(),
            task_map: HashMap::new(),
            runtime,
        }
    }
    pub fn runtime(&self) -> &R {
        &self.runtime
    }
}

impl<R> Scheduler<R> {
    pub fn add_task(&mut self, key: TaskUid, mut task: Task<R>) {
        if let Some(next) = task.schedule.next() {
            let next_up = NextUp { key, time: next };
            self.task_map.insert(key, task);
            self.next_up_heap.push(next_up);
        }
    }
    pub fn delete_task(&mut self, key: TaskUid) -> Option<Task<R>> {
        self.task_map.remove(&key)
    }

    pub fn execute(&mut self) {
        let now = chrono::Utc::now();
        while let Some(peek) = self.next_up_heap.peek() {
            if peek.time > now {
                break;
            } else {
                let mut next_up = self.next_up_heap.pop().expect("should has peek");
                let Some(task) = self.task_map.get_mut(&next_up.key) else {
                    // has been deleted
                    continue;
                };
                (task.run)(&self.runtime);
                if let Some(next_call) = task.schedule.next() {
                    next_up.time = next_call;
                    self.next_up_heap.push(next_up);
                } else {
                    self.task_map.remove(&next_up.key);
                }
            }
        }
    }
}
