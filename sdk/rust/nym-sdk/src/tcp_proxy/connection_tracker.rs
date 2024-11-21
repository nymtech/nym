use anyhow::{bail, Result};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// Used by the ProxyClient for tracking # of active clients created on new TCP connection
// TODO used by the connection pool for maintaining # of clients in pool +/-
#[derive(Debug)]
pub struct ConnectionTracker {
    count: Arc<AtomicUsize>,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement(&self) -> Result<()> {
        if self.get_count() == 0 {
            bail!("count already 0");
        }
        self.count.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn get_count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl Clone for ConnectionTracker {
    fn clone(&self) -> Self {
        Self {
            count: Arc::clone(&self.count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::thread;

    #[test]
    fn test_increment_decrement() -> Result<()> {
        let tracker = ConnectionTracker::new();
        tracker.increment();
        tracker.increment();
        assert_eq!(tracker.get_count(), 2, "should be 2 after single increment");
        tracker.decrement()?;
        assert_eq!(
            tracker.get_count(),
            1,
            "should be 1 after two increments and one decrement"
        );
        Ok(())
    }

    #[test]
    fn test_clone() {
        let tracker = ConnectionTracker::new();
        let tracker_clone = tracker.clone();

        tracker.increment();
        assert_eq!(
            tracker_clone.get_count(),
            1,
            "tracker clones should share the same count"
        );
    }

    #[test]
    fn test_multiple_threads() {
        let tracker = ConnectionTracker::new();
        let mut handles = vec![];

        for _ in 0..10 {
            let thread_tracker = tracker.clone();
            let handle = thread::spawn(move || {
                thread_tracker.increment();
                thread::sleep(std::time::Duration::from_millis(10));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            tracker.get_count(),
            10,
            "should be 10 after 10 thread increments"
        );
    }

    #[test]
    fn test_concurrent_increment_decrement() -> Result<()> {
        let tracker = ConnectionTracker::new();
        let mut handles = vec![];

        for i in 0..10 {
            let thread_tracker = tracker.clone();
            let handle = thread::spawn(move || {
                if i < 5 {
                    thread_tracker.increment();
                } else {
                    thread_tracker.decrement().unwrap();
                }
                thread::sleep(std::time::Duration::from_millis(10));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            tracker.get_count(),
            0,
            "should be 0 after equal increments and decrements"
        );
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_zero_floor() {
        let tracker = ConnectionTracker::new();
        tracker.decrement().unwrap();
    }

    #[test]
    fn test_stress() {
        let tracker = ConnectionTracker::new();
        let mut handles = vec![];
        let num_threads = 100;

        for _ in 0..num_threads {
            let thread_tracker = tracker.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    thread_tracker.increment();
                    thread::sleep(std::time::Duration::from_micros(1));
                    thread_tracker.decrement().unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            tracker.get_count(),
            0,
            "should return to 0 after all increments and decrements"
        );
    }
}
