use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::flow::*;
use crate::traits::Transient;

pub struct Factory;

impl Factory {
    pub fn new() -> Self {
        Self
    }
    
    pub fn group(&self) -> Arc<RwLock<group::Group>> {
        Arc::new(RwLock::new(group::Group::new()))
    }
    
    pub fn node(&self) -> Arc<RwLock<node::Node>> {
        Arc::new(RwLock::new(node::Node::new()))
    }
    
    pub fn sequence(&self) -> Arc<RwLock<sequence::Sequence>> {
        Arc::new(RwLock::new(sequence::Sequence::new()))
    }
    
    pub fn sequence_with_steps(&self, steps: Vec<Arc<RwLock<dyn Transient>>>) -> Arc<RwLock<sequence::Sequence>> {
        Arc::new(RwLock::new(sequence::Sequence::with_steps(steps)))
    }
    
    pub fn barrier(&self) -> Arc<RwLock<barrier::Barrier>> {
        Arc::new(RwLock::new(barrier::Barrier::new()))
    }
    
    pub fn future<T: Send + Sync + Clone + 'static>(&self) -> Arc<RwLock<future::Future<T>>> {
        Arc::new(RwLock::new(future::Future::new()))
    }
    
    pub fn future_with_value<T: Send + Sync + Clone + 'static>(&self, value: T) -> Arc<RwLock<future::Future<T>>> {
        Arc::new(RwLock::new(future::Future::with_value(value)))
    }
    
    pub fn timed_future<T: Send + Sync + Clone + 'static>(&self, timeout: Duration) -> Arc<RwLock<future::TimedFuture<T>>> {
        Arc::new(RwLock::new(future::TimedFuture::new(timeout)))
    }
    
    pub fn timer(&self, interval: Duration) -> Arc<RwLock<timer::Timer>> {
        Arc::new(RwLock::new(timer::Timer::new(interval)))
    }
    
    pub fn channel<T: Send + Sync + 'static>(&self) -> Arc<RwLock<channel::Channel<T>>> {
        Arc::new(RwLock::new(channel::Channel::new()))
    }
    
    pub fn coroutine(&self) -> Arc<RwLock<coroutine::Coroutine>> {
        Arc::new(RwLock::new(coroutine::Coroutine::new()))
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}