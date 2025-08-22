use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type TimeoutAction = Box<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimeoutBehavior {
    Complete,    // Complete the timeout when expired
    Cancel,      // Cancel the target when expired
    Retry,       // Retry the target when expired
    Custom,      // Execute custom action when expired
}

pub struct AdvancedTimeout {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    duration: Duration,
    start_time: Instant,
    behavior: TimeoutBehavior,
    max_retries: Option<u32>,
    current_retry: u32,
    retry_delay: Duration,
    last_retry_time: Option<Instant>,
    target: Option<Arc<RwLock<dyn Transient>>>,
    on_timeout_action: Option<TimeoutAction>,
    on_retry_action: Option<TimeoutAction>,
    has_timed_out: bool,
}

impl AdvancedTimeout {
    pub fn new(duration: Duration, behavior: TimeoutBehavior) -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            duration,
            start_time: Instant::now(),
            behavior,
            max_retries: None,
            current_retry: 0,
            retry_delay: Duration::from_millis(100),
            last_retry_time: None,
            target: None,
            on_timeout_action: None,
            on_retry_action: None,
            has_timed_out: false,
        }
    }
    
    pub fn complete_on_timeout(duration: Duration) -> Self {
        Self::new(duration, TimeoutBehavior::Complete)
    }
    
    pub fn cancel_on_timeout(duration: Duration) -> Self {
        Self::new(duration, TimeoutBehavior::Cancel)
    }
    
    pub fn retry_on_timeout(duration: Duration, max_retries: u32, retry_delay: Duration) -> Self {
        let mut timeout = Self::new(duration, TimeoutBehavior::Retry);
        timeout.max_retries = Some(max_retries);
        timeout.retry_delay = retry_delay;
        timeout
    }
    
    pub fn custom_on_timeout(duration: Duration) -> Self {
        Self::new(duration, TimeoutBehavior::Custom)
    }
    
    pub fn with_target(mut self, target: Arc<RwLock<dyn Transient>>) -> Self {
        self.target = Some(target);
        self
    }
    
    pub fn with_timeout_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_timeout_action = Some(Box::new(action));
        self
    }
    
    pub fn with_retry_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_retry_action = Some(Box::new(action));
        self
    }
    
    pub fn duration(&self) -> Duration {
        self.duration
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn remaining(&self) -> Duration {
        self.duration.saturating_sub(self.elapsed())
    }
    
    pub fn is_expired(&self) -> bool {
        self.elapsed() >= self.duration
    }
    
    pub fn retry_count(&self) -> u32 {
        self.current_retry
    }
    
    pub fn max_retries(&self) -> Option<u32> {
        self.max_retries
    }
    
    pub fn can_retry(&self) -> bool {
        if let Some(max_retries) = self.max_retries {
            self.current_retry < max_retries
        } else {
            false
        }
    }
    
    pub fn should_retry(&self) -> bool {
        if let Some(last_retry) = self.last_retry_time {
            last_retry.elapsed() >= self.retry_delay
        } else {
            true
        }
    }
    
    pub async fn execute_timeout(&mut self) -> Result<()> {
        if self.has_timed_out {
            return Ok(());
        }
        
        self.has_timed_out = true;
        warn!("Timeout {} expired after {}ms", self.id, self.duration.as_millis());
        
        match self.behavior {
            TimeoutBehavior::Complete => {
                debug!("Timeout {} completing", self.id);
                self.complete().await;
            }
            TimeoutBehavior::Cancel => {
                if let Some(ref target) = self.target {
                    debug!("Timeout {} canceling target", self.id);
                    if let Ok(mut transient) = target.try_write() {
                        transient.complete().await;
                    }
                }
            }
            TimeoutBehavior::Retry => {
                if self.can_retry() {
                    debug!("Timeout {} initiating retry {}/{}", self.id, self.current_retry + 1, 
                           self.max_retries.unwrap_or(0));
                    self.execute_retry().await?;
                } else {
                    debug!("Timeout {} exhausted retries, completing", self.id);
                    self.complete().await;
                }
            }
            TimeoutBehavior::Custom => {
                if let Some(ref action) = self.on_timeout_action {
                    debug!("Timeout {} executing custom action", self.id);
                    action();
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn execute_retry(&mut self) -> Result<()> {
        if !self.can_retry() || !self.should_retry() {
            return Ok(());
        }
        
        self.current_retry += 1;
        self.last_retry_time = Some(Instant::now());
        self.has_timed_out = false;
        self.start_time = Instant::now(); // Reset timeout for retry
        
        if let Some(ref action) = self.on_retry_action {
            action();
        }
        
        if let Some(ref target) = self.target {
            if let Ok(mut transient) = target.try_write() {
                // Reset target state for retry
                transient.resume().await;
            }
        }
        
        debug!("Timeout {} retry {} initiated", self.id, self.current_retry);
        Ok(())
    }
    
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.current_retry = 0;
        self.last_retry_time = None;
        self.has_timed_out = false;
        debug!("Timeout {} reset", self.id);
    }
    
    pub fn extend(&mut self, additional_time: Duration) {
        self.duration += additional_time;
        debug!("Timeout {} extended by {}ms (total: {}ms)", 
               self.id, additional_time.as_millis(), self.duration.as_millis());
    }
}

impl Default for AdvancedTimeout {
    fn default() -> Self {
        Self::new(Duration::from_secs(30), TimeoutBehavior::Complete)
    }
}

#[async_trait]
impl Transient for AdvancedTimeout {
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
        if self.behavior == TimeoutBehavior::Retry {
            // For retry behavior, stay active until max retries exhausted
            if self.is_expired() && !self.can_retry() {
                false
            } else {
                matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
            }
        } else {
            // For other behaviors, inactive when expired
            if self.is_expired() && self.has_timed_out {
                false
            } else {
                matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
            }
        }
    }
    
    fn is_completed(&self) -> bool {
        if self.behavior == TimeoutBehavior::Retry {
            self.state == GeneratorState::Completed || (self.is_expired() && !self.can_retry())
        } else {
            self.state == GeneratorState::Completed || (self.is_expired() && self.has_timed_out)
        }
    }
    
    async fn complete(&mut self) {
        debug!("Timeout {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Timeout {} step #{} ({}ms remaining)", 
               self.id, self.step_number, self.remaining().as_millis());
        
        // Check for retry condition
        if self.behavior == TimeoutBehavior::Retry && self.is_expired() && self.can_retry() && self.should_retry() {
            self.execute_retry().await?;
            return Ok(());
        }
        
        // Handle timeout
        if self.is_expired() && !self.has_timed_out {
            self.execute_timeout().await?;
            return Ok(());
        }
        
        // Step target if present and not expired
        if !self.is_expired() {
            let should_complete = if let Some(ref target) = self.target {
                if let Ok(mut transient) = target.try_write() {
                    if transient.is_completed() {
                        true
                    } else {
                        transient.step().await?;
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            
            if should_complete {
                debug!("Timeout {} target completed, completing timeout", self.id);
                self.complete().await;
            }
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("Timeout {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Timeout {} suspending", self.id);
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
impl Steppable for AdvancedTimeout {}

#[async_trait]
impl Generator for AdvancedTimeout {
    type Output = bool;
    
    fn state(&self) -> GeneratorState {
        if self.is_completed() {
            GeneratorState::Completed
        } else {
            self.state
        }
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        static TRUE: bool = true;
        static FALSE: bool = false;
        if self.is_expired() { 
            Some(&TRUE) 
        } else { 
            Some(&FALSE) 
        }
    }
    
    async fn pre(&mut self) {
        trace!("Timeout {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Timeout {} post-step", self.id);
    }
}