use super::Schedule;
use crate::Dtu;

/// A schedule that never runs. You may use it as a unit element for [`or`](`super::ScheduleExt::or`) combinator,
/// or to init a [`builder`](super::ScheduleDynBuilder).
#[derive(Debug, Clone, Copy, Default)]
pub struct Never;

impl Schedule for Never {
    fn peek_next(&mut self) -> Option<Dtu> {
        None
    }

    fn next(&mut self) -> Option<Dtu> {
        None
    }

    fn forward_to(&mut self, _dtu: Dtu) {}
}
