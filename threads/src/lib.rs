use std::sync::mpsc::{self, channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct PoolHandle<O> {
    receiver: Receiver<O>,
}

impl<O> PoolHandle<O> {
    pub fn recv(self) -> Vec<O> {
        self.receiver.iter().collect()
    }

    pub fn recv_all(handles: Vec<PoolHandle<O>>) -> Vec<O> {
        let mut out = Vec::new();

        for handle in handles {
            for value in handle.recv() {
                out.push(value);
            }
        }

        out
    }
}

pub trait Pool {
    fn get_max_threads(&self) -> usize;

    fn single<T: Send + 'static, O: Send + 'static>(
        jobs: Vec<T>,
        func: impl Fn(T) -> O + Send + Sync + 'static,
    ) -> PoolHandle<O>;

    fn multiple<T: Send + 'static, O: Send + 'static>(
        jobs: Vec<T>,
        func: impl Fn(T) -> O + Send + Sync + 'static,
        num_threads: usize,
    ) -> Vec<PoolHandle<O>>;
}

pub struct ThreadPool(usize);
impl ThreadPool {
    pub fn new(threads: usize) -> Self {
        Self(threads)
    }
}

impl Pool for ThreadPool {
    fn get_max_threads(&self) -> usize {
        self.0
    }

    fn single<T: Send + 'static, O: Send + 'static>(
        jobs: Vec<T>,
        func: impl Fn(T) -> O + Send + Sync + 'static,
    ) -> PoolHandle<O> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for job in jobs {
                tx.send(func(job)).unwrap();
            }
        });

        PoolHandle { receiver: rx }
    }

    fn multiple<T: Send + 'static, O: Send + 'static>(
        jobs: Vec<T>,
        func: impl Fn(T) -> O + Send + Sync + 'static,
        num_threads: usize,
    ) -> Vec<PoolHandle<O>> {
        let func = Arc::new(func);
        let job_queue = Arc::new(Mutex::new(jobs)); // Shared queue

        let mut handles: Vec<PoolHandle<O>> = Vec::new();
        for _ in 0..num_threads {
            let (tx, rx) = channel();

            let func = Arc::clone(&func);
            let job_queue = Arc::clone(&job_queue);

            thread::spawn(move || {
                while let Some(job) = job_queue.lock().unwrap().pop() {
                    let result = func(job);
                    tx.send(result).unwrap();
                }
            });

            handles.push(PoolHandle { receiver: rx });
        }

        handles
    }
}
