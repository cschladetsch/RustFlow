use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crossbeam::channel;
use parking_lot::RwLock;
use tracing::{debug, error, info};

use crate::Result;

pub struct ThreadedExecutor {
    thread_count: usize,
    handles: Vec<thread::JoinHandle<()>>,
    shutdown_sender: Option<channel::Sender<()>>,
}

impl ThreadedExecutor {
    pub fn new(thread_count: usize) -> Self {
        Self {
            thread_count,
            handles: Vec::new(),
            shutdown_sender: None,
        }
    }
    
    pub fn start<F>(&mut self, work_fn: F) -> Result<()> 
    where
        F: Fn(usize) + Send + Sync + Clone + 'static,
    {
        let (shutdown_sender, shutdown_receiver) = channel::unbounded();
        self.shutdown_sender = Some(shutdown_sender);
        
        info!("Starting threaded executor with {} threads", self.thread_count);
        
        for i in 0..self.thread_count {
            let work_fn = work_fn.clone();
            let shutdown_receiver = shutdown_receiver.clone();
            
            let handle = thread::spawn(move || {
                debug!("Worker thread {} starting", i);
                
                loop {
                    if shutdown_receiver.try_recv().is_ok() {
                        debug!("Worker thread {} shutting down", i);
                        break;
                    }
                    
                    work_fn(i);
                    
                    thread::sleep(Duration::from_millis(1));
                }
                
                debug!("Worker thread {} finished", i);
            });
            
            self.handles.push(handle);
        }
        
        Ok(())
    }
    
    pub fn shutdown(mut self) -> Result<()> {
        info!("Shutting down threaded executor");
        
        if let Some(sender) = self.shutdown_sender.take() {
            for _ in 0..self.thread_count {
                let _ = sender.send(());
            }
        }
        
        for handle in self.handles {
            if let Err(e) = handle.join() {
                error!("Error joining thread: {:?}", e);
            }
        }
        
        info!("Threaded executor shutdown complete");
        Ok(())
    }
}

pub struct ThreadPool {
    inner: Arc<RwLock<ThreadedExecutor>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ThreadedExecutor::new(size))),
        }
    }
    
    pub fn execute<F>(&self, task: F) -> Result<()>
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(task);
        Ok(())
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new(num_cpus::get())
    }
}

#[cfg(feature = "num_cpus")]
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(not(feature = "num_cpus"))]
mod num_cpus {
    pub fn get() -> usize {
        4
    }
}