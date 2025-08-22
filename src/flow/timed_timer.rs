use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type TimerAction = Box<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimedTimerBehavior {
    OneShot,        // Fire once and complete
    Repeating,      // Fire repeatedly until stopped
    Limited,        // Fire limited number of times
    Conditional,    // Fire only while condition is met
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerCompletionBehavior {
    Complete,       // Complete when done
    Reset,          // Reset and start over
    Pause,          // Pause until resumed
}

pub struct TimedTimer {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    
    // Timer configuration
    interval: Duration,
    behavior: TimedTimerBehavior,
    completion_behavior: TimerCompletionBehavior,
    
    // Timing state
    start_time: Instant,
    last_fire_time: Option<Instant>,
    next_fire_time: Instant,
    
    // Limits and conditions
    max_fires: Option<u64>,
    fire_count: u64,
    condition: Option<Box<dyn Fn() -> bool + Send + Sync>>,
    
    // Timeout controls
    total_timeout: Option<Duration>,
    fire_timeout: Option<Duration>,  // Timeout between fires
    
    // State tracking
    is_paused: bool,
    pause_time: Option<Instant>,
    accumulated_pause_duration: Duration,
    
    // Actions
    on_fire_action: Option<TimerAction>,
    on_timeout_action: Option<TimerAction>,
    on_complete_action: Option<TimerAction>,
    on_pause_action: Option<TimerAction>,
    on_resume_action: Option<TimerAction>,
    
    // Target
    target: Option<Arc<RwLock<dyn Transient>>>,
}

impl TimedTimer {
    pub fn new(interval: Duration, behavior: TimedTimerBehavior) -> Self {
        let now = Instant::now();
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            interval,
            behavior,
            completion_behavior: TimerCompletionBehavior::Complete,
            start_time: now,
            last_fire_time: None,
            next_fire_time: now + interval,
            max_fires: None,
            fire_count: 0,
            condition: None,
            total_timeout: None,
            fire_timeout: None,
            is_paused: false,
            pause_time: None,
            accumulated_pause_duration: Duration::ZERO,
            on_fire_action: None,
            on_timeout_action: None,
            on_complete_action: None,
            on_pause_action: None,
            on_resume_action: None,
            target: None,
        }
    }
    
    /// One-shot timer - fires once after interval
    pub fn one_shot(interval: Duration) -> Self {
        Self::new(interval, TimedTimerBehavior::OneShot)
    }
    
    /// Repeating timer - fires every interval
    pub fn repeating(interval: Duration) -> Self {
        Self::new(interval, TimedTimerBehavior::Repeating)
    }
    
    /// Limited timer - fires max_fires times
    pub fn limited(interval: Duration, max_fires: u64) -> Self {
        let mut timer = Self::new(interval, TimedTimerBehavior::Limited);
        timer.max_fires = Some(max_fires);
        timer
    }
    
    /// Conditional timer - fires while condition is true
    pub fn conditional(interval: Duration) -> Self {
        Self::new(interval, TimedTimerBehavior::Conditional)
    }
    
    pub fn with_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Box::new(condition));
        self
    }
    
    pub fn with_total_timeout(mut self, timeout: Duration) -> Self {
        self.total_timeout = Some(timeout);
        self
    }
    
    pub fn with_fire_timeout(mut self, timeout: Duration) -> Self {
        self.fire_timeout = Some(timeout);
        self
    }
    
    pub fn with_completion_behavior(mut self, behavior: TimerCompletionBehavior) -> Self {
        self.completion_behavior = behavior;
        self
    }
    
    pub fn with_target(mut self, target: Arc<RwLock<dyn Transient>>) -> Self {
        self.target = Some(target);
        self
    }
    
    pub fn with_fire_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_fire_action = Some(Box::new(action));
        self
    }
    
    pub fn with_timeout_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_timeout_action = Some(Box::new(action));
        self
    }
    
    pub fn with_complete_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_complete_action = Some(Box::new(action));
        self
    }
    
    pub fn with_pause_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_pause_action = Some(Box::new(action));
        self
    }
    
    pub fn with_resume_action<F>(mut self, action: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_resume_action = Some(Box::new(action));
        self
    }
    
    pub fn interval(&self) -> Duration {
        self.interval
    }
    
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
        // Update next fire time if not paused
        if !self.is_paused {
            self.next_fire_time = Instant::now() + interval;
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed() - self.accumulated_pause_duration
    }
    
    pub fn time_until_next_fire(&self) -> Duration {
        if self.is_paused {
            Duration::MAX
        } else {
            self.next_fire_time.saturating_duration_since(Instant::now())
        }
    }
    
    pub fn fire_count(&self) -> u64 {
        self.fire_count
    }
    
    pub fn max_fires(&self) -> Option<u64> {
        self.max_fires
    }
    
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }
    
    pub fn is_ready_to_fire(&self) -> bool {
        if self.is_paused {
            return false;
        }
        
        // Check if it's time to fire
        let time_ready = Instant::now() >= self.next_fire_time;
        
        if !time_ready {
            return false;
        }
        
        // Check behavior-specific conditions
        match self.behavior {
            TimedTimerBehavior::OneShot => self.fire_count == 0,
            TimedTimerBehavior::Repeating => true,
            TimedTimerBehavior::Limited => {
                if let Some(max_fires) = self.max_fires {
                    self.fire_count < max_fires
                } else {
                    true
                }
            }
            TimedTimerBehavior::Conditional => {
                if let Some(ref condition) = self.condition {
                    condition()
                } else {
                    true
                }
            }
        }
    }
    
    pub fn is_total_timeout_expired(&self) -> bool {
        if let Some(total_timeout) = self.total_timeout {
            self.elapsed() >= total_timeout
        } else {
            false
        }
    }
    
    pub fn is_fire_timeout_expired(&self) -> bool {
        if let Some(fire_timeout) = self.fire_timeout {
            if let Some(last_fire) = self.last_fire_time {
                last_fire.elapsed() >= fire_timeout
            } else {
                self.elapsed() >= fire_timeout
            }
        } else {
            false
        }
    }
    
    pub fn should_complete(&self) -> bool {
        // Check various completion conditions
        match self.behavior {
            TimedTimerBehavior::OneShot => self.fire_count > 0,
            TimedTimerBehavior::Limited => {
                if let Some(max_fires) = self.max_fires {
                    self.fire_count >= max_fires
                } else {
                    false
                }
            }
            _ => self.is_total_timeout_expired(),
        }
    }
    
    pub async fn fire(&mut self) -> Result<()> {
        debug!("TimedTimer {} firing (fire #{}/{})", 
               self.id, self.fire_count + 1,
               self.max_fires.map_or("∞".to_string(), |max| max.to_string()));
        
        self.fire_count += 1;
        self.last_fire_time = Some(Instant::now());
        
        // Update next fire time for repeating timers
        if self.behavior == TimedTimerBehavior::Repeating || 
           (self.behavior == TimedTimerBehavior::Limited && !self.should_complete()) {
            self.next_fire_time = Instant::now() + self.interval;
        }
        
        // Execute fire action
        if let Some(ref action) = self.on_fire_action {
            action();
        }
        
        // Fire target
        if let Some(ref target) = self.target {
            if let Ok(mut transient) = target.try_write() {
                transient.step().await?;
            }
        }
        
        Ok(())
    }
    
    pub async fn pause_timer(&mut self) {
        if !self.is_paused {
            self.is_paused = true;
            self.pause_time = Some(Instant::now());
            debug!("TimedTimer {} paused", self.id);
            
            if let Some(ref action) = self.on_pause_action {
                action();
            }
        }
    }
    
    pub async fn resume_timer(&mut self) {
        if self.is_paused {
            if let Some(pause_time) = self.pause_time {
                self.accumulated_pause_duration += pause_time.elapsed();
                
                // Adjust next fire time
                let remaining = self.time_until_next_fire();
                self.next_fire_time = Instant::now() + remaining;
            }
            
            self.is_paused = false;
            self.pause_time = None;
            debug!("TimedTimer {} resumed", self.id);
            
            if let Some(ref action) = self.on_resume_action {
                action();
            }
        }
    }
    
    pub async fn handle_timeout(&mut self) -> Result<()> {
        debug!("TimedTimer {} handling timeout", self.id);
        
        if let Some(ref action) = self.on_timeout_action {
            action();
        }
        
        match self.completion_behavior {
            TimerCompletionBehavior::Complete => {
                self.execute_complete().await?;
            }
            TimerCompletionBehavior::Reset => {
                self.reset();
            }
            TimerCompletionBehavior::Pause => {
                self.pause_timer().await;
            }
        }
        
        Ok(())
    }
    
    pub async fn execute_complete(&mut self) -> Result<()> {
        debug!("TimedTimer {} completing after {} fires", self.id, self.fire_count);
        
        if let Some(ref action) = self.on_complete_action {
            action();
        }
        
        self.complete().await;
        Ok(())
    }
    
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.fire_count = 0;
        self.last_fire_time = None;
        self.next_fire_time = now + self.interval;
        self.start_time = now;
        self.accumulated_pause_duration = Duration::ZERO;
        self.is_paused = false;
        self.pause_time = None;
        self.state = GeneratorState::Running;
        debug!("TimedTimer {} reset", self.id);
    }
    
    pub fn get_stats(&self) -> TimerStats {
        TimerStats {
            fire_count: self.fire_count,
            elapsed: self.elapsed(),
            time_until_next_fire: self.time_until_next_fire(),
            is_paused: self.is_paused,
            total_timeout_remaining: self.total_timeout.map(|timeout| timeout.saturating_sub(self.elapsed())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimerStats {
    pub fire_count: u64,
    pub elapsed: Duration,
    pub time_until_next_fire: Duration,
    pub is_paused: bool,
    pub total_timeout_remaining: Option<Duration>,
}

impl Default for TimedTimer {
    fn default() -> Self {
        Self::one_shot(Duration::from_secs(1))
    }
}

#[async_trait]
impl Transient for TimedTimer {
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
        if self.should_complete() || self.is_total_timeout_expired() {
            false
        } else {
            matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
        }
    }
    
    fn is_completed(&self) -> bool {
        self.state == GeneratorState::Completed || self.should_complete() || self.is_total_timeout_expired()
    }
    
    async fn complete(&mut self) {
        debug!("TimedTimer {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("TimedTimer {} step #{} ({} fires, {}ms until next)", 
               self.id, self.step_number, self.fire_count, 
               self.time_until_next_fire().as_millis());
        
        // Check for total timeout
        if self.is_total_timeout_expired() {
            self.handle_timeout().await?;
            return Ok(());
        }
        
        // Check for fire timeout
        if self.is_fire_timeout_expired() {
            warn!("TimedTimer {} fire timeout expired", self.id);
            self.handle_timeout().await?;
            return Ok(());
        }
        
        // Check if ready to fire
        if self.is_ready_to_fire() {
            self.fire().await?;
            
            // Check if should complete after firing
            if self.should_complete() {
                self.execute_complete().await?;
            }
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("TimedTimer {} resuming", self.id);
        self.state = GeneratorState::Running;
        if self.is_paused {
            self.resume_timer().await;
        }
    }
    
    async fn suspend(&mut self) {
        debug!("TimedTimer {} suspending", self.id);
        self.state = GeneratorState::Suspended;
        if !self.is_paused {
            self.pause_timer().await;
        }
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl Steppable for TimedTimer {}

#[async_trait]
impl Generator for TimedTimer {
    type Output = u64; // Number of fires
    
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
        Some(&self.fire_count)
    }
    
    async fn pre(&mut self) {
        trace!("TimedTimer {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("TimedTimer {} post-step", self.id);
    }
}

// Example usage patterns
impl TimedTimer {
    /// Game countdown timer
    pub fn countdown(duration: Duration) -> Self {
        Self::one_shot(duration)
            .with_fire_action(|| {
                println!("Time's up!");
            })
            .with_timeout_action(|| {
                println!("Countdown finished!");
            })
    }
    
    /// Heartbeat timer for network keep-alive
    pub fn heartbeat(interval: Duration, total_timeout: Duration) -> Self {
        Self::repeating(interval)
            .with_total_timeout(total_timeout)
            .with_fire_action(|| {
                println!("Heartbeat sent");
            })
            .with_timeout_action(|| {
                println!("Connection timed out");
            })
    }
    
    /// Rate-limited operation timer
    pub fn rate_limited(interval: Duration, max_operations: u64) -> Self {
        Self::limited(interval, max_operations)
            .with_fire_action(|| {
                println!("Operation executed");
            })
            .with_complete_action(|| {
                println!("Rate limit reached");
            })
    }
    
    /// Conditional timer that fires only when condition is met
    pub fn conditional_with_condition<F>(interval: Duration, condition: F) -> Self 
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        Self::conditional(interval)
            .with_condition(condition)
            .with_fire_action(|| {
                println!("Conditional timer fired");
            })
    }
}