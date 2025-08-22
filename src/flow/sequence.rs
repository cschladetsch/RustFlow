use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Sequence {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    steps: Vec<Arc<RwLock<dyn Transient>>>,
    current_step: usize,
}

impl Sequence {
    pub fn new() -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            steps: Vec::new(),
            current_step: 0,
        }
    }
    
    pub fn with_steps(steps: Vec<Arc<RwLock<dyn Transient>>>) -> Self {
        let state = if steps.is_empty() {
            GeneratorState::Completed
        } else {
            GeneratorState::Running
        };
        
        Self {
            id: TransientId::new(),
            name: None,
            state,
            step_number: 0,
            steps,
            current_step: 0,
        }
    }
    
    pub async fn add_step(&mut self, step: Arc<RwLock<dyn Transient>>) {
        debug!("Sequence {} adding step", self.id);
        self.steps.push(step);
        
        if self.state == GeneratorState::Completed && self.steps.len() == 1 {
            self.state = GeneratorState::Running;
            self.current_step = 0;
        }
    }
    
    pub fn current_step_index(&self) -> Option<usize> {
        if self.current_step < self.steps.len() {
            Some(self.current_step)
        } else {
            None
        }
    }
    
    pub fn steps(&self) -> &[Arc<RwLock<dyn Transient>>] {
        &self.steps
    }
    
    pub fn current_step(&self) -> Option<&Arc<RwLock<dyn Transient>>> {
        self.steps.get(self.current_step)
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transient for Sequence {
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
        debug!("Sequence {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Sequence {} step #{} (step {}/{})", 
               self.id, self.step_number, 
               self.current_step + 1, self.steps.len());
        
        if self.current_step >= self.steps.len() {
            self.complete().await;
            return Ok(());
        }
        
        let current = &self.steps[self.current_step];
        let mut should_advance = false;
        
        if let Ok(mut transient) = current.try_write() {
            if transient.is_completed() {
                should_advance = true;
            } else {
                // Simplified stepping - just step the transient
                if let Err(e) = transient.step().await {
                    debug!("Error stepping sequence step {}: {}", self.current_step, e);
                }
            }
        }
        
        if should_advance {
            self.current_step += 1;
            debug!("Sequence {} advancing to step {}", self.id, self.current_step + 1);
            
            if self.current_step >= self.steps.len() {
                self.complete().await;
            }
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("Sequence {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Sequence {} suspending", self.id);
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
impl Steppable for Sequence {}

#[async_trait]
impl Generator for Sequence {
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
    
    async fn pre(&mut self) {
        trace!("Sequence {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Sequence {} post-step", self.id);
    }
}