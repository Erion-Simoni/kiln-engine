//! Task pool for background jobs and async asset loading.
//!
//! With the `threads` feature on native targets, jobs run on a fixed worker
//! pool. Without `threads`, or on unsupported targets, jobs run on the caller.

use std::sync::mpsc;
#[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
use std::sync::{Arc, Mutex};

/// Handle to a background job result.
pub struct JobHandle<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> JobHandle<T> {
    /// Block until the job finishes.
    pub fn join(self) -> T {
        self.rx.recv().expect("job worker disconnected")
    }

    /// Poll for completion without blocking.
    pub fn try_join(&self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

/// Pool of worker threads (or a single-thread fallback).
pub struct TaskPool {
    #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
    inner: NativePool,
}

#[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
struct NativePool {
    sender: Mutex<mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>>,
    _workers: Vec<std::thread::JoinHandle<()>>,
}

impl TaskPool {
    /// Create a pool with `num_threads` workers (clamped to at least 1).
    ///
    /// When `threads` is disabled, `num_threads` is ignored and work runs inline.
    pub fn new(num_threads: usize) -> Self {
        #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
        {
            let n = num_threads.max(1);
            let (tx, rx) = mpsc::channel::<Box<dyn FnOnce() + Send + 'static>>();
            let rx = Arc::new(Mutex::new(rx));
            let mut workers = Vec::with_capacity(n);
            for i in 0..n {
                let rx = Arc::clone(&rx);
                let handle = std::thread::Builder::new()
                    .name(format!("kiln-job-{i}"))
                    .spawn(move || loop {
                        let job = {
                            let guard = rx.lock().unwrap();
                            guard.recv()
                        };
                        match job {
                            Ok(f) => f(),
                            Err(_) => break,
                        }
                    })
                    .expect("failed to spawn kiln job worker");
                workers.push(handle);
            }
            return Self {
                inner: NativePool {
                    sender: Mutex::new(tx),
                    _workers: workers,
                },
            };
        }

        #[cfg(not(all(feature = "threads", not(target_arch = "wasm32"))))]
        {
            let _ = num_threads;
            Self {}
        }
    }

    /// Pool sized to available parallelism (or 1 when threads are off).
    pub fn with_default_size() -> Self {
        let n = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self::new(n)
    }

    /// Number of worker threads (1 when running inline).
    pub fn thread_count(&self) -> usize {
        #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
        {
            self.inner._workers.len()
        }
        #[cfg(not(all(feature = "threads", not(target_arch = "wasm32"))))]
        {
            1
        }
    }

    /// Whether this pool can run work off the calling thread.
    pub fn is_parallel(&self) -> bool {
        #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
        {
            true
        }
        #[cfg(not(all(feature = "threads", not(target_arch = "wasm32"))))]
        {
            false
        }
    }

    /// Spawn a job that returns `T`.
    pub fn spawn<T, F>(&self, f: F) -> JobHandle<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();

        #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
        {
            let job = Box::new(move || {
                let value = f();
                let _ = tx.send(value);
            });
            self.inner
                .sender
                .lock()
                .unwrap()
                .send(job)
                .expect("job pool shut down");
        }

        #[cfg(not(all(feature = "threads", not(target_arch = "wasm32"))))]
        {
            let value = f();
            let _ = tx.send(value);
        }

        JobHandle { rx }
    }

    /// Run `f` over each chunk of `items` (parallel when threads are on).
    pub fn scope_chunks<T, F>(&self, items: &mut [T], chunk_size: usize, f: F)
    where
        T: Send,
        F: Fn(&mut [T]) + Sync,
    {
        if items.is_empty() {
            return;
        }
        let chunk_size = chunk_size.max(1);

        #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
        {
            std::thread::scope(|s| {
                for chunk in items.chunks_mut(chunk_size) {
                    let f = &f;
                    s.spawn(move || f(chunk));
                }
            });
            return;
        }

        #[cfg(not(all(feature = "threads", not(target_arch = "wasm32"))))]
        {
            for chunk in items.chunks_mut(chunk_size) {
                f(chunk);
            }
        }
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::with_default_size()
    }
}

/// Load bytes from a path on a background job (async asset loading helper).
pub fn load_file_async(pool: &TaskPool, path: impl Into<std::path::PathBuf>) -> JobHandle<std::io::Result<Vec<u8>>> {
    let path = path.into();
    pool.spawn(move || std::fs::read(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_returns_value() {
        let pool = TaskPool::new(2);
        let handle = pool.spawn(|| 21 + 21);
        assert_eq!(handle.join(), 42);
    }

    #[test]
    fn scope_chunks_covers_all() {
        let pool = TaskPool::new(4);
        let mut data: Vec<u32> = (0..100).collect();
        pool.scope_chunks(&mut data, 16, |chunk| {
            for v in chunk.iter_mut() {
                *v *= 2;
            }
        });
        assert_eq!(data[0], 0);
        assert_eq!(data[50], 100);
        assert_eq!(data[99], 198);
    }

    #[test]
    fn try_join_eventually_ready() {
        let pool = TaskPool::new(1);
        let handle = pool.spawn(|| 7u32);
        let mut value = None;
        for _ in 0..1000 {
            if let Some(v) = handle.try_join() {
                value = Some(v);
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(value, Some(7));
    }
}
