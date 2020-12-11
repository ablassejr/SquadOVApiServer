use async_trait::async_trait;
use crate::SquadOvError;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use std::collections::VecDeque;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait JobWorker<TData: Send + Sync> {
    fn new() -> Self;
    async fn work(&self, data: &TData) -> Result<(), SquadOvError>;
}

struct WorkerHandler<TData: Send + Sync, TWorker>
where
    TWorker: JobWorker<TData>
{
    worker: Arc<TWorker>,
    phantom: std::marker::PhantomData<TData>,
    running: Arc<AtomicBool>
}

impl<TData, TWorker> WorkerHandler<TData, TWorker>
where
    TData: Send + Sync + 'static,
    TWorker: JobWorker<TData> + Send + Sync + 'static
{
    fn new() -> Self {
        Self {
            worker: Arc::new(TWorker::new()),
            phantom: std::marker::PhantomData,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn run(&self, data: TData, queue: Arc<JobQueue<TData>>) {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);

        let running = self.running.clone();
        let worker = self.worker.clone();
        tokio::task::spawn(async move {
            match worker.work(&data).await {
                Ok(_) => (),
                Err(err) => match err {
                    SquadOvError::Defer => {
                        log::warn!("Not ready to process job - deferring.");
                        match queue.enqueue_retry(data) {
                            Ok(_) => (),
                            Err(e2) => log::error!("Failed to re-enqueue job: {:?}", e2),
                        }
                    },
                    _ => log::error!("Failed to process job: {:?}", err)
                }
            };

            running.store(false, std::sync::atomic::Ordering::Relaxed);
        });
    }
}

struct TaskWrapper<TData: Send + Sync> {
    data: TData,
    threshold: DateTime<Utc>,
}

pub struct JobQueue<TData: Send + Sync>
{
    queue: Arc<RwLock<VecDeque<TaskWrapper<TData>>>>,
}

impl<TData> JobQueue<TData>
where
    TData : Send + Sync + 'static,
{
    pub fn new<TWorker>(num_workers: i32) -> Arc<Self>
    where
        TWorker: JobWorker<TData> + Send + Sync + 'static
    {
        let queue = Arc::new(RwLock::new(VecDeque::new()));

        let mut workers: Vec<WorkerHandler<TData, TWorker>> = vec![];
        for _i in 0..num_workers {
            workers.push(WorkerHandler::new());
        }

        let workers = Arc::new(workers);

        let ret_queue = Arc::new(Self{
            queue: queue.clone(),
        });

        // This thread is the primary thread of the job queue that
        // watches for new jobs and pawns off tasks onto the workers
        // in the pool.
        let inner_queue = ret_queue.clone();
        std::thread::spawn(move || {
            // We need a tokio runtime only for this thread to handle the job worker.
            // Note that for this to work we need to use sqlx using async std and NOT a tokio runtime.
            // For whatever reason, the 1st time we call pool.begin() if sqlx is set to use a tokio runtime
            // will never return when called within this `rt` runtime. Dunno. Setting sqlx to use
            // the async-std runtime works fine though.
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                loop {
                    // Don't constantly look for a new task so that there's time for the 
                    // queue RwLock to be held by a writer.
                    async_std::task::sleep(std::time::Duration::from_millis(100)).await;
    
                    let mut q = match queue.write() {
                        Ok(x) => x,
                        Err(err) => {
                            log::error!("Failed to grab job queue lock: {:?}", err);
                            break;
                        }
                    };
    
                    // We need to assign available workers with jobs from the queue.
                    for wk in &*workers {
                        if q.is_empty() {
                            break;
                        }
    
                        if wk.running.load(std::sync::atomic::Ordering::Relaxed) {
                            continue;
                        }
    
                        let data = q.pop_front().unwrap();
                        if Utc::now() < data.threshold {
                            q.push_back(data);
                            continue;
                        }

                        wk.run(data.data, inner_queue.clone());
                    }
                }
            });
        });

        ret_queue
    }

    pub fn enqueue(&self, data: TData) -> Result<(), SquadOvError> {
        let mut q = self.queue.write()?;
        q.push_back(TaskWrapper{
            data,
            threshold: Utc::now(),
        });
        Ok(())
    }

    pub fn enqueue_retry(&self, data: TData) -> Result<(), SquadOvError> {
        let mut q = self.queue.write()?;
        q.push_back(TaskWrapper{
            data,
            threshold: Utc::now() + chrono::Duration::seconds(1),
        });
        Ok(())
    }
}