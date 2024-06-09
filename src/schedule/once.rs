use crate::Dtu;

use super::{IntoSchedule, Schedule};

/// A schedule that only allows the task to run once.
pub struct Once {
    pub(crate) next: Option<Dtu>,
}

impl Once {
    pub fn new(next: Dtu) -> Self {
        Self { next: Some(next) }
    }
}

impl Schedule for Once {
    fn peek_next(&mut self) -> Option<Dtu> {
        self.next
    }

    fn next(&mut self) -> Option<Dtu> {
        self.next.take()
    }

    fn forward(&mut self, dtu: Dtu) {
        super::forward_default(self, dtu)
    }
}

impl IntoSchedule for Dtu {
    type Output = Once;
    fn into_schedule(self) -> Self::Output {
        Once::new(self)
    }
}
