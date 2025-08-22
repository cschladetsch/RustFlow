use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, error, info};

use crate::kernel::Kernel;
use crate::traits::Transient;
use crate::Result;

pub struct Runtime {
    kernel: Arc<RwLock<Kernel>>,
    rt_handle: tokio::runtime::Handle,
}

impl Runtime {
    pub fn new() -> Self {
        let kernel = Arc::new(RwLock::new(Kernel::new()));
        let rt_handle = tokio::runtime::Handle::current();
        
        Self {
            kernel,
            rt_handle,
        }
    }
    
    pub fn kernel(&self) -> Arc<RwLock<Kernel>> {
        Arc::clone(&self.kernel)
    }
    
    pub async fn run_frame(&self, delta_time: Duration) -> Result<()> {
        let mut kernel = self.kernel.write().await;
        kernel.update(delta_time).await
    }
    
    pub async fn run_for(&self, duration: Duration, frame_time: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        
        info!("Running RustFlow for {:?} with frame time {:?}", duration, frame_time);
        
        while start.elapsed() < duration {
            let frame_start = std::time::Instant::now();
            
            if let Err(e) = self.run_frame(frame_time).await {
                error!("Frame error: {}", e);
            }
            
            let frame_elapsed = frame_start.elapsed();
            if frame_elapsed < frame_time {
                time::sleep(frame_time - frame_elapsed).await;
            }
        }
        
        Ok(())
    }
    
    pub async fn run_until_complete(&self, frame_time: Duration) -> Result<()> {
        info!("Running RustFlow until completion with frame time {:?}", frame_time);
        
        loop {
            let frame_start = std::time::Instant::now();
            
            if let Err(e) = self.run_frame(frame_time).await {
                error!("Frame error: {}", e);
                break;
            }
            
            {
                let kernel = self.kernel.read().await;
                if kernel.is_break() || kernel.is_completed() {
                    debug!("Runtime stopping: break={}, completed={}", 
                           kernel.is_break(), kernel.is_completed());
                    break;
                }
            }
            
            let frame_elapsed = frame_start.elapsed();
            if frame_elapsed < frame_time {
                time::sleep(frame_time - frame_elapsed).await;
            }
        }
        
        Ok(())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}