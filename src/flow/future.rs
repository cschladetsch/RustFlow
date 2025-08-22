use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{watch, RwLock};
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Future<T: Send + Sync + Clone + 'static> {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    value: Option<T>,
    sender: Option<watch::Sender<Option<T>>>,
    receiver: watch::Receiver<Option<T>>,
}

impl<T: Send + Sync + Clone + 'static> Future<T> {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(None);
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            value: None,
            sender: Some(sender),
            receiver,
        }
    }
    
    pub fn with_value(value: T) -> Self {
        let (sender, receiver) = watch::channel(Some(value.clone()));
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Completed,
            step_number: 0,
            value: Some(value),
            sender: Some(sender),
            receiver,
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.value.is_some()
    }
    
    pub async fn set_value(&mut self, value: T) -> Result<()> {
        debug!("Future {} setting value", self.id);
        self.value = Some(value.clone());
        
        if let Some(sender) = &self.sender {
            let _ = sender.send(Some(value));
        }
        
        self.state = GeneratorState::Completed;
        Ok(())
    }
    
    pub async fn wait(&self) -> Option<T> {
        let mut receiver = self.receiver.clone();
        while receiver.changed().await.is_ok() {
            let value = receiver.borrow().clone();
            if value.is_some() {
                return value;
            }
        }
        None
    }
}

impl<T: Send + Sync + Clone + 'static> Default for Future<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> Transient for Future<T> {
    fn id(&self) -> TransientId {
        self.id.clone()
    }
    
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    
    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
    
    fn is_active(&self) -> bool {
        matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
    }
    
    fn is_completed(&self) -> bool {
        self.state == GeneratorState::Completed
    }
    
    async fn complete(&mut self) {
        debug!("Future {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> Steppable for Future<T> {
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Future {} step #{}", self.id, self.step_number);
        
        Ok(())
    }
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> Generator for Future<T> {
    type Output = T;
    
    fn state(&self) -> GeneratorState {
        self.state
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        self.value.as_ref()
    }
    
    async fn resume(&mut self) {
        debug!("Future {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Future {} suspending", self.id);
        self.state = GeneratorState::Suspended;
    }
    
    async fn pre(&mut self) {
        trace!("Future {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Future {} post-step", self.id);
    }
}

pub struct TimedFuture<T: Send + Sync + Clone + 'static> {
    future: Future<T>,
    timeout: Duration,
    start_time: Instant,
}

impl<T: Send + Sync + Clone + 'static> TimedFuture<T> {
    pub fn new(timeout: Duration) -> Self {
        Self {
            future: Future::new(),
            timeout,
            start_time: Instant::now(),
        }
    }
    
    pub fn with_value(value: T, timeout: Duration) -> Self {
        Self {
            future: Future::with_value(value),
            timeout,
            start_time: Instant::now(),
        }
    }
    
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    
    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() >= self.timeout
    }
    
    pub async fn set_value(&mut self, value: T) -> Result<()> {
        self.future.set_value(value).await
    }
    
    pub async fn wait(&self) -> Option<T> {
        self.future.wait().await
    }
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> Transient for TimedFuture<T> {
    fn id(&self) -> TransientId {
        self.future.id()
    }
    
    fn name(&self) -> Option<&str> {
        self.future.name()
    }
    
    fn set_name(&mut self, name: String) {
        self.future.set_name(name);
    }
    
    fn is_active(&self) -> bool {
        self.future.is_active() && !self.is_timed_out()
    }
    
    fn is_completed(&self) -> bool {
        self.future.is_completed() || self.is_timed_out()
    }
    
    async fn complete(&mut self) {
        self.future.complete().await;
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> Steppable for TimedFuture<T> {
    async fn step(&mut self) -> Result<()> {
        if self.is_timed_out() && self.future.is_active() {
            debug!("TimedFuture {} timed out", self.id());
            self.future.complete().await;
        }
        
        self.future.step().await
    }
}

#[async_trait]
impl<T: Send + Sync + Clone + 'static> Generator for TimedFuture<T> {
    type Output = T;
    
    fn state(&self) -> GeneratorState {
        if self.is_timed_out() {
            GeneratorState::Completed
        } else {
            self.future.state()
        }
    }
    
    fn step_number(&self) -> u64 {
        self.future.step_number()
    }
    
    fn value(&self) -> Option<&Self::Output> {
        self.future.value()
    }
    
    async fn resume(&mut self) {
        self.future.resume().await;
    }
    
    async fn suspend(&mut self) {
        self.future.suspend().await;
    }
    
    async fn pre(&mut self) {
        self.future.pre().await;
    }
    
    async fn post(&mut self) {
        self.future.post().await;
    }
}