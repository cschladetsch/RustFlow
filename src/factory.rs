use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::flow::{
    Barrier, Boundary, Channel, Coroutine, Future, Group, Node, 
    Periodic, Sequence, TimedBarrier, TimedFuture, TimedTimer, 
    TimedTrigger, Timer, Trigger
};
use crate::traits::Transient;

pub struct Factory;

impl Factory {
    pub fn new() -> Self {
        Self
    }
    
    // Basic components
    pub fn group(&self) -> Arc<RwLock<Group>> {
        Arc::new(RwLock::new(Group::new()))
    }
    
    pub fn node(&self) -> Arc<RwLock<Node>> {
        Arc::new(RwLock::new(Node::new()))
    }
    
    pub fn sequence(&self) -> Arc<RwLock<Sequence>> {
        Arc::new(RwLock::new(Sequence::new()))
    }
    
    pub fn sequence_with_steps(&self, steps: Vec<Arc<RwLock<dyn Transient>>>) -> Arc<RwLock<Sequence>> {
        Arc::new(RwLock::new(Sequence::with_steps(steps)))
    }
    
    pub fn barrier(&self) -> Arc<RwLock<Barrier>> {
        Arc::new(RwLock::new(Barrier::new()))
    }
    
    // Timers
    pub fn timer(&self, interval: Duration) -> Arc<RwLock<Timer>> {
        Arc::new(RwLock::new(Timer::new(interval)))
    }
    
    pub fn timed_timer(&self, interval: Duration, behavior: crate::flow::TimedTimerBehavior) -> Arc<RwLock<TimedTimer>> {
        Arc::new(RwLock::new(TimedTimer::new(interval, behavior)))
    }
    
    pub fn periodic(&self, period: Duration) -> Arc<RwLock<Periodic>> {
        Arc::new(RwLock::new(Periodic::new(period)))
    }
    
    // Triggers
    pub fn trigger(&self) -> Arc<RwLock<Trigger>> {
        Arc::new(RwLock::new(Trigger::new()))
    }
    
    pub fn timed_trigger(&self, trigger_type: crate::flow::TimedTriggerType, timeout: Duration) -> Arc<RwLock<TimedTrigger>> {
        Arc::new(RwLock::new(TimedTrigger::new(trigger_type, timeout)))
    }
    
    // Barriers
    pub fn timed_barrier(&self, participants: Vec<String>, timeout: Duration) -> Arc<RwLock<TimedBarrier>> {
        Arc::new(RwLock::new(TimedBarrier::new(participants, timeout)))
    }
    
    // Boundaries and guards
    pub fn boundary(&self, boundary_type: crate::flow::BoundaryType) -> Arc<RwLock<Boundary>> {
        Arc::new(RwLock::new(Boundary::new(boundary_type)))
    }
    
    // Futures
    pub fn future<T: Send + Sync + Clone + 'static>(&self) -> Arc<RwLock<Future<T>>> {
        Arc::new(RwLock::new(Future::new()))
    }
    
    pub fn timed_future<T: Send + Sync + Clone + 'static>(&self, timeout: Duration) -> Arc<RwLock<TimedFuture<T>>> {
        Arc::new(RwLock::new(TimedFuture::new(timeout)))
    }
    
    // Channels
    pub fn channel<T: Send + Sync + 'static>(&self) -> Arc<RwLock<Channel<T>>> {
        Arc::new(RwLock::new(Channel::new()))
    }
    
    // Coroutines
    pub fn coroutine(&self) -> Arc<RwLock<Coroutine>> {
        Arc::new(RwLock::new(Coroutine::new()))
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}