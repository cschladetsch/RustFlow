use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use crate::types::{DebugLevel, GeneratorState, TimeFrame, TransientId};
use crate::Result;

#[async_trait]
pub trait Transient: Send + Sync {
    fn id(&self) -> TransientId;
    fn name(&self) -> Option<&str>;
    fn set_name(&mut self, name: String);
    
    fn is_active(&self) -> bool;
    fn is_completed(&self) -> bool;
    
    async fn complete(&mut self);
    
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[async_trait]
pub trait Steppable: Transient {
    async fn step(&mut self) -> Result<()>;
}

#[async_trait]
pub trait Generator: Steppable {
    type Output: Send + Sync;
    
    fn state(&self) -> GeneratorState;
    fn step_number(&self) -> u64;
    fn value(&self) -> Option<&Self::Output>;
    
    async fn resume(&mut self);
    async fn suspend(&mut self);
    async fn pre(&mut self);
    async fn post(&mut self);
    
    fn is_running(&self) -> bool {
        self.state() == GeneratorState::Running
    }
}

#[async_trait]
pub trait Group: Generator<Output = ()> {
    fn contents(&self) -> &[Arc<RwLock<dyn Transient>>];
    fn generators(&self) -> Vec<Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>>;
    fn is_empty(&self) -> bool;
    
    async fn add_transient(&mut self, transient: Arc<RwLock<dyn Transient>>);
    async fn add_transients(&mut self, transients: Vec<Arc<RwLock<dyn Transient>>>);
    async fn remove_transient(&mut self, id: TransientId) -> bool;
    
    async fn on_added(&mut self, _transient: &Arc<RwLock<dyn Transient>>) {}
    async fn on_removed(&mut self, _transient: &Arc<RwLock<dyn Transient>>) {}
}

#[async_trait]
pub trait Node: Group {
    async fn add_generator(&mut self, generator: Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>);
    async fn add_generators(&mut self, generators: Vec<Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>>);
}

#[async_trait]
pub trait Coroutine: Generator<Output = ()> {
    type Future: std::future::Future<Output = ()> + Send;
    
    fn create_future(&self) -> Self::Future;
}

#[async_trait]
pub trait Future: Generator<Output = Self::Value> {
    type Value: Send + Sync + Clone;
    
    fn is_ready(&self) -> bool;
    async fn set_value(&mut self, value: Self::Value);
    async fn wait(&self) -> Self::Value;
}

#[async_trait]
pub trait TimedFuture: Future {
    fn timeout(&self) -> Duration;
    fn is_timed_out(&self) -> bool;
}

#[async_trait]
pub trait Barrier: Generator<Output = ()> {
    async fn add_dependency(&mut self, dependency: Arc<RwLock<dyn Transient>>);
    async fn remove_dependency(&mut self, id: TransientId) -> bool;
    fn dependencies(&self) -> &[Arc<RwLock<dyn Transient>>];
    fn all_completed(&self) -> bool;
}

#[async_trait]
pub trait Trigger: Generator<Output = ()> {
    async fn add_dependency(&mut self, dependency: Arc<RwLock<dyn Transient>>);
    async fn remove_dependency(&mut self, id: TransientId) -> bool;
    fn dependencies(&self) -> &[Arc<RwLock<dyn Transient>>];
    fn any_completed(&self) -> bool;
}

#[async_trait]
pub trait Sequence: Generator<Output = ()> {
    async fn add_step(&mut self, step: Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>);
    fn current_step(&self) -> Option<usize>;
    fn steps(&self) -> &[Arc<RwLock<dyn Generator<Output = dyn Send + Sync>>>];
}

#[async_trait]
pub trait Timer: Generator<Output = ()> {
    fn interval(&self) -> Duration;
    fn elapsed(&self) -> Duration;
    fn is_expired(&self) -> bool;
}

#[async_trait]
pub trait Periodic: Timer {
    fn period(&self) -> Duration;
    fn tick_count(&self) -> u64;
}

#[async_trait]
pub trait Channel<T: Send + Sync>: Generator<Output = Option<T>> {
    async fn send(&self, item: T) -> Result<()>;
    async fn recv(&mut self) -> Option<T>;
    fn capacity(&self) -> Option<usize>;
    fn is_closed(&self) -> bool;
    async fn close(&mut self);
}

pub trait Named {
    fn named(self, name: String) -> Self where Self: Sized;
}