use async_trait::async_trait;
use std::any::Any;
use tokio::sync::mpsc;
use tracing::debug;

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::{FlowError, Result};

pub struct Channel<T: Send + Sync + 'static> {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    sender: mpsc::UnboundedSender<T>,
    receiver: mpsc::UnboundedReceiver<T>,
    last_value: Option<T>,
}

impl<T: Send + Sync + 'static> Channel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            sender,
            receiver,
            last_value: None,
        }
    }
    
    pub async fn send(&self, item: T) -> Result<()> {
        self.sender.send(item).map_err(|_| FlowError::Send)
    }
    
    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> Transient for Channel<T> {
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
        debug!("Channel {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        
        if let Ok(value) = self.receiver.try_recv() {
            self.last_value = Some(value);
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        self.state = GeneratorState::Suspended;
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> Steppable for Channel<T> {}

#[async_trait]
impl<T: Send + Sync + 'static> Generator for Channel<T> {
    type Output = Option<T>;
    
    fn state(&self) -> GeneratorState {
        self.state
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        Some(&self.last_value)
    }
    
    async fn pre(&mut self) {}
    
    async fn post(&mut self) {}
}