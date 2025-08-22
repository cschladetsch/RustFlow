use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Barrier {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    dependencies: Vec<Arc<RwLock<dyn Transient>>>,
}

impl Barrier {
    pub fn new() -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            dependencies: Vec::new(),
        }
    }
    
    pub async fn add_dependency(&mut self, dependency: Arc<RwLock<dyn Transient>>) {
        debug!("Barrier {} adding dependency", self.id);
        self.dependencies.push(dependency);
    }
    
    pub fn dependencies(&self) -> &[Arc<RwLock<dyn Transient>>] {
        &self.dependencies
    }
    
    pub fn all_completed(&self) -> bool {
        self.dependencies.iter().all(|dep| {
            if let Ok(transient) = dep.try_read() {
                transient.is_completed()
            } else {
                false
            }
        })
    }
}

impl Default for Barrier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transient for Barrier {
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
        debug!("Barrier {} completing", self.id);
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
impl Steppable for Barrier {
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Barrier {} step #{}", self.id, self.step_number);
        
        if self.all_completed() {
            debug!("Barrier {} all dependencies completed", self.id);
            self.complete().await;
        }
        
        Ok(())
    }
}

#[async_trait]
impl Generator for Barrier {
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
        debug!("Barrier {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Barrier {} suspending", self.id);
        self.state = GeneratorState::Suspended;
    }
    
    async fn pre(&mut self) {
        trace!("Barrier {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Barrier {} post-step", self.id);
    }
}