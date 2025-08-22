use async_trait::async_trait;
use std::any::Any;

use crate::types::{GeneratorState, TransientId};
use crate::Result;

#[async_trait]
pub trait Transient: Send + Sync {
    fn id(&self) -> TransientId;
    fn name(&self) -> Option<&str>;
    fn set_name(&mut self, name: String);
    
    fn is_active(&self) -> bool;
    fn is_completed(&self) -> bool;
    
    async fn complete(&mut self);
    async fn step(&mut self) -> Result<()>;
    async fn resume(&mut self);
    async fn suspend(&mut self);
    
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[async_trait]
pub trait Steppable: Transient {
    // Inherits step from Transient
}

#[async_trait]
pub trait Generator: Steppable {
    type Output: Send + Sync;
    
    fn state(&self) -> GeneratorState;
    fn step_number(&self) -> u64;
    fn value(&self) -> Option<&Self::Output>;
    
    async fn pre(&mut self);
    async fn post(&mut self);
    
    fn is_running(&self) -> bool {
        self.state() == GeneratorState::Running
    }
}

pub trait Named {
    fn named(self, name: String) -> Self where Self: Sized;
}