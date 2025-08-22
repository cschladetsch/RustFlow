use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, trace};

use crate::flow::node::Node;
use crate::logger::Logger;
use crate::traits::{Generator, Steppable, Transient};
use crate::types::{DebugLevel, GeneratorState, TimeFrame, TransientId};
use crate::Factory;
use crate::Result;

pub struct Kernel {
    id: TransientId,
    name: Option<String>,
    debug_level: DebugLevel,
    logger: Logger,
    time_frame: TimeFrame,
    root: Arc<RwLock<Node>>,
    factory: Factory,
    is_breaking: bool,
    wait_until: Option<std::time::Instant>,
    state: GeneratorState,
    step_number: u64,
}

impl Kernel {
    pub fn new() -> Self {
        let id = TransientId::new();
        let logger = Logger::new("RustFlow".to_string());
        let time_frame = TimeFrame::new();
        let factory = Factory::new();
        
        let root = Arc::new(RwLock::new(Node::new()));
        
        Self {
            id,
            name: None,
            debug_level: DebugLevel::Medium,
            logger,
            time_frame,
            root,
            factory,
            is_breaking: false,
            wait_until: None,
            state: GeneratorState::Running,
            step_number: 0,
        }
    }
    
    pub fn debug_level(&self) -> DebugLevel {
        self.debug_level
    }
    
    pub fn set_debug_level(&mut self, level: DebugLevel) {
        self.debug_level = level;
    }
    
    pub fn logger(&self) -> &Logger {
        &self.logger
    }
    
    pub fn logger_mut(&mut self) -> &mut Logger {
        &mut self.logger
    }
    
    pub fn time(&self) -> &TimeFrame {
        &self.time_frame
    }
    
    pub fn root(&self) -> Arc<RwLock<Node>> {
        Arc::clone(&self.root)
    }
    
    pub fn factory(&self) -> &Factory {
        &self.factory
    }
    
    pub fn is_break(&self) -> bool {
        self.is_breaking
    }
    
    pub fn break_flow(&mut self) {
        info!("Breaking flow execution");
        self.is_breaking = true;
    }
    
    pub async fn wait_for(&mut self, duration: Duration) {
        let wait_until = std::time::Instant::now() + duration;
        self.wait_until = Some(wait_until);
        debug!("Kernel waiting for {:?}", duration);
    }
    
    pub async fn update(&mut self, delta_time: Duration) -> Result<()> {
        self.update_time(delta_time);
        self.process().await
    }
    
    fn update_time(&mut self, delta_time: Duration) {
        self.time_frame.update(delta_time);
    }
    
    fn step_time(&mut self) {
        self.time_frame.step();
    }
    
    async fn process(&mut self) -> Result<()> {
        if self.is_breaking {
            debug!("Kernel is breaking, skipping process");
            return Ok(());
        }
        
        if let Some(wait_until) = self.wait_until {
            if std::time::Instant::now() < wait_until {
                trace!("Kernel still waiting");
                return Ok(());
            }
            self.wait_until = None;
            debug!("Kernel wait period ended");
        }
        
        let root_clone = Arc::clone(&self.root);
        let mut root = root_clone.write().await;
        
        if !root.contents().is_empty() {
            trace!("Stepping kernel with {} contents", root.contents().len());
        }
        
        root.step().await?;
        self.step().await?;
        
        Ok(())
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transient for Kernel {
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
        info!("Kernel {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[async_trait]
impl Steppable for Kernel {
    async fn step(&mut self) -> Result<()> {
        self.step_time();
        
        if let Some(wait_until) = self.wait_until {
            if std::time::Instant::now() < wait_until {
                return Ok(());
            }
            self.wait_until = None;
        }
        
        self.step_number += 1;
        trace!("Kernel step #{}", self.step_number);
        
        Ok(())
    }
}

#[async_trait]
impl Generator for Kernel {
    type Output = bool;
    
    fn state(&self) -> GeneratorState {
        self.state
    }
    
    fn step_number(&self) -> u64 {
        self.step_number
    }
    
    fn value(&self) -> Option<&Self::Output> {
        Some(&true)
    }
    
    async fn resume(&mut self) {
        info!("Kernel {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        info!("Kernel {} suspending", self.id);
        self.state = GeneratorState::Suspended;
    }
    
    async fn pre(&mut self) {
        trace!("Kernel pre-step");
    }
    
    async fn post(&mut self) {
        trace!("Kernel post-step");
    }
}