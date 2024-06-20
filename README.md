# Tsuki-Scheduler
[![Crates.io Version](https://img.shields.io/crates/v/tsuki-scheduler)](https://crates.io/crates/tsuki-scheduler)
![Release status](https://github.com/4t145/tsuki-scheduler/actions/workflows/test-and-release.yml/badge.svg)
[![docs.rs](https://img.shields.io/docsrs/tsuki-scheduler)](https://docs.rs/tsuki-scheduler)

A simple, light wight, composable and extensible scheduler for every runtime.
```text
Scheduler = Schedule × Runtime
```

## Usage

This small crate can help you running tasks in 

- tokio
- async-std
- new thread
- local
- promise
- and more as long as the way to create a task in this runtime is implemented.

with a combination of

- cron schedule
- once or periodically
- after or before some time
- utc date-time iterator
- and more as long as it implement a trait `Schedule`.

For a more detailed document, check the [rust doc](https://docs.rs/tsuki-scheduler).

```shell
cargo add tsuki-scheduler
```

or 

```toml
tsuki-scheduler = "0.1"
```
### Create scheduler
```rust
use tsuki_scheduler::prelude::*;
let mut scheduler = Scheduler::new(Tokio);
let mut scheduler = Scheduler::new(AsyncStd);
let mut scheduler = Scheduler::new(Promise);
let mut scheduler = Scheduler::new(Thread);

// or you may use the async wrapper
let mut scheduler_runner = AsyncSchedulerRunner::<Tokio>::default();
let client = scheduler_runner.client();
```

### Compose schedules
What if I want to create a very complex schedule like this:

1. Firstly it will run at 10 seconds later.
2. And then, it will run at every hour's 10th minute.
3. Meanwhile, it will run every 80 minutes.
4. Though, it won't run within 30 minutes after the last run.
5. Finally, it will stop running after 100 days later.

And you can actually do it:
```rust
use tsuki_scheduler::prelude::*;
use chrono::TimeDelta;

let start_time = now() + TimeDelta::seconds(10);
let schedule = Once::new(start_time)
    .then(
        Cron::utc_from_cron_expr("00 10 * * * *")
            .expect("invalid cron")
            .or(Period::new(
                TimeDelta::minutes(80),
                start_time + TimeDelta::minutes(80),
            ))
            .throttling(TimeDelta::minutes(30)),
    )
    .before(start_time + TimeDelta::days(100));
```

### Add executes and delete tasks
```rust
use tsuki_scheduler::prelude::*;
let mut scheduler = Scheduler::new(Tokio);
let hello_tsuki_task = Task::tokio(
    Cron::local_from_cron_expr("*/2 * * * * *").unwrap(),
    || async {
        println!("Hello, tsuki!");
    },
);
let id = TaskUid::uuid();
scheduler.add_task(id, hello_tsuki_task);
scheduler.execute_by_now();
scheduler.delete_task(id);
```

### Manage the handles
You may ignore all the task handles, if you want to manage the handles, implement your own manager by implementing the trait `HandleManager`.

### Async runtime
In a async runtime, you may spawn a task for scheduler to execute periodically driven by event loop. This crate provides an implementation, you can check the [example](examples/tokio.rs) for tokio runtime.

## Feature flags
|flag|description|
|:---|:----------|
|uuid|allow to create TaskUid by uuid-v4 |
|cron|allow to create a schedule described by a cron expression |
|tokio|enable tokio runtime |
|async_std|enable async_std runtime |
|thread|enable thread runtime |
|promise|enable js promise runtime |
|async-scheduler|a default async wrapper for async runtime|


## Alternative crates
* [`tokio-cron-scheduler`](https://github.com/mvniekerk/tokio-cron-scheduler)
