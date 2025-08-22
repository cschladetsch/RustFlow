use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type TimedTriggerCondition = Box<dyn Fn() -> bool + Send + Sync>;
pub type TimedTriggerAction = Box<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimedTriggerType {
    Once,           // Trigger once within timeout
    Repeating,      // Trigger repeatedly within timeout window
    WindowStart,    // Trigger at start of time window
    WindowEnd,      // Trigger at end of time window  
    Debounced,      // Trigger after condition stable for duration
    Throttled,      // Trigger at most once per time period
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimeoutBehavior {
    Complete,       // Complete when timeout expires
    Reset,          // Reset timer when timeout expires
    TriggerOnTimeout, // Force trigger when timeout expires
}

pub struct TimedTrigger {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    
    // Trigger configuration
    condition: Option<TimedTriggerCondition>,
    action: Option<TimedTriggerAction>,
    trigger_type: TimedTriggerType,
    
    // Timing configuration
    timeout: Duration,
    start_time: Instant,
    timeout_behavior: TimeoutBehavior,
    
    // Debounce/throttle configuration
    debounce_duration: Duration,
    throttle_duration: Duration,
    
    // State tracking
    has_triggered: bool,
    trigger_count: u64,
    last_trigger_time: Option<Instant>,
    last_condition_state: bool,
    condition_stable_since: Option<Instant>,
    has_timed_out: bool,
    
    // Target and actions
    target: Option<Arc<RwLock<dyn Transient>>>,
    on_timeout_action: Option<TimedTriggerAction>,
    on_window_start_action: Option<TimedTriggerAction>,
    on_window_end_action: Option<TimedTriggerAction>,
    
    // Window settings
    max_triggers: Option<u64>,
}

impl TimedTrigger {
    pub fn new(trigger_type: TimedTriggerType, timeout: Duration) -> Self {
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            condition: None,
            action: None,
            trigger_type,
            timeout,
            start_time: Instant::now(),
            timeout_behavior: TimeoutBehavior::Complete,
            debounce_duration: Duration::from_millis(100),
            throttle_duration: Duration::from_millis(1000),
            has_triggered: false,
            trigger_count: 0,
            last_trigger_time: None,
            last_condition_state: false,
            condition_stable_since: None,
            has_timed_out: false,
            target: None,
            on_timeout_action: None,
            on_window_start_action: None,
            on_window_end_action: None,
            max_triggers: None,
        }
    }
    
    /// Trigger once within timeout window
    pub fn once_within(timeout: Duration) -> Self {
        Self::new(TimedTriggerType::Once, timeout)
    }
    
    /// Trigger repeatedly within timeout window
    pub fn repeat_within(timeout: Duration) -> Self {
        Self::new(TimedTriggerType::Repeating, timeout)
    }
    
    /// Debounced trigger - trigger after condition stable for duration
    pub fn debounced(debounce_duration: Duration, timeout: Duration) -> Self {
        let mut trigger = Self::new(TimedTriggerType::Debounced, timeout);
        trigger.debounce_duration = debounce_duration;
        trigger
    }
    
    /// Throttled trigger - trigger at most once per period
    pub fn throttled(throttle_duration: Duration, timeout: Duration) -> Self {
        let mut trigger = Self::new(TimedTriggerType::Throttled, timeout);
        trigger.throttle_duration = throttle_duration;
        trigger
    }
    
    /// Window trigger - trigger at specific points in time window
    pub fn window(trigger_type: TimedTriggerType, timeout: Duration) -> Self {
        Self::new(trigger_type, timeout)
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
    
    pub fn with_timeout_behavior(mut self, behavior: TimeoutBehavior) -> Self {
        self.timeout_behavior = behavior;
        self
    }
    
    pub fn with_timeout_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_timeout_action = Some(Box::new(action));
        self
    }
    
    pub fn with_window_start_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_window_start_action = Some(Box::new(action));
        self
    }
    
    pub fn with_window_end_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_window_end_action = Some(Box::new(action));
        self
    }
    
    pub fn with_max_triggers(mut self, max_triggers: u64) -> Self {
        self.max_triggers = Some(max_triggers);
        self
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.elapsed())
    }
    
    pub fn is_timed_out(&self) -> bool {
        self.elapsed() >= self.timeout
    }
    
    pub fn trigger_count(&self) -> u64 {
        self.trigger_count
    }
    
    pub fn has_triggered(&self) -> bool {
        self.has_triggered
    }
    
    pub fn can_trigger(&self) -> bool {
        if let Some(max_triggers) = self.max_triggers {
            self.trigger_count < max_triggers
        } else {
            match self.trigger_type {
                TimedTriggerType::Once => !self.has_triggered,
                _ => !self.is_timed_out(),
            }
        }
    }
    
    pub fn should_trigger(&mut self) -> bool {
        if !self.can_trigger() {
            return false;
        }
        
        // Get current condition state
        let current_condition = if let Some(ref condition) = self.condition {
            condition()
        } else {
            false
        };
        
        // Handle different trigger types
        match self.trigger_type {
            TimedTriggerType::Once => {
                current_condition && !self.has_triggered
            }
            TimedTriggerType::Repeating => {
                current_condition
            }
            TimedTriggerType::WindowStart => {
                // Trigger once at the beginning if condition is true
                if self.step_number == 1 {
                    current_condition
                } else {
                    false
                }
            }
            TimedTriggerType::WindowEnd => {
                // Trigger at the end of the window if condition is true
                self.is_timed_out() && current_condition
            }
            TimedTriggerType::Debounced => {
                self.check_debounced_condition(current_condition)
            }
            TimedTriggerType::Throttled => {
                self.check_throttled_condition(current_condition)
            }
        }
    }
    
    fn check_debounced_condition(&mut self, current_condition: bool) -> bool {
        // Track when condition became stable
        if current_condition != self.last_condition_state {
            self.condition_stable_since = Some(Instant::now());
            self.last_condition_state = current_condition;
            return false;
        }
        
        // Check if condition has been stable long enough
        if current_condition {
            if let Some(stable_since) = self.condition_stable_since {
                stable_since.elapsed() >= self.debounce_duration
            } else {
                false
            }
        } else {
            false
        }
    }
    
    fn check_throttled_condition(&mut self, current_condition: bool) -> bool {
        if !current_condition {
            return false;
        }
        
        // Check if enough time has passed since last trigger
        if let Some(last_trigger) = self.last_trigger_time {
            last_trigger.elapsed() >= self.throttle_duration
        } else {
            true
        }
    }
    
    pub async fn execute_trigger(&mut self) -> Result<()> {
        debug!("TimedTrigger {} executing trigger #{}", self.id, self.trigger_count + 1);
        
        self.has_triggered = true;
        self.trigger_count += 1;
        self.last_trigger_time = Some(Instant::now());
        
        // Reset debounce state after triggering
        if self.trigger_type == TimedTriggerType::Debounced {
            self.condition_stable_since = None;
        }
        
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
    
    pub async fn execute_timeout(&mut self) -> Result<()> {
        if self.has_timed_out {
            return Ok(());
        }
        
        self.has_timed_out = true;
        debug!("TimedTrigger {} timeout after {}ms", self.id, self.timeout.as_millis());
        
        // Execute timeout action
        if let Some(ref action) = self.on_timeout_action {
            action();
        }
        
        match self.timeout_behavior {
            TimeoutBehavior::Complete => {
                self.complete().await;
            }
            TimeoutBehavior::Reset => {
                self.reset();
            }
            TimeoutBehavior::TriggerOnTimeout => {
                self.execute_trigger().await?;
            }
        }
        
        Ok(())
    }
    
    pub async fn execute_window_start(&mut self) -> Result<()> {
        debug!("TimedTrigger {} window start", self.id);
        
        if let Some(ref action) = self.on_window_start_action {
            action();
        }
        
        Ok(())
    }
    
    pub async fn execute_window_end(&mut self) -> Result<()> {
        debug!("TimedTrigger {} window end", self.id);
        
        if let Some(ref action) = self.on_window_end_action {
            action();
        }
        
        Ok(())
    }
    
    pub fn reset(&mut self) {
        self.has_triggered = false;
        self.trigger_count = 0;
        self.last_trigger_time = None;
        self.last_condition_state = false;
        self.condition_stable_since = None;
        self.has_timed_out = false;
        self.start_time = Instant::now();
        self.state = GeneratorState::Running;
        debug!("TimedTrigger {} reset", self.id);
    }
    
    pub fn extend_timeout(&mut self, additional_time: Duration) {
        self.timeout += additional_time;
        debug!("TimedTrigger {} timeout extended by {}ms", self.id, additional_time.as_millis());
    }
}

impl Default for TimedTrigger {
    fn default() -> Self {
        Self::new(TimedTriggerType::Once, Duration::from_secs(30))
    }
}

#[async_trait]
impl Transient for TimedTrigger {
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
            TimedTriggerType::Once => {
                !self.has_triggered && !self.is_timed_out() && matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
            }
            _ => {
                !self.is_timed_out() && matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
            }
        }
    }
    
    fn is_completed(&self) -> bool {
        match self.trigger_type {
            TimedTriggerType::Once => {
                self.state == GeneratorState::Completed || self.has_triggered || self.is_timed_out()
            }
            TimedTriggerType::WindowEnd => {
                self.state == GeneratorState::Completed || self.is_timed_out()
            }
            _ => {
                self.state == GeneratorState::Completed || 
                (self.is_timed_out() && self.timeout_behavior == TimeoutBehavior::Complete)
            }
        }
    }
    
    async fn complete(&mut self) {
        debug!("TimedTrigger {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("TimedTrigger {} step #{} ({}ms remaining, {} triggers)", 
               self.id, self.step_number, self.remaining().as_millis(), self.trigger_count);
        
        // Handle window start
        if self.step_number == 1 && self.trigger_type == TimedTriggerType::WindowStart {
            self.execute_window_start().await?;
        }
        
        // Check for timeout
        if self.is_timed_out() && !self.has_timed_out {
            // Handle window end before timeout
            if self.trigger_type == TimedTriggerType::WindowEnd {
                self.execute_window_end().await?;
                if self.should_trigger() {
                    self.execute_trigger().await?;
                }
            }
            
            self.execute_timeout().await?;
            return Ok(());
        }
        
        // Check for trigger condition
        if self.should_trigger() {
            self.execute_trigger().await?;
            
            // Complete if it was a once trigger
            if self.trigger_type == TimedTriggerType::Once {
                self.complete().await;
            }
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("TimedTrigger {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("TimedTrigger {} suspending", self.id);
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
impl Steppable for TimedTrigger {}

#[async_trait]
impl Generator for TimedTrigger {
    type Output = u64; // Number of triggers executed
    
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
        Some(&self.trigger_count)
    }
    
    async fn pre(&mut self) {
        trace!("TimedTrigger {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("TimedTrigger {} post-step", self.id);
    }
}

// Example usage patterns
impl TimedTrigger {
    /// Example: Game event that must happen within time limit
    pub fn game_event(timeout: Duration) -> Self {
        Self::once_within(timeout)
            .with_condition(|| {
                // Example: Check if player performed action
                false // Placeholder
            })
            .with_action(|| {
                println!("Game event triggered!");
            })
            .with_timeout_action(|| {
                println!("Game event timed out!");
            })
    }
    
    /// Example: User input debouncing
    pub fn input_debounce(debounce_duration: Duration) -> Self {
        Self::debounced(debounce_duration, Duration::from_secs(60))
            .with_condition(|| {
                // Example: Check if user input changed
                false // Placeholder
            })
            .with_action(|| {
                println!("Input processed after debounce!");
            })
    }
    
    /// Example: Rate-limited API calls
    pub fn api_rate_limit(rate_limit: Duration) -> Self {
        Self::throttled(rate_limit, Duration::MAX)
            .with_condition(|| {
                // Example: Check if API call needed
                false // Placeholder
            })
            .with_action(|| {
                println!("API call made!");
            })
    }
}