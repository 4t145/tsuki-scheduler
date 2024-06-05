use std::{
    collections::{BinaryHeap, HashMap},
    future::Future,
    hash::Hash,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    time::Instant,
};

pub trait Schedule {
    fn next(&self) -> Option<std::time::Instant>;
    fn merge<S>(self, schedule: S) -> MergedSchedule<Self, S>
    where
        Self: Sized,
    {
        MergedSchedule(self, schedule)
    }
}

pub struct MergedSchedule<S0, S1>(S0, S1);
impl<S0, S1> Schedule for MergedSchedule<S0, S1>
where
    S0: Schedule,
    S1: Schedule,
{
    fn next(&self) -> Option<std::time::Instant> {
        match (self.0.next(), self.1.next()) {
            (None, None) => None,
            (None, Some(next)) => Some(next),
            (Some(next), None) => Some(next),
            (Some(next_0), Some(next_1)) => Some(next_0.min(next_1)),
        }
    }
}

pub trait Timer {
    fn tick(&self) -> impl Future<Output = ()>;
}
type TaskRun = dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>;
pub struct Task {
    schedule: Box<dyn Schedule>,
    run: Box<TaskRun>,
}

pub trait Runtime {
    fn execute(&self, task: &TaskRun);
}

pub struct Scheduler<K> {
    pub next: Option<Instant>,
    pub next_up_qu: BinaryHeap<NextUp<Rc<K>>>,
    pub task: HashMap<Rc<K>, Task>,
    pub runtime: Arc<dyn Runtime>,
}

pub struct NextUp<K> {
    key: K,
    time: Instant,
}
impl<K> PartialEq for NextUp<K> {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl<K> Eq for NextUp<K> {}

impl<K> PartialOrd for NextUp<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<K> Ord for NextUp<K> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}
impl<K> Scheduler<K>
where
    K: Eq + Ord + Hash,
{
    pub fn add_task(&mut self, key: K, task: Task)
    where
        K: Clone,
    {
        if let Some(next) = task.schedule.next() {
            let key = Rc::new(key);
            let next_up = NextUp {
                key: key.clone(),
                time: next,
            };
            self.task.insert(key, task);
            self.next_up_qu.push(next_up);
        }
    }
    pub fn delete_task(&mut self, key: &K) -> Option<Task> {
        self.task.remove(key)
    }

    pub fn execute(&mut self) {
        while let Some(peek) = self.next_up_qu.peek() {
            if peek.time.elapsed().is_zero() {
                self.next = Some(peek.time);
                break;
            } else {
                let mut next_up = self.next_up_qu.pop().expect("should has peek");
                let Some(task) = self.task.get(&next_up.key) else {
                    // has been deleted
                    continue;
                };
                self.runtime.execute(&task.run);
                if let Some(next_call) = task.schedule.next() {
                    next_up.time = next_call;
                    self.next_up_qu.push(next_up);
                }
            }
        }
    }
}
