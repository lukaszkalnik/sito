//! A small thread pool that runs the newest job first.
//!
//! Thumbnails are requested for every tile that stays on screen briefly, so a
//! fast scroll queues many renders for photos already scrolled past. Running
//! the newest first renders what is on screen now before those.

use std::fmt;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Shared {
    jobs: Mutex<Vec<Job>>,
    ready: Condvar,
}

/// Worker threads taking the most recently added job first.
pub struct LifoPool {
    shared: Arc<Shared>,
}

impl fmt::Debug for LifoPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LifoPool").finish_non_exhaustive()
    }
}

impl LifoPool {
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

    /// Queues `job`; it runs before every job queued earlier and not started yet.
    pub fn spawn(&self, job: impl FnOnce() + Send + 'static) {
        self.shared
            .jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Box::new(job));
        self.shared.ready.notify_one();
    }
}

fn work(shared: &Shared) {
    loop {
        let job = {
            let mut jobs = shared.jobs.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some(job) = jobs.pop() {
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
    fn runs_the_newest_waiting_job_first() {
        let pool = LifoPool::new(1, "test").unwrap();
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

        assert_eq!(*order.lock().unwrap(), [3, 2, 1]);
    }

    #[test]
    fn several_workers_run_every_job() {
        let pool = LifoPool::new(2, "test").unwrap();
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
