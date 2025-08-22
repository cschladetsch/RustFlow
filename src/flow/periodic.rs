use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type PeriodicAction = Box<dyn Fn() + Send + Sync>;

pub struct Periodic {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    period: Duration,
    last_trigger: Option<Instant>,
    max_iterations: Option<u64>,
    current_iterations: u64,
    action: Option<PeriodicAction>,
    target: Option<Arc<RwLock<dyn Transient>>>,
}

impl Periodic {
    pub fn new(period: Duration) -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            period,
            last_trigger: None,
            max_iterations: None,
            current_iterations: 0,
            action: None,
            target: None,
        }
    }
    
    pub fn with_max_iterations(mut self, max_iterations: u64) -> Self {
        self.max_iterations = Some(max_iterations);
        self
    }
    
    pub fn with_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.action = Some(Box::new(action));
        self
    }
    
    pub fn with_target(mut self, target: Arc<RwLock<dyn Transient>>) -> Self {
        self.target = Some(target);
        self
    }
    
    pub fn period(&self) -> Duration {
        self.period
    }
    
    pub fn set_period(&mut self, period: Duration) {
        self.period = period;
    }
    
    pub fn iterations(&self) -> u64 {
        self.current_iterations
    }
    
    pub fn max_iterations(&self) -> Option<u64> {
        self.max_iterations
    }
    
    pub fn time_until_next_trigger(&self) -> Option<Duration> {
        if let Some(last_trigger) = self.last_trigger {
            let elapsed = last_trigger.elapsed();
            if elapsed < self.period {
                Some(self.period - elapsed)
            } else {
                Some(Duration::ZERO)
            }
        } else {
            Some(Duration::ZERO)
        }
    }
    
    pub fn is_ready_to_trigger(&self) -> bool {
        if let Some(last_trigger) = self.last_trigger {
            last_trigger.elapsed() >= self.period
        } else {
            true
        }
    }
    
    pub fn should_complete(&self) -> bool {
        if let Some(max_iterations) = self.max_iterations {
            self.current_iterations >= max_iterations
        } else {
            false
        }
    }
    
    pub async fn execute_periodic(&mut self) -> Result<()> {
        debug!("Periodic {} executing iteration {}", self.id, self.current_iterations + 1);
        
        self.current_iterations += 1;
        self.last_trigger = Some(Instant::now());
        
        // Execute action if present
        if let Some(ref action) = self.action {
            action();
        }
        
        // Trigger target if present
        if let Some(ref target) = self.target {
            if let Ok(mut transient) = target.try_write() {
                transient.step().await?;
            }
        }
        
        Ok(())
    }
    
    pub fn reset(&mut self) {
        self.current_iterations = 0;
        self.last_trigger = None;
        debug!("Periodic {} reset", self.id);
    }
    
    pub fn reset_timer(&mut self) {
        self.last_trigger = Some(Instant::now());
        trace!("Periodic {} timer reset", self.id);
    }
}

impl Default for Periodic {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

#[async_trait]
impl Transient for Periodic {
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
        if self.should_complete() {
            false
        } else {
            matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
        }
    }
    
    fn is_completed(&self) -> bool {
        self.should_complete() || self.state == GeneratorState::Completed
    }
    
    async fn complete(&mut self) {
        debug!("Periodic {} completing after {} iterations", self.id, self.current_iterations);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Periodic {} step #{} (iteration {}/{})", 
               self.id, self.step_number, self.current_iterations,
               self.max_iterations.map_or("∞".to_string(), |max| max.to_string()));
        
        if self.is_ready_to_trigger() {
            self.execute_periodic().await?;
            
            if self.should_complete() {
                self.complete().await;
            }
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("Periodic {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Periodic {} suspending", self.id);
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
impl Steppable for Periodic {}

#[async_trait]
impl Generator for Periodic {
    type Output = u64;
    
    fn state(&self) -> GeneratorState {
        if self.should_complete() {
            GeneratorState::Completed
        } else {
            self.state
        }
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        Some(&self.current_iterations)
    }
    
    async fn pre(&mut self) {
        trace!("Periodic {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Periodic {} post-step", self.id);
    }
}