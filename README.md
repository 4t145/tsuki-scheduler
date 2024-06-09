# Tsuki-Scheduler
A simple, light wight, composable and extensible scheduler for every runtime.
```
Scheduler = Schedule × Runtime
```

## Usage

This small crate can help you running tasks in 

- tokio
- async-std
- thread
- wasm (javascript promise)
- and more as long as the way to create a task in this runtime is implemented.

with a combination of

- cron schedule
- once or periodically
- after or before some time
- utc date-time iterator
- and more as long as it implement a trait `Schedule`.

For a more detailed document, check the [rust doc](https://docs.rs/tsuki-scheduler).

### Create scheduler
```rust
use tsuki_scheduler::prelude::*;
let mut scheduler = Scheduler::new(Tokio);
let mut scheduler = Scheduler::new(AsyncStd);
let mut scheduler = Scheduler::new(Wasm);
let mut scheduler = Scheduler::new(Thread);
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
In a async runtime, you may spawn a task for scheduler to execute periodically driven by event loop, you can check the [example](examples/tokio.rs) for tokio runtime.

## Feature flags
|flag|description|
|:---|:----------|
|uuid|allow to create TaskUid by uuid-v4 |
|cron|allow to create a schedule described by a cron expression |
|tokio|enable tokio runtime |
|async_std|enable async_std runtime |
|thread|enable thread runtime |
|wasm|enable wasm runtime |
