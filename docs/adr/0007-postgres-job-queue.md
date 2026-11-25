# ADR-0007: Solver jobs are queued in PostgreSQL, not Redis

Status: Accepted (2026-11-25). Changes the plan for roadmap milestone 4, which named a Redis queue.

## Decision
The `solver_jobs` table is the queue. A worker claims the oldest queued job with
`SELECT ... FOR UPDATE SKIP LOCKED` and marks it `running` under its worker id. While it
solves, a heartbeat thread writes `heartbeat_at` every 2 s and reads `cancel_requested`. Any
worker requeues a running job whose heartbeat is older than 30 s, so it runs again from the
start. After 3 starts the job fails instead, with the error `interrupted`. A cancel is the
database flag, so it reaches whichever process runs the job.

Workers run inside the API process (`RIVERLAB_WORKER_MODE=embedded`, the default) or as separate
processes (`python -m riverlab_api.worker`, with the API in `external` mode). Several workers
can share one database: the row lock keeps two of them from claiming the same job.

## Why not Redis
- RiverLab already depends on PostgreSQL, and the job's state already lives there. A second
  store would have to be kept consistent with it: a job enqueued in Redis but not committed in
  Postgres, or the reverse.
- Users run RiverLab on a laptop or an Android phone (docs/android.md). Another server to
  install, start and keep alive is a real cost there.
- Solves last seconds to hours and arrive a few at a time. Polling every 2 s, with an in-process
  wake-up on submit, is plenty. Redis's latency and throughput would buy nothing here.

## Consequences
- Jobs survive restarts and crashes, with no lost work beyond the interrupted solve itself. The
  solve restarts from scratch; GTOpen has no checkpointing.
- After a restart, an interrupted job waits up to the 30 s staleness window before it is
  requeued.
- Result files are still written to `RIVERLAB_RESULT_STORAGE_DIR`. External workers on other
  machines must share that directory. S3-compatible result storage remains future work.
- A cloud backend can use the same queue: a remote worker needs only database access and the
  result storage.
