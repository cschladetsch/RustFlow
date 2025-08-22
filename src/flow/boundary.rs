use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type GuardCondition = Box<dyn Fn() -> bool + Send + Sync>;
pub type BoundaryAction = Box<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoundaryType {
    Guard,       // Prevents execution while condition is false
    RateLimit,   // Limits execution frequency
    Circuit,     // Circuit breaker pattern
    Timeout,     // Time-based boundary
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Blocking all requests
    HalfOpen,    // Testing if service recovered
}

pub struct Boundary {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    boundary_type: BoundaryType,
    
    // Guard fields
    guard_condition: Option<GuardCondition>,
    
    // Rate limit fields
    rate_limit: Option<Duration>,
    last_execution: Option<Instant>,
    
    // Circuit breaker fields
    circuit_state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    recovery_timeout: Duration,
    last_failure_time: Option<Instant>,
    success_count: u32,
    success_threshold: u32,
    
    // Timeout fields
    timeout_duration: Option<Duration>,
    start_time: Option<Instant>,
    
    // Target and actions
    target: Option<Arc<RwLock<dyn Transient>>>,
    on_block_action: Option<BoundaryAction>,
    on_allow_action: Option<BoundaryAction>,
}

impl Boundary {
    pub fn new(boundary_type: BoundaryType) -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            boundary_type,
            guard_condition: None,
            rate_limit: None,
            last_execution: None,
            circuit_state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            last_failure_time: None,
            success_count: 0,
            success_threshold: 3,
            timeout_duration: None,
            start_time: None,
            target: None,
            on_block_action: None,
            on_allow_action: None,
        }
    }
    
    pub fn guard() -> Self {
        Self::new(BoundaryType::Guard)
    }
    
    pub fn rate_limit(interval: Duration) -> Self {
        let mut boundary = Self::new(BoundaryType::RateLimit);
        boundary.rate_limit = Some(interval);
        boundary
    }
    
    pub fn circuit_breaker(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        let mut boundary = Self::new(BoundaryType::Circuit);
        boundary.failure_threshold = failure_threshold;
        boundary.recovery_timeout = recovery_timeout;
        boundary
    }
    
    pub fn timeout(duration: Duration) -> Self {
        let mut boundary = Self::new(BoundaryType::Timeout);
        boundary.timeout_duration = Some(duration);
        boundary.start_time = Some(Instant::now());
        boundary
    }
    
    pub fn with_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.guard_condition = Some(Box::new(condition));
        self
    }
    
    pub fn with_target(mut self, target: Arc<RwLock<dyn Transient>>) -> Self {
        self.target = Some(target);
        self
    }
    
    pub fn with_block_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_block_action = Some(Box::new(action));
        self
    }
    
    pub fn with_allow_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_allow_action = Some(Box::new(action));
        self
    }
    
    pub async fn check_boundary(&mut self) -> bool {
        match self.boundary_type {
            BoundaryType::Guard => self.check_guard(),
            BoundaryType::RateLimit => self.check_rate_limit(),
            BoundaryType::Circuit => self.check_circuit(),
            BoundaryType::Timeout => self.check_timeout(),
        }
    }
    
    fn check_guard(&self) -> bool {
        if let Some(ref condition) = self.guard_condition {
            condition()
        } else {
            true
        }
    }
    
    fn check_rate_limit(&mut self) -> bool {
        if let Some(rate_limit) = self.rate_limit {
            if let Some(last_execution) = self.last_execution {
                if last_execution.elapsed() < rate_limit {
                    return false;
                }
            }
            self.last_execution = Some(Instant::now());
            true
        } else {
            true
        }
    }
    
    fn check_circuit(&mut self) -> bool {
        let now = Instant::now();
        
        match self.circuit_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if now.duration_since(last_failure) >= self.recovery_timeout {
                        debug!("Circuit breaker {} transitioning to half-open", self.id);
                        self.circuit_state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    fn check_timeout(&self) -> bool {
        if let (Some(timeout), Some(start_time)) = (self.timeout_duration, self.start_time) {
            start_time.elapsed() < timeout
        } else {
            true
        }
    }
    
    pub async fn record_success(&mut self) {
        if self.boundary_type == BoundaryType::Circuit {
            match self.circuit_state {
                CircuitState::HalfOpen => {
                    self.success_count += 1;
                    if self.success_count >= self.success_threshold {
                        debug!("Circuit breaker {} closing after successful recovery", self.id);
                        self.circuit_state = CircuitState::Closed;
                        self.failure_count = 0;
                    }
                }
                CircuitState::Closed => {
                    self.failure_count = 0;
                }
                _ => {}
            }
        }
    }
    
    pub async fn record_failure(&mut self) {
        if self.boundary_type == BoundaryType::Circuit {
            self.failure_count += 1;
            self.last_failure_time = Some(Instant::now());
            
            match self.circuit_state {
                CircuitState::Closed => {
                    if self.failure_count >= self.failure_threshold {
                        warn!("Circuit breaker {} opening due to {} failures", self.id, self.failure_count);
                        self.circuit_state = CircuitState::Open;
                    }
                }
                CircuitState::HalfOpen => {
                    warn!("Circuit breaker {} re-opening due to failure during recovery", self.id);
                    self.circuit_state = CircuitState::Open;
                    self.success_count = 0;
                }
                _ => {}
            }
        }
    }
    
    pub fn is_timeout_expired(&self) -> bool {
        if self.boundary_type == BoundaryType::Timeout {
            !self.check_timeout()
        } else {
            false
        }
    }
    
    pub fn circuit_state(&self) -> CircuitState {
        self.circuit_state
    }
    
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }
}

impl Default for Boundary {
    fn default() -> Self {
        Self::new(BoundaryType::Guard)
    }
}

#[async_trait]
impl Transient for Boundary {
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
        if self.boundary_type == BoundaryType::Timeout && self.is_timeout_expired() {
            false
        } else {
            matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
        }
    }
    
    fn is_completed(&self) -> bool {
        if self.boundary_type == BoundaryType::Timeout && self.is_timeout_expired() {
            true
        } else {
            self.state == GeneratorState::Completed
        }
    }
    
    async fn complete(&mut self) {
        debug!("Boundary {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Boundary {} step #{} (type: {:?})", self.id, self.step_number, self.boundary_type);
        
        let allowed = self.check_boundary().await;
        
        if allowed {
            trace!("Boundary {} allowing execution", self.id);
            
            if let Some(ref action) = self.on_allow_action {
                action();
            }
            
            let step_result = if let Some(ref target) = self.target {
                if let Ok(mut transient) = target.try_write() {
                    transient.step().await
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            };
            
            match step_result {
                Ok(_) => {
                    self.record_success().await;
                }
                Err(_) => {
                    self.record_failure().await;
                }
            }
        } else {
            trace!("Boundary {} blocking execution", self.id);
            
            if let Some(ref action) = self.on_block_action {
                action();
            }
        }
        
        if self.boundary_type == BoundaryType::Timeout && self.is_timeout_expired() {
            self.complete().await;
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("Boundary {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Boundary {} suspending", self.id);
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
impl Steppable for Boundary {}

#[async_trait]
impl Generator for Boundary {
    type Output = bool;
    
    fn state(&self) -> GeneratorState {
        self.state
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        // Return whether boundary is currently allowing execution
        static TRUE: bool = true;
        static FALSE: bool = false;
        match self.boundary_type {
            BoundaryType::Circuit => Some(match self.circuit_state {
                CircuitState::Closed | CircuitState::HalfOpen => &TRUE,
                CircuitState::Open => &FALSE,
            }),
            BoundaryType::Timeout => {
                if self.is_timeout_expired() { Some(&FALSE) } else { Some(&TRUE) }
            },
            _ => Some(&TRUE),
        }
    }
    
    async fn pre(&mut self) {
        trace!("Boundary {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Boundary {} post-step", self.id);
    }
}