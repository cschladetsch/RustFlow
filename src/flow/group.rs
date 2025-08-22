use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Group {
    id: TransientId,
    name: Option<String>,
    contents: Vec<Arc<RwLock<dyn Transient>>>,
    state: GeneratorState,
    step_number: u64,
}

impl Group {
    pub fn new() -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            contents: Vec::new(),
            state: GeneratorState::Running,
            step_number: 0,
        }
    }
    
    pub fn contents(&self) -> &[Arc<RwLock<dyn Transient>>] {
        &self.contents
    }
    
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }
    
    pub async fn add(&mut self, transient: Arc<RwLock<dyn Transient>>) {
        debug!("Group {} adding transient", self.id);
        self.contents.push(transient);
    }
    
    pub async fn add_many(&mut self, transients: Vec<Arc<RwLock<dyn Transient>>>) {
        for transient in transients {
            self.add(transient).await;
        }
    }
    
    pub async fn remove(&mut self, id: TransientId) -> bool {
        if let Some(pos) = self.contents.iter().position(|t| {
            if let Ok(transient) = t.try_read() {
                transient.id().value() == id.value()
            } else {
                false
            }
        }) {
            self.contents.remove(pos);
            debug!("Group {} removed transient {}", self.id, id);
            true
        } else {
            false
        }
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transient for Group {
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
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl Steppable for Group {
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
impl Generator for Group {
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
    }
    
    async fn suspend(&mut self) {
        debug!("Group {} suspending", self.id);
        self.state = GeneratorState::Suspended;
    }
    
    async fn pre(&mut self) {
        trace!("Group {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Group {} post-step", self.id);
    }
}