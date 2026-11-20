//! Work spread over a few threads.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// A few threads that work through a list of items together.
pub(super) struct Pool {
    threads: usize,
}

impl Pool {
    /// The pool for mise calls, which mostly wait on the network or the disk.
    pub(super) fn for_mise() -> Self {
        Pool { threads: 8 }
    }

    /// What `work` gives for each of `items`, in their order.
    pub(super) fn map<T: Sync, R: Send>(
        &self,
        items: &[T],
        work: impl Fn(&T) -> R + Sync,
    ) -> Vec<R> {
        let next = AtomicUsize::new(0);
        let results: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
        thread::scope(|scope| {
            for _ in 0..self.threads.min(items.len()) {
                scope.spawn(|| {
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else {
                            return;
                        };
                        let result = work(item);
                        *results[index]
                            .lock()
                            .expect("a result's lock is not poisoned") = Some(result);
                    }
                });
            }
        });
        results
            .into_iter()
            .map(|result| {
                result
                    .into_inner()
                    .expect("a result's lock is not poisoned")
                    .expect("every item is worked on")
            })
            .collect()
    }
}
