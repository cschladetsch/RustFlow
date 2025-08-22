use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type TriggerCondition = Box<dyn Fn() -> bool + Send + Sync>;
pub type TriggerAction = Box<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    Once,      // Trigger only once when condition becomes true
    Repeating, // Trigger every time condition becomes true
    Edge,      // Trigger on state change (false -> true)
}

pub struct Trigger {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    condition: Option<TriggerCondition>,
    action: Option<TriggerAction>,
    trigger_type: TriggerType,
    has_triggered: bool,
    last_condition_state: bool,
    target: Option<Arc<RwLock<dyn Transient>>>,
}

impl Trigger {
    pub fn new() -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            condition: None,
            action: None,
            trigger_type: TriggerType::Once,
            has_triggered: false,
            last_condition_state: false,
            target: None,
        }
    }
    
    pub fn with_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Box::new(condition));
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
    
    pub fn set_type(mut self, trigger_type: TriggerType) -> Self {
        self.trigger_type = trigger_type;
        self
    }
    
    pub fn once() -> Self {
        Self::new().set_type(TriggerType::Once)
    }
    
    pub fn repeating() -> Self {
        Self::new().set_type(TriggerType::Repeating)
    }
    
    pub fn edge() -> Self {
        Self::new().set_type(TriggerType::Edge)
    }
    
    pub fn should_trigger(&mut self) -> bool {
        if let Some(ref condition) = self.condition {
            let current_state = condition();
            
            let should_trigger = match self.trigger_type {
                TriggerType::Once => current_state && !self.has_triggered,
                TriggerType::Repeating => current_state,
                TriggerType::Edge => current_state && !self.last_condition_state,
            };
            
            self.last_condition_state = current_state;
            should_trigger
        } else {
            false
        }
    }
    
    pub async fn execute_trigger(&mut self) -> Result<()> {
        debug!("Trigger {} executing", self.id);
        self.has_triggered = true;
        
        // Execute action if present
        if let Some(ref action) = self.action {
            action();
        }
        
        // Trigger target if present
        if let Some(ref target) = self.target {
            if let Ok(mut transient) = target.try_write() {
                transient.resume().await;
            }
        }
        
        Ok(())
    }
    
    pub fn reset(&mut self) {
        self.has_triggered = false;
        self.last_condition_state = false;
        debug!("Trigger {} reset", self.id);
    }
    
    pub fn has_triggered(&self) -> bool {
        self.has_triggered
    }
    
    pub fn trigger_type(&self) -> TriggerType {
        self.trigger_type
    }
}

impl Default for Trigger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transient for Trigger {
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
        match self.trigger_type {
            TriggerType::Once => matches!(self.state, GeneratorState::Running | GeneratorState::Suspended) && !self.has_triggered,
            _ => matches!(self.state, GeneratorState::Running | GeneratorState::Suspended),
        }
    }
    
    fn is_completed(&self) -> bool {
        match self.trigger_type {
            TriggerType::Once => self.state == GeneratorState::Completed || self.has_triggered,
            _ => self.state == GeneratorState::Completed,
        }
    }
    
    async fn complete(&mut self) {
        debug!("Trigger {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("Trigger {} step #{}", self.id, self.step_number);
        
        if self.should_trigger() {
            self.execute_trigger().await?;
            
            if self.trigger_type == TriggerType::Once {
                self.complete().await;
            }
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("Trigger {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("Trigger {} suspending", self.id);
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
impl Steppable for Trigger {}

#[async_trait]
impl Generator for Trigger {
    type Output = bool;
    
    fn state(&self) -> GeneratorState {
        self.state
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        Some(&self.has_triggered)
    }
    
    async fn pre(&mut self) {
        trace!("Trigger {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("Trigger {} post-step", self.id);
    }
}