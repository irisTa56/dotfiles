//! Work spread over a few threads.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// How many items are worked on at once: each is a mise call, mostly waiting on the
/// network or the disk.
const THREADS: usize = 8;

/// What `work` gives for each of `items`, in their order, worked on a few at a time.
pub(super) fn map<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let results: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
    thread::scope(|scope| {
        for _ in 0..THREADS.min(items.len()) {
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
