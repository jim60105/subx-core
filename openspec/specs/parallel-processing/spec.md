# Parallel Processing

## Purpose

Define the core parallel-execution machinery: the `TaskScheduler` entry point and its boxed-task collection contract, bounded concurrency, non-blocking I/O in the async executor, active task accounting correctness, and UUIDv7 worker and task identifiers. The `subx-cli` repository carries the same-named `parallel-processing` capability holding the command-surface half: aggregated result reporting, progress opt-out, batch task submission, queue overflow strategy, and optional task priority ordering. Implemented in `src/core/parallel/`.

## Requirements

### Requirement: Task Scheduler Entry Point

The system SHALL expose `TaskScheduler::new()` as the primary entry point for batch execution, and the scheduler SHALL accept a collection of boxed `Task + Send + Sync` values and return a corresponding `Vec<TaskResult>`. The task-count reporting and the empty-input early return of `subx-cli`'s match command are specified by the `parallel-processing` capability's *Parallel Match Reports Task Count and Handles an Empty Input Set* requirement in `subx-cli`.

#### Scenario: Parallel match over a directory
- **GIVEN** a directory containing N video files and `subx-cli`'s match command using the parallel execution path
- **WHEN** the scheduler runs the generated `FileProcessingTask` set
- **THEN** each video SHALL be processed by the scheduler

### Requirement: Bounded Concurrency

The system SHALL limit the number of concurrently running tasks to the active worker count configured from `config.parallel`, preventing unbounded task fan-out.

#### Scenario: Concurrency is reported
- **GIVEN** the parallel matcher has been initialized
- **WHEN** the scheduler begins running tasks
- **THEN** it SHALL expose the active worker count via `scheduler.get_active_workers()` and SHALL not run more tasks simultaneously than that count

### Requirement: Batch Task Submission

The system SHALL expose `TaskScheduler::submit_batch_tasks` (or equivalent) that accepts a collection of boxed `Task + Send + Sync` values and returns one `TaskResult` per submitted task preserving order, enabling batch file-processing workflows such as `FileProcessingTask` validation and conversion. Exercised by `tests/parallel_processing_integration_tests.rs`.

#### Scenario: Batch returns one result per task
- **GIVEN** a scheduler built via `TaskScheduler::new_with_defaults()` and `N` boxed `FileProcessingTask` values
- **WHEN** `scheduler.submit_batch_tasks(tasks)` is awaited
- **THEN** the returned `Vec<TaskResult>` SHALL have length `N` and successful tasks SHALL be reported as `TaskResult::Success(_)`

### Requirement: Task Queue Overflow Strategy

When the scheduler's internal task queue has reached `parallel.task_queue_size` and a new task is submitted, the system SHALL apply the configured `parallel.overflow_strategy` before enqueueing the task, supporting the following variants with these exact semantics. Implemented in `src/core/parallel/scheduler.rs::submit_task_with_priority`.

- `Block` — the call SHALL wait (polling at ~10 ms intervals) until queue space becomes available, then enqueue the new task.
- `DropOldest` — the scheduler SHALL pop the oldest pending task from the front of the queue and then enqueue the new task.
- `Reject` — the call SHALL return `SubXError::parallel_processing("Task queue is full")` without enqueueing.
- `Drop` — the call SHALL return `TaskResult::Failed("Task dropped due to queue overflow")` without enqueueing.
- `Expand` — the scheduler SHALL enqueue the new task even though the queue size exceeds `task_queue_size`.

#### Scenario: Reject strategy returns an error when full
- **GIVEN** `parallel.overflow_strategy = Reject`, the queue already holds `task_queue_size` tasks, and a new task is submitted
- **WHEN** `submit_task_with_priority` executes
- **THEN** it SHALL return a `SubXError::parallel_processing` error whose message contains `Task queue is full` and the new task SHALL NOT be enqueued

#### Scenario: DropOldest replaces the oldest pending task
- **GIVEN** `parallel.overflow_strategy = DropOldest`, the queue is full, and a new task is submitted
- **WHEN** `submit_task_with_priority` executes
- **THEN** the oldest pending task SHALL be removed from the queue and the new task SHALL be enqueued

### Requirement: Optional Task Priority Ordering

When `parallel.enable_task_priorities` is `true`, the scheduler SHALL insert each incoming task ahead of all pending tasks with a strictly lower `TaskPriority` (defined in `src/core/parallel/scheduler.rs` with variants `Low`, `Normal`, `High`, `Critical`), and the worker SHALL dispatch the highest-priority pending task first. When `enable_task_priorities` is `false`, tasks SHALL be enqueued in FIFO order regardless of their declared priority.

#### Scenario: High-priority task runs before earlier normal tasks
- **GIVEN** `parallel.enable_task_priorities = true`, a queue holding one `TaskPriority::Normal` task, and a new `TaskPriority::High` task submitted afterwards
- **WHEN** a worker becomes available to dispatch the next task
- **THEN** the `High` task SHALL be dispatched before the earlier `Normal` task

#### Scenario: Priorities ignored when the flag is disabled
- **GIVEN** `parallel.enable_task_priorities = false` and tasks submitted in the order `Low`, `Critical`, `Normal`
- **WHEN** a worker dispatches tasks
- **THEN** the tasks SHALL be dispatched in the order they were submitted (`Low`, `Critical`, `Normal`)

### Requirement: Non-blocking I/O in async executor

All blocking filesystem operations within async task execution functions SHALL be offloaded to the tokio blocking thread pool via `spawn_blocking`. Direct `std::fs` calls within `async fn` bodies SHALL NOT occur, because they would stall the tokio runtime and starve other concurrent tasks.

#### Scenario: file copy in async context
- **WHEN** an async task executor copies a file
- **THEN** the blocking I/O SHALL be wrapped in `spawn_blocking` and SHALL NOT block the tokio runtime

### Requirement: Active task accounting correctness

The scheduler SHALL maintain accurate `active_tasks` state across all code paths including normal completion, overflow rejection, overflow dropping, and oldest-task eviction. An RAII guard pattern SHALL ensure that `active_tasks` entries are removed when the task's processing scope ends, regardless of the exit path.

#### Scenario: overflow-rejected task cleanup
- **WHEN** a task submission is rejected due to queue overflow
- **THEN** the task's `active_tasks` entry SHALL be removed before the error is returned

#### Scenario: overflow-dropped task cleanup
- **WHEN** a task is dropped due to overflow strategy
- **THEN** the task's `active_tasks` entry SHALL be removed

#### Scenario: evicted task notification
- **WHEN** the oldest task is evicted by `DropOldest`
- **THEN** a `TaskResult::Failed` with a descriptive message SHALL be sent to the evicted task's channel, and its `active_tasks` entry SHALL be cleaned up

#### Scenario: normal completion cleanup
- **WHEN** a task completes normally
- **THEN** its `active_tasks` entry SHALL be removed

### Requirement: UUIDv7 Worker and Task Identifiers

The parallel processing subsystem SHALL identify workers and tasks with UUIDv7 values produced through the `uuid` crate's `Uuid::now_v7()` constructor (or, where strict 1ms monotonicity across IDs is required, through `crate::core::uuidv7::Uuidv7Generator`). The system SHALL NOT call `Uuid::new_v4()` from `src/core/parallel/worker.rs` or `src/core/parallel/scheduler.rs`, and the `uuid` crate's `v4` feature SHALL NOT be enabled in this crate's `Cargo.toml` (the manifest that declares `uuid`; the `subx-cli` superproject manifest does not).

#### Scenario: Worker identifier is UUIDv7

- **WHEN** `Worker::new()` constructs a fresh worker
- **THEN** the worker's `id` field SHALL be a `Uuid` whose version nibble equals `7`

#### Scenario: WorkerPool::execute assigns UUIDv7 to dispatched workers

- **WHEN** `WorkerPool::execute` enrolls a new worker for an incoming task
- **THEN** the `worker_id` recorded in the internal `workers` map SHALL be a `Uuid` whose version nibble equals `7`

#### Scenario: Task identifier in test harnesses uses UUIDv7

- **WHEN** the in-tree `CounterTask::task_id` (used by scheduler unit tests in `src/core/parallel/scheduler.rs`) is invoked
- **THEN** the returned string SHALL parse as a `Uuid` whose version nibble equals `7`

#### Scenario: uuid v4 feature is disabled

- **WHEN** `subx-core`'s own `Cargo.toml` is inspected
- **THEN** the `uuid` dependency entry SHALL list `features = ["v7"]` and SHALL NOT include `"v4"`

