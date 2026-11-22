//! Keeps background work off the cache while it is moved or emptied.
//!
//! Work that reads or writes the cache holds a share of the gate: each
//! thumbnail rendered, each EXIF pass, each scan. Moving the cache or
//! emptying the photo cache takes it whole. Shares are never nested (a
//! holder never asks for a second one), so a waiting move cannot deadlock
//! against them.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// The gate in front of the cache.
#[derive(Debug, Default)]
pub struct CacheGate {
    lock: RwLock<()>,
    /// Scans holding or waiting for a share.
    scans: AtomicUsize,
    /// Set while a move or an emptying waits for or holds the whole gate:
    /// long work (the EXIF pass) then stops and lets it in.
    closing: AtomicBool,
}

/// The cache is in use by a scan, which may take minutes: moving or
/// emptying it is refused rather than waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

/// A share of the gate held by a scan.
#[derive(Debug)]
pub struct ScanShare<'a> {
    _share: RwLockReadGuard<'a, ()>,
    scans: &'a AtomicUsize,
}

impl Drop for ScanShare<'_> {
    fn drop(&mut self) {
        self.scans.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The whole gate, held by a move or an emptying.
#[derive(Debug)]
pub struct Exclusive<'a> {
    _whole: RwLockWriteGuard<'a, ()>,
    closing: &'a AtomicBool,
}

impl Drop for Exclusive<'_> {
    fn drop(&mut self) {
        self.closing.store(false, Ordering::SeqCst);
    }
}

impl CacheGate {
    /// A share for a short piece of work, or one that stops once
    /// [`Self::is_closing`] says so. Waits while the cache is being moved.
    pub fn work(&self) -> RwLockReadGuard<'_, ()> {
        self.lock.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// A share for a scan, which refuses moves while it is held. Waits
    /// while the cache is being moved.
    pub fn scan(&self) -> ScanShare<'_> {
        // Counted before waiting, so a move about to start sees it.
        self.scans.fetch_add(1, Ordering::SeqCst);
        ScanShare {
            _share: self.work(),
            scans: &self.scans,
        }
    }

    /// Whether a move or an emptying is waiting for the gate: long work
    /// stops and starts again afterwards.
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::SeqCst)
    }

    /// The whole gate, once every share is given back.
    ///
    /// # Errors
    ///
    /// [`Busy`] while a scan runs.
    pub fn exclusive(&self) -> Result<Exclusive<'_>, Busy> {
        self.closing.store(true, Ordering::SeqCst);
        if self.scans.load(Ordering::SeqCst) > 0 {
            self.closing.store(false, Ordering::SeqCst);
            return Err(Busy);
        }
        Ok(Exclusive {
            _whole: self.lock.write().unwrap_or_else(PoisonError::into_inner),
            closing: &self.closing,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_running_scan_refuses_the_whole_gate() {
        let gate = CacheGate::default();
        let scan = gate.scan();

        assert_eq!(gate.exclusive().err(), Some(Busy));
        assert!(!gate.is_closing());
        drop(scan);
        assert!(gate.exclusive().is_ok());
    }

    #[test]
    fn closing_lasts_while_the_gate_is_held() {
        let gate = CacheGate::default();
        let whole = gate.exclusive().unwrap();
        assert!(gate.is_closing());
        drop(whole);
        assert!(!gate.is_closing());
    }

    #[test]
    fn work_waits_until_the_gate_is_given_back() {
        let gate = Arc::new(CacheGate::default());
        let whole = gate.exclusive().unwrap();
        let (done, finished) = mpsc::channel();
        let worker = {
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                let _share = gate.work();
                done.send(()).unwrap();
            })
        };

        assert!(finished.recv_timeout(Duration::from_millis(50)).is_err());
        drop(whole);
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn the_whole_gate_waits_for_work_in_progress() {
        let gate = Arc::new(CacheGate::default());
        let share = gate.work();
        let (taken, has_taken) = mpsc::channel();
        let mover = {
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                let _whole = gate.exclusive().unwrap();
                taken.send(()).unwrap();
            })
        };

        assert!(has_taken.recv_timeout(Duration::from_millis(50)).is_err());
        drop(share);
        has_taken.recv_timeout(Duration::from_secs(5)).unwrap();
        mover.join().unwrap();
    }
}
