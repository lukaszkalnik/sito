//! A small thread pool that runs jobs in the order they were queued.
//!
//! The UI decides which thumbnails to request and when (visible tiles top to
//! bottom, a few at a time), so the pool keeps that order.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Shared {
    jobs: Mutex<VecDeque<Job>>,
    ready: Condvar,
}

/// Worker threads taking jobs in the order they were queued.
pub struct WorkerPool {
    shared: Arc<Shared>,
}

impl fmt::Debug for WorkerPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkerPool").finish_non_exhaustive()
    }
}

impl WorkerPool {
    /// Starts `workers` threads named `<name>-<n>`; they live as long as the app.
    ///
    /// # Errors
    ///
    /// Returns an error if a thread cannot be started.
    pub fn new(workers: usize, name: &str) -> std::io::Result<Self> {
        let shared = Arc::new(Shared::default());
        for n in 0..workers {
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name(format!("{name}-{n}"))
                .spawn(move || work(&shared))?;
        }
        Ok(Self { shared })
    }

    /// Queues `job`; it runs after every job queued earlier.
    pub fn spawn(&self, job: impl FnOnce() + Send + 'static) {
        self.shared
            .jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(Box::new(job));
        self.shared.ready.notify_one();
    }
}

fn work(shared: &Shared) {
    loop {
        let job = {
            let mut jobs = shared.jobs.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some(job) = jobs.pop_front() {
                    break job;
                }
                jobs = shared
                    .ready
                    .wait(jobs)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        job();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn runs_waiting_jobs_in_the_order_they_were_queued() {
        let pool = WorkerPool::new(1, "test").unwrap();
        let (gate_open, gate) = mpsc::channel::<()>();
        let (started, has_started) = mpsc::channel::<()>();
        // Keep the only worker busy while the other jobs are queued.
        pool.spawn(move || {
            started.send(()).unwrap();
            gate.recv().unwrap();
        });
        has_started.recv().unwrap();

        let order = Arc::new(Mutex::new(Vec::new()));
        let (done, all_done) = mpsc::channel::<()>();
        for n in 1..=3 {
            let order = Arc::clone(&order);
            let done = done.clone();
            pool.spawn(move || {
                order.lock().unwrap().push(n);
                done.send(()).unwrap();
            });
        }
        gate_open.send(()).unwrap();
        for _ in 0..3 {
            all_done.recv_timeout(Duration::from_secs(5)).unwrap();
        }

        assert_eq!(*order.lock().unwrap(), [1, 2, 3]);
    }

    #[test]
    fn several_workers_run_every_job() {
        let pool = WorkerPool::new(2, "test").unwrap();
        let (done, all_done) = mpsc::channel::<usize>();
        for n in 0..50 {
            let done = done.clone();
            pool.spawn(move || done.send(n).unwrap());
        }

        let mut seen: Vec<usize> = (0..50)
            .map(|_| all_done.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect();
        seen.sort_unstable();

        assert_eq!(seen, (0..50).collect::<Vec<_>>());
    }
}
