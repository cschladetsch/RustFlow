use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::core::{Generator, Group, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct GroupImpl {
    id: TransientId,
    name: Option<String>,
    contents: Vec<Arc<RwLock<dyn Transient>>>,
    state: GeneratorState,
    step_number: u64,
}

impl GroupImpl {
    pub fn new() -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            contents: Vec::new(),
            state: GeneratorState::Running,
            step_number: 0,
        }
    }
}

#[async_trait]
impl Transient for GroupImpl {
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
        debug!("Group {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[async_trait]
impl Steppable for GroupImpl {
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Group {} step #{}", self.id, self.step_number);
        
        Ok(())
    }
}

#[async_trait]
impl Generator for GroupImpl {
    type Output = ();
    
    fn state(&self) -> GeneratorState {
        self.state
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        Some(&())
    }
    
    async fn resume(&mut self) {
        debug!("Group {} resuming", self.id);
        self.state = GeneratorState::Running;
        
        for content in &self.contents {
            if let Ok(mut transient) = content.try_write() {
                if let Some(generator) = transient.as_any_mut().downcast_mut::<dyn Generator<Output = dyn Send + Sync>>() {
                    generator.resume().await;
                }
            }
        }
    }
    
    async fn suspend(&mut self) {
        debug!("Group {} suspending", self.id);
        self.state = GeneratorState::Suspended;
        
        for content in &self.contents {
            if let Ok(mut transient) = content.try_write() {
                if let Some(generator) = transient.as_any_mut().downcast_mut::<dyn Generator<Output = dyn Send + Sync>>() {
                    generator.suspend().await;
                }
            }
        }
    }
    
    async fn pre(&mut self) {
        trace!("Group {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Group {} post-step", self.id);
    }
}

#[async_trait]
impl Group for GroupImpl {
    fn contents(&self) -> &[Arc<RwLock<dyn Transient>>] {
        &self.contents
    }
    
    fn generators(&self) -> Vec<Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>> {
        Vec::new() // This would need proper downcasting in a real implementation
    }
    
    fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }
    
    async fn add_transient(&mut self, transient: Arc<RwLock<dyn Transient>>) {
        debug!("Group {} adding transient", self.id);
        self.contents.push(transient.clone());
        self.on_added(&transient).await;
    }
    
    async fn add_transients(&mut self, transients: Vec<Arc<RwLock<dyn Transient>>>) {
        for transient in transients {
            self.add_transient(transient).await;
        }
    }
    
    async fn remove_transient(&mut self, id: TransientId) -> bool {
        if let Some(pos) = self.contents.iter().position(|t| {
            if let Ok(transient) = t.try_read() {
                transient.id().value() == id.value()
            } else {
                false
            }
        }) {
            let transient = self.contents.remove(pos);
            self.on_removed(&transient).await;
            debug!("Group {} removed transient {}", self.id, id);
            true
        } else {
            false
        }
    }
    
    async fn on_added(&mut self, transient: &Arc<RwLock<dyn Transient>>) {
        if let Ok(t) = transient.try_read() {
            trace!("Group {} transient {} added", self.id, t.id());
        }
    }
    
    async fn on_removed(&mut self, transient: &Arc<RwLock<dyn Transient>>) {
        if let Ok(t) = transient.try_read() {
            trace!("Group {} transient {} removed", self.id, t.id());
        }
    }
}