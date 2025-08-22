use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    
    async fn step(&mut self) -> Result<()> {
        self.group.step().await
    }
    
    async fn resume(&mut self) {
        self.group.resume().await;
    }
    
    async fn suspend(&mut self) {
        self.group.suspend().await;
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl Steppable for Node {}

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
    
    async fn pre(&mut self) {
        self.group.pre().await;
    }
    
    async fn post(&mut self) {
        self.group.post().await;
    }
}