use std::{
    sync::Arc,
    thread::{spawn, JoinHandle},
};

pub struct ThreadPool {
    pub max_threads: usize,
}
impl ThreadPool {
    pub fn execute<T, O>(
        &self,
        jobs: &Vec<T>,
        func: impl Fn(&T) -> O + Send + Sync + 'static,
    ) -> Vec<O>
    where
        T: Send + Sync + Clone + 'static,
        O: Send + 'static,
    {
        let func = Arc::new(func);
        let batches = jobs
            .chunks(self.max_threads)
            .map(|v| v.to_vec())
            .collect::<Vec<_>>();

        let mut results = Vec::with_capacity(jobs.len());

        for batch in batches {
            let mut handles: Vec<JoinHandle<O>> = Vec::new();

            for v in batch {
                let func = Arc::clone(&func);
                handles.push(spawn(move || func(&v)));
            }

            for handle in handles {
                results.push(handle.join().expect("Thread failed"));
            }
        }

        results
    }
}
