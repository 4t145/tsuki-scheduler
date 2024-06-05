use std::{
    sync::atomic::AtomicPtr,
    time::{Duration, Instant},
};

use crate::Schedule;

pub struct Period {
    pub period: Duration,
    pub create_at: Instant,
}


