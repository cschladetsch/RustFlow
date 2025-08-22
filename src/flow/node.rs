use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::flow::group::Group;
use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct Node {
    group: Group,
}

impl Node {
    pub fn new() -> Self {
        Self {
            group: Group::new(),
        }
    }
    
    pub fn contents(&self) -> &[Arc<RwLock<dyn Transient>>] {
        self.group.contents()
    }
    
    pub fn is_empty(&self) -> bool {
        self.group.is_empty()
    }
    
    pub async fn add(&mut self, transient: Arc<RwLock<dyn Transient>>) {
        self.group.add(transient).await;
    }
    
    pub async fn add_many(&mut self, transients: Vec<Arc<RwLock<dyn Transient>>>) {
        self.group.add_many(transients).await;
    }
    
    pub async fn remove(&mut self, id: TransientId) -> bool {
        self.group.remove(id).await
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transient for Node {
    fn id(&self) -> TransientId {
        self.group.id()
    }
    
    fn name(&self) -> Option<&str> {
        self.group.name()
    }
    
    fn set_name(&mut self, name: String) {
        self.group.set_name(name);
    }
    
    fn is_active(&self) -> bool {
        self.group.is_active()
    }
    
    fn is_completed(&self) -> bool {
        self.group.is_completed()
    }
    
    async fn complete(&mut self) {
        self.group.complete().await;
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl Steppable for Node {
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        trace!("Node {} stepping {} contents", self.id(), self.contents().len());
        
        let mut completed_ids = Vec::new();
        
        // Simplified stepping - just step the transients directly
        for content in self.contents() {
            if let Ok(mut transient) = content.try_write() {
                if transient.is_completed() {
                    completed_ids.push(transient.id());
                    continue;
                }
                
                // Try to step if it's steppable - simplified approach
                if let Err(e) = transient.step().await {
                    debug!("Error stepping transient {}: {}", transient.id(), e);
                }
            }
        }
        
        // Remove completed transients
        for id in completed_ids {
            self.remove(id).await;
        }
        
        self.group.step().await?;
        
        Ok(())
    }
}

#[async_trait]
impl Generator for Node {
    type Output = ();
    
    fn state(&self) -> GeneratorState {
        self.group.state()
    }
    
    fn step_number(&self) -> u64 {
        self.group.step_number()
    }
    
    fn value(&self) -> Option<&Self::Output> {
        self.group.value()
    }
    
    async fn resume(&mut self) {
        self.group.resume().await;
    }
    
    async fn suspend(&mut self) {
        self.group.suspend().await;
    }
    
    async fn pre(&mut self) {
        self.group.pre().await;
    }
    
    async fn post(&mut self) {
        self.group.post().await;
    }
}