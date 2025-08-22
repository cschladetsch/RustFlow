use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use crate::core::*;
use crate::generators::*;
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Factory {
    kernel: Option<Arc<RwLock<crate::kernel::Kernel>>>,
}

impl Factory {
    pub fn new() -> Self {
        Self { kernel: None }
    }
    
    pub fn with_kernel(kernel: Arc<RwLock<crate::kernel::Kernel>>) -> Self {
        Self { kernel: Some(kernel) }
    }
    
    pub fn create_group(&self) -> Arc<RwLock<dyn Group>> {
        Arc::new(RwLock::new(GroupImpl::new()))
    }
    
    pub fn create_node(&self) -> Arc<RwLock<dyn Node>> {
        Arc::new(RwLock::new(NodeImpl::new()))
    }
    
    pub fn create_sequence(&self) -> Arc<RwLock<dyn Sequence>> {
        Arc::new(RwLock::new(SequenceImpl::new()))
    }
    
    pub fn create_barrier(&self) -> Arc<RwLock<dyn Barrier>> {
        Arc::new(RwLock::new(BarrierImpl::new()))
    }
    
    pub fn create_trigger(&self) -> Arc<RwLock<dyn Trigger>> {
        Arc::new(RwLock::new(TriggerImpl::new()))
    }
    
    pub fn create_future<T: Send + Sync + Clone + 'static>(&self) -> Arc<RwLock<dyn Future<Value = T>>> {
        Arc::new(RwLock::new(FutureImpl::<T>::new()))
    }
    
    pub fn create_future_with_value<T: Send + Sync + Clone + 'static>(&self, value: T) -> Arc<RwLock<dyn Future<Value = T>>> {
        Arc::new(RwLock::new(FutureImpl::<T>::with_value(value)))
    }
    
    pub fn create_timed_future<T: Send + Sync + Clone + 'static>(&self, timeout: Duration) -> Arc<RwLock<dyn TimedFuture<Value = T>>> {
        Arc::new(RwLock::new(TimedFutureImpl::<T>::new(timeout)))
    }
    
    pub fn create_timer(&self, interval: Duration) -> Arc<RwLock<dyn Timer>> {
        Arc::new(RwLock::new(TimerImpl::new(interval)))
    }
    
    pub fn create_periodic_timer(&self, period: Duration) -> Arc<RwLock<dyn Periodic>> {
        Arc::new(RwLock::new(PeriodicImpl::new(period)))
    }
    
    pub fn create_channel<T: Send + Sync + 'static>(&self) -> Arc<RwLock<dyn Channel<T>>> {
        Arc::new(RwLock::new(ChannelImpl::<T>::new(None)))
    }
    
    pub fn create_bounded_channel<T: Send + Sync + 'static>(&self, capacity: usize) -> Arc<RwLock<dyn Channel<T>>> {
        Arc::new(RwLock::new(ChannelImpl::<T>::new(Some(capacity))))
    }
    
    pub async fn create_coroutine<F, Fut>(&self, f: F) -> Arc<RwLock<dyn Coroutine>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: StdFuture<Output = ()> + Send + 'static,
    {
        Arc::new(RwLock::new(CoroutineImpl::new(f)))
    }
    
    pub fn create_do<F>(&self, action: F) -> Arc<RwLock<dyn Generator<Output = ()>>>
    where
        F: FnOnce() + Send + 'static,
    {
        Arc::new(RwLock::new(DoImpl::new(action)))
    }
    
    pub fn create_value<T: Send + Sync + Clone + 'static>(&self, value: T) -> Arc<RwLock<dyn Generator<Output = T>>> {
        Arc::new(RwLock::new(ValueImpl::new(value)))
    }
    
    pub fn create_if<F>(&self, predicate: F, then_gen: Arc<RwLock<dyn Generator<Output = ()>>>) -> Arc<RwLock<dyn Generator<Output = ()>>>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        Arc::new(RwLock::new(IfImpl::new(predicate, then_gen)))
    }
    
    pub fn create_if_else<F>(&self, 
        predicate: F, 
        then_gen: Arc<RwLock<dyn Generator<Output = ()>>>,
        else_gen: Arc<RwLock<dyn Generator<Output = ()>>>
    ) -> Arc<RwLock<dyn Generator<Output = ()>>>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        Arc::new(RwLock::new(IfElseImpl::new(predicate, then_gen, else_gen)))
    }
    
    pub fn create_while<F>(&self, predicate: F, body: Vec<Arc<RwLock<dyn Generator<Output = ()>>>>) -> Arc<RwLock<dyn Generator<Output = ()>>>
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        Arc::new(RwLock::new(WhileImpl::new(predicate, body)))
    }
    
    pub fn create_wait(&self, duration: Duration) -> Arc<RwLock<dyn Generator<Output = ()>>> {
        Arc::new(RwLock::new(WaitImpl::new(duration)))
    }
    
    pub fn create_nop(&self) -> Arc<RwLock<dyn Generator<Output = ()>>> {
        Arc::new(RwLock::new(NopImpl::new()))
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}