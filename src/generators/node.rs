use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::core::{Generator, Group, Node, Steppable, Transient};
use crate::generators::GroupImpl;
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub struct NodeImpl {
    group: GroupImpl,
}

impl NodeImpl {
    pub fn new() -> Self {
        Self {
            group: GroupImpl::new(),
        }
    }
}

#[async_trait]
impl Transient for NodeImpl {
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
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[async_trait]
impl Steppable for NodeImpl {
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        trace!("Node {} stepping {} contents", self.id(), self.contents().len());
        
        let mut to_remove = Vec::new();
        
        for (i, content) in self.contents().iter().enumerate() {
            if let Ok(mut transient) = content.try_write() {
                if transient.is_completed() {
                    to_remove.push(i);
                    continue;
                }
                
                if let Some(generator) = transient.as_any_mut().downcast_mut::<dyn Generator<Output = dyn Send + Sync>>() {
                    generator.pre().await;
                    if let Some(steppable) = transient.as_any_mut().downcast_mut::<dyn Steppable>() {
                        if let Err(e) = steppable.step().await {
                            debug!("Error stepping transient {}: {}", transient.id(), e);
                        }
                    }
                    generator.post().await;
                }
            }
        }
        
        for &i in to_remove.iter().rev() {
            if let Some(content) = self.group.contents.get(i) {
                if let Ok(transient) = content.try_read() {
                    let id = transient.id();
                    drop(transient);
                    self.group.remove_transient(id).await;
                }
            }
        }
        
        self.group.step().await?;
        
        Ok(())
    }
}

#[async_trait]
impl Generator for NodeImpl {
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

#[async_trait]
impl Group for NodeImpl {
    fn contents(&self) -> &[Arc<RwLock<dyn Transient>>] {
        self.group.contents()
    }
    
    fn generators(&self) -> Vec<Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>> {
        self.group.generators()
    }
    
    fn is_empty(&self) -> bool {
        self.group.is_empty()
    }
    
    async fn add_transient(&mut self, transient: Arc<RwLock<dyn Transient>>) {
        self.group.add_transient(transient).await;
    }
    
    async fn add_transients(&mut self, transients: Vec<Arc<RwLock<dyn Transient>>>) {
        self.group.add_transients(transients).await;
    }
    
    async fn remove_transient(&mut self, id: TransientId) -> bool {
        self.group.remove_transient(id).await
    }
    
    async fn on_added(&mut self, transient: &Arc<RwLock<dyn Transient>>) {
        self.group.on_added(transient).await;
    }
    
    async fn on_removed(&mut self, transient: &Arc<RwLock<dyn Transient>>) {
        self.group.on_removed(transient).await;
    }
}

#[async_trait]
impl Node for NodeImpl {
    async fn add_generator(&mut self, generator: Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>) {
        let transient: Arc<RwLock<dyn Transient>> = generator as Arc<RwLock<dyn Transient>>;
        self.add_transient(transient).await;
    }
    
    async fn add_generators(&mut self, generators: Vec<Arc<RwLock<dyn Generator<Output = impl Send + Sync>>>>) {
        for generator in generators {
            self.add_generator(generator).await;
        }
    }
}