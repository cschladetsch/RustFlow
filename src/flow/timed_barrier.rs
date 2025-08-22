use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use crate::traits::{Generator, Steppable, Transient};
use crate::types::{GeneratorState, TransientId};
use crate::Result;

pub type ParticipantId = String;
pub type ParticipantData = Box<dyn Any + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimedBarrierBehavior {
    WaitForAll,        // Wait for all participants, timeout if not all arrive
    WaitForMajority,   // Wait for majority of participants
    WaitForMinimum,    // Wait for minimum number of participants
}

pub struct ParticipantResult {
    pub id: ParticipantId,
    pub data: ParticipantData,
    pub timestamp: Instant,
}

pub struct TimedBarrier {
    id: TransientId,
    name: Option<String>,
    state: GeneratorState,
    step_number: u64,
    
    // Barrier configuration
    expected_participants: Vec<ParticipantId>,
    behavior: TimedBarrierBehavior,
    minimum_participants: usize,
    
    // Timeout configuration
    timeout: Duration,
    start_time: Instant,
    has_timed_out: bool,
    
    // Participant tracking
    arrived_participants: HashMap<ParticipantId, ParticipantResult>,
    waiting_participants: Vec<ParticipantId>,
    cached_arrived_count: usize,
    
    // Actions
    on_complete_action: Option<Box<dyn Fn(&HashMap<ParticipantId, ParticipantResult>) + Send + Sync>>,
    on_timeout_action: Option<Box<dyn Fn(&HashMap<ParticipantId, ParticipantResult>) + Send + Sync>>,
    on_participant_arrive_action: Option<Box<dyn Fn(&ParticipantResult) + Send + Sync>>,
}

impl TimedBarrier {
    pub fn new(participants: Vec<ParticipantId>, timeout: Duration) -> Self {
        let waiting_participants = participants.clone();
        
        Self {
            id: TransientId::new(),
            name: None,
            state: GeneratorState::Running,
            step_number: 0,
            expected_participants: participants,
            behavior: TimedBarrierBehavior::WaitForAll,
            minimum_participants: 1,
            timeout,
            start_time: Instant::now(),
            has_timed_out: false,
            arrived_participants: HashMap::new(),
            waiting_participants,
            cached_arrived_count: 0,
            on_complete_action: None,
            on_timeout_action: None,
            on_participant_arrive_action: None,
        }
    }
    
    /// Example: P2P dice game - wait for all players to roll dice
    pub fn dice_game(player_ids: Vec<String>, timeout: Duration) -> Self {
        let mut barrier = Self::new(player_ids, timeout);
        barrier.name = Some("DiceGame".to_string());
        barrier.behavior = TimedBarrierBehavior::WaitForAll;
        barrier
    }
    
    /// Example: Voting system - wait for majority
    pub fn voting(voter_ids: Vec<String>, timeout: Duration) -> Self {
        let mut barrier = Self::new(voter_ids, timeout);
        barrier.name = Some("Voting".to_string());
        barrier.behavior = TimedBarrierBehavior::WaitForMajority;
        barrier
    }
    
    /// Example: Minimum quorum - wait for minimum number
    pub fn quorum(participant_ids: Vec<String>, minimum: usize, timeout: Duration) -> Self {
        let mut barrier = Self::new(participant_ids, timeout);
        barrier.name = Some("Quorum".to_string());
        barrier.behavior = TimedBarrierBehavior::WaitForMinimum;
        barrier.minimum_participants = minimum;
        barrier
    }
    
    pub fn with_behavior(mut self, behavior: TimedBarrierBehavior) -> Self {
        self.behavior = behavior;
        self
    }
    
    pub fn with_minimum(mut self, minimum: usize) -> Self {
        self.minimum_participants = minimum.min(self.expected_participants.len());
        self
    }
    
    pub fn with_on_complete<F>(mut self, action: F) -> Self
    where
        F: Fn(&HashMap<ParticipantId, ParticipantResult>) + Send + Sync + 'static,
    {
        self.on_complete_action = Some(Box::new(action));
        self
    }
    
    pub fn with_on_timeout<F>(mut self, action: F) -> Self
    where
        F: Fn(&HashMap<ParticipantId, ParticipantResult>) + Send + Sync + 'static,
    {
        self.on_timeout_action = Some(Box::new(action));
        self
    }
    
    pub fn with_on_participant_arrive<F>(mut self, action: F) -> Self
    where
        F: Fn(&ParticipantResult) + Send + Sync + 'static,
    {
        self.on_participant_arrive_action = Some(Box::new(action));
        self
    }
    
    /// Example: Player rolls dice and reports result
    pub async fn participant_arrive<T: Any + Send + Sync>(&mut self, participant_id: ParticipantId, data: T) -> Result<()> {
        if self.has_timed_out || self.state == GeneratorState::Completed {
            return Ok(()); // Too late
        }
        
        if !self.expected_participants.contains(&participant_id) {
            warn!("TimedBarrier {} received data from unexpected participant: {}", self.id, participant_id);
            return Ok(());
        }
        
        if self.arrived_participants.contains_key(&participant_id) {
            trace!("TimedBarrier {} participant {} already arrived, updating data", self.id, participant_id);
        } else {
            debug!("TimedBarrier {} participant {} arrived ({}/{})", 
                   self.id, participant_id, 
                   self.arrived_participants.len() + 1, 
                   self.expected_participants.len());
        }
        
        let result = ParticipantResult {
            id: participant_id.clone(),
            data: Box::new(data),
            timestamp: Instant::now(),
        };
        
        // Call participant arrive action
        if let Some(ref action) = self.on_participant_arrive_action {
            action(&result);
        }
        
        self.arrived_participants.insert(participant_id.clone(), result);
        self.waiting_participants.retain(|id| id != &participant_id);
        self.cached_arrived_count = self.arrived_participants.len();
        
        Ok(())
    }
    
    pub fn is_ready_to_complete(&self) -> bool {
        if self.has_timed_out {
            return false;
        }
        
        match self.behavior {
            TimedBarrierBehavior::WaitForAll => {
                self.arrived_participants.len() == self.expected_participants.len()
            }
            TimedBarrierBehavior::WaitForMajority => {
                let majority = (self.expected_participants.len() / 2) + 1;
                self.arrived_participants.len() >= majority
            }
            TimedBarrierBehavior::WaitForMinimum => {
                self.arrived_participants.len() >= self.minimum_participants
            }
        }
    }
    
    pub fn is_timed_out(&self) -> bool {
        !self.has_timed_out && self.start_time.elapsed() >= self.timeout
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.elapsed())
    }
    
    pub fn arrived_count(&self) -> usize {
        self.arrived_participants.len()
    }
    
    pub fn waiting_count(&self) -> usize {
        self.waiting_participants.len()
    }
    
    pub fn expected_count(&self) -> usize {
        self.expected_participants.len()
    }
    
    pub fn get_participant_data<T: Any>(&self, participant_id: &str) -> Option<&T> {
        self.arrived_participants
            .get(participant_id)
            .and_then(|result| result.data.downcast_ref::<T>())
    }
    
    pub fn get_all_participants(&self) -> &HashMap<ParticipantId, ParticipantResult> {
        &self.arrived_participants
    }
    
    pub fn get_waiting_participants(&self) -> &Vec<ParticipantId> {
        &self.waiting_participants
    }
    
    async fn execute_timeout(&mut self) -> Result<()> {
        if self.has_timed_out {
            return Ok(());
        }
        
        self.has_timed_out = true;
        warn!("TimedBarrier {} timed out after {}ms with {}/{} participants", 
              self.id, self.timeout.as_millis(), 
              self.arrived_participants.len(), self.expected_participants.len());
        
        if let Some(ref action) = self.on_timeout_action {
            action(&self.arrived_participants);
        }
        
        self.complete().await;
        Ok(())
    }
    
    async fn execute_complete(&mut self) -> Result<()> {
        debug!("TimedBarrier {} completing with {}/{} participants", 
               self.id, self.arrived_participants.len(), self.expected_participants.len());
        
        if let Some(ref action) = self.on_complete_action {
            action(&self.arrived_participants);
        }
        
        self.complete().await;
        Ok(())
    }
    
    pub fn reset(&mut self) {
        self.arrived_participants.clear();
        self.waiting_participants = self.expected_participants.clone();
        self.cached_arrived_count = 0;
        self.has_timed_out = false;
        self.start_time = Instant::now();
        self.state = GeneratorState::Running;
        debug!("TimedBarrier {} reset", self.id);
    }
}

impl Default for TimedBarrier {
    fn default() -> Self {
        Self::new(Vec::new(), Duration::from_secs(30))
    }
}

#[async_trait]
impl Transient for TimedBarrier {
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
        if self.is_ready_to_complete() || self.has_timed_out {
            false
        } else {
            matches!(self.state, GeneratorState::Running | GeneratorState::Suspended)
        }
    }
    
    fn is_completed(&self) -> bool {
        self.state == GeneratorState::Completed || self.is_ready_to_complete() || self.has_timed_out
    }
    
    async fn complete(&mut self) {
        debug!("TimedBarrier {} completing", self.id);
        self.state = GeneratorState::Completed;
    }
    
    async fn step(&mut self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        
        self.step_number += 1;
        trace!("TimedBarrier {} step #{} ({}/{} participants, {}ms remaining)", 
               self.id, self.step_number,
               self.arrived_participants.len(), self.expected_participants.len(),
               self.remaining().as_millis());
        
        // Check for timeout first
        if self.is_timed_out() {
            self.execute_timeout().await?;
            return Ok(());
        }
        
        // Check if ready to complete
        if self.is_ready_to_complete() {
            self.execute_complete().await?;
            return Ok(());
        }
        
        Ok(())
    }
    
    async fn resume(&mut self) {
        debug!("TimedBarrier {} resuming", self.id);
        self.state = GeneratorState::Running;
    }
    
    async fn suspend(&mut self) {
        debug!("TimedBarrier {} suspending", self.id);
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
impl Steppable for TimedBarrier {}

#[async_trait]
impl Generator for TimedBarrier {
    type Output = usize; // Number of arrived participants
    
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
        Some(&self.cached_arrived_count)
    }
    
    async fn pre(&mut self) {
        trace!("TimedBarrier {} pre-step", self.id);
    }
    
    async fn post(&mut self) {
        trace!("TimedBarrier {} post-step", self.id);
    }
}

// Example usage structures for dice game
#[derive(Clone, Debug)]
pub struct DiceRoll {
    pub value: u8,
    pub timestamp: Instant,
}

impl DiceRoll {
    pub fn new(value: u8) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
        }
    }
    
    pub fn roll() -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::SystemTime;
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        let value = (hasher.finish() % 6 + 1) as u8;
        
        Self::new(value)
    }
}

// Helper function for dice game example
impl TimedBarrier {
    /// Helper method for dice game - extract all dice rolls
    pub fn get_all_dice_rolls(&self) -> Vec<(ParticipantId, DiceRoll)> {
        self.arrived_participants
            .iter()
            .filter_map(|(id, result)| {
                result.data.downcast_ref::<DiceRoll>()
                    .map(|dice| (id.clone(), dice.clone()))
            })
            .collect()
    }
    
    /// Helper method for dice game - get winner (highest roll)
    pub fn get_dice_winner(&self) -> Option<(ParticipantId, DiceRoll)> {
        self.get_all_dice_rolls()
            .into_iter()
            .max_by_key(|(_, dice)| dice.value)
    }
}