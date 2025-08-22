use async_trait::async_trait;
use std::any::Any;
use std::time::{Duration, Instant};
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Timer {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    interval: Duration,
    start_time: Instant,
}

impl Timer {
    pub fn new(interval: Duration) -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            interval,
            start_time: Instant::now(),
        }
    }
    
    pub fn interval(&self) -> Duration {
        self.interval
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn is_expired(&self) -> bool {
        self.elapsed() >= self.interval
    }
}

#[async_trait]
impl Transient for Timer {
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
        matches!(self.state, GeneratorState::Running | GeneratorState::Suspended) && !self.is_expired()
    }
    
    fn is_completed(&self) -> bool {
        self.state == GeneratorState::Completed || self.is_expired()
    }
    
    async fn complete(&mut self) {
        debug!("Timer {} completing", self.id);
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
impl Steppable for Timer {
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Timer {} step #{}", self.id, self.step_number);
        
        if self.is_expired() {
            debug!("Timer {} expired", self.id);
            self.complete().await;
        }
        
        Ok(())
    }
}

#[async_trait]
impl Generator for Timer {
    type Output = ();
    
    fn state(&self) -> GeneratorState {
        if self.is_expired() {
            GeneratorState::Completed
        } else {
            self.state
        }
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        Some(&())
    }
    
    async fn resume(&mut self) {
        debug!("Timer {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Timer {} suspending", self.id);
        self.state = GeneratorState::Suspended;
    }
    
    async fn pre(&mut self) {
        trace!("Timer {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Timer {} post-step", self.id);
    }
}