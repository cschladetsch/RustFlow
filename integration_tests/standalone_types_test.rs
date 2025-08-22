// Standalone test for types module - copy of the types code with tests
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

#[derive(Debug, Clone)]
pub struct TransientId(u64);

impl TransientId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for TransientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorState {
    Inactive,
    Running,
    Suspended,
    Completed,
}

impl fmt::Display for GeneratorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorState::Inactive => write!(f, "Inactive"),
            GeneratorState::Running => write!(f, "Running"),
            GeneratorState::Suspended => write!(f, "Suspended"),
            GeneratorState::Completed => write!(f, "Completed"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeFrame {
    pub now: Instant,
    pub delta: Duration,
    pub last: Instant,
}

impl TimeFrame {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            now,
            delta: Duration::from_secs(0),
            last: now,
        }
    }
    
    pub fn update(&mut self, delta_time: Duration) {
        self.last = self.now;
        self.delta = delta_time;
        self.now += delta_time;
    }
    
    pub fn step(&mut self) {
        let now = Instant::now();
        self.last = self.now;
        self.delta = now.duration_since(self.now);
        self.now = now;
    }
}

impl Default for TimeFrame {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("🧪 Running RustFlow Comprehensive Test Suite");
    println!("=============================================");

    test_transient_id_uniqueness();
    test_transient_id_display();
    test_generator_state_display();
    test_time_frame_creation();
    test_time_frame_update();
    test_time_frame_step();
    test_debug_level_values();
    test_log_level_values();
    test_time_frame_monotonic_updates();
    test_multiple_transient_ids();
    test_generator_state_transitions();
    property_transient_id_always_increases();
    property_time_frame_delta_accumulates();
    stress_test_many_transient_ids();
    performance_test_transient_id_creation();
    performance_test_time_frame_updates();

    println!("\n✅ All tests passed!");
    println!("📊 Test Summary:");
    println!("  - Unit Tests: 11 passed");
    println!("  - Property Tests: 2 passed");  
    println!("  - Stress Tests: 1 passed");
    println!("  - Performance Tests: 2 passed");
    println!("  - Total: 16 tests passed");
}

fn test_transient_id_uniqueness() {
    print!("🔍 Testing TransientId uniqueness... ");
    let id1 = TransientId::new();
    let id2 = TransientId::new();
    
    assert_ne!(id1.value(), id2.value());
    assert!(id2.value() > id1.value());
    println!("✓");
}

fn test_transient_id_display() {
    print!("🔍 Testing TransientId display... ");
    let id = TransientId::new();
    let display_string = format!("{}", id);
    assert!(display_string.starts_with('#'));
    assert!(display_string.len() > 1);
    println!("✓");
}

fn test_generator_state_display() {
    print!("🔍 Testing GeneratorState display... ");
    assert_eq!(format!("{}", GeneratorState::Inactive), "Inactive");
    assert_eq!(format!("{}", GeneratorState::Running), "Running");
    assert_eq!(format!("{}", GeneratorState::Suspended), "Suspended");
    assert_eq!(format!("{}", GeneratorState::Completed), "Completed");
    println!("✓");
}

fn test_time_frame_creation() {
    print!("🔍 Testing TimeFrame creation... ");
    let time_frame = TimeFrame::new();
    assert_eq!(time_frame.delta, Duration::from_secs(0));
    assert_eq!(time_frame.now, time_frame.last);
    println!("✓");
}

fn test_time_frame_update() {
    print!("🔍 Testing TimeFrame update... ");
    let mut time_frame = TimeFrame::new();
    let original_now = time_frame.now;
    
    let delta = Duration::from_millis(16);
    time_frame.update(delta);
    
    assert_eq!(time_frame.delta, delta);
    assert_eq!(time_frame.last, original_now);
    assert_eq!(time_frame.now, original_now + delta);
    println!("✓");
}

fn test_time_frame_step() {
    print!("🔍 Testing TimeFrame step... ");
    let mut time_frame = TimeFrame::new();
    let original_now = time_frame.now;
    
    std::thread::sleep(Duration::from_millis(1));
    time_frame.step();
    
    assert!(time_frame.delta > Duration::from_secs(0));
    assert_eq!(time_frame.last, original_now);
    assert!(time_frame.now > original_now);
    println!("✓");
}

fn test_debug_level_values() {
    print!("🔍 Testing DebugLevel values... ");
    assert_eq!(DebugLevel::None as u8, 0);
    assert_eq!(DebugLevel::Low as u8, 1);
    assert_eq!(DebugLevel::Medium as u8, 2);
    assert_eq!(DebugLevel::High as u8, 3);
    println!("✓");
}

fn test_log_level_values() {
    print!("🔍 Testing LogLevel values... ");
    assert_eq!(LogLevel::Error as u8, 0);
    assert_eq!(LogLevel::Warn as u8, 1);
    assert_eq!(LogLevel::Info as u8, 2);
    assert_eq!(LogLevel::Debug as u8, 3);
    assert_eq!(LogLevel::Trace as u8, 4);
    println!("✓");
}

fn test_time_frame_monotonic_updates() {
    print!("🔍 Testing TimeFrame monotonic updates... ");
    let mut time_frame = TimeFrame::new();
    let start_time = time_frame.now;
    
    let deltas = [10, 15, 20, 25, 30];  // milliseconds
    let mut expected_time = start_time;
    
    for &delta_ms in &deltas {
        let delta = Duration::from_millis(delta_ms);
        expected_time += delta;
        
        time_frame.update(delta);
        
        assert_eq!(time_frame.now, expected_time);
        assert_eq!(time_frame.delta, delta);
    }
    println!("✓");
}

fn test_multiple_transient_ids() {
    print!("🔍 Testing multiple TransientId creation... ");
    let count = 100;
    let mut ids = Vec::new();
    
    for _ in 0..count {
        ids.push(TransientId::new());
    }
    
    // All IDs should be unique
    for i in 0..ids.len() {
        for j in i + 1..ids.len() {
            assert_ne!(ids[i].value(), ids[j].value());
        }
    }
    
    // IDs should be monotonically increasing
    for i in 1..ids.len() {
        assert!(ids[i].value() > ids[i - 1].value());
    }
    println!("✓");
}

fn test_generator_state_transitions() {
    print!("🔍 Testing GeneratorState transitions... ");
    use GeneratorState::*;
    
    // Test all states can be created
    let states = [Inactive, Running, Suspended, Completed];
    
    for &state in &states {
        match state {
            Inactive => assert_eq!(format!("{}", state), "Inactive"),
            Running => assert_eq!(format!("{}", state), "Running"),
            Suspended => assert_eq!(format!("{}", state), "Suspended"),
            Completed => assert_eq!(format!("{}", state), "Completed"),
        }
    }
    println!("✓");
}

// Property-based tests
fn property_transient_id_always_increases() {
    print!("🔬 Property test: TransientId always increases... ");
    let mut last_id = TransientId::new().value();
    
    for _ in 0..1000 {
        let new_id = TransientId::new().value();
        assert!(new_id > last_id);
        last_id = new_id;
    }
    println!("✓");
}

fn property_time_frame_delta_accumulates() {
    print!("🔬 Property test: TimeFrame delta accumulation... ");
    let mut time_frame = TimeFrame::new();
    let initial_time = time_frame.now;
    let mut total_delta = Duration::from_secs(0);
    
    let test_deltas = [1, 16, 33, 50, 100]; // milliseconds
    
    for &delta_ms in &test_deltas {
        let delta = Duration::from_millis(delta_ms);
        total_delta += delta;
        time_frame.update(delta);
    }
    
    assert_eq!(time_frame.now, initial_time + total_delta);
    println!("✓");
}

// Stress test
fn stress_test_many_transient_ids() {
    print!("💪 Stress test: Creating 10,000 unique TransientIds... ");
    use std::collections::HashSet;
    
    let count = 10000;
    let mut ids = HashSet::new();
    
    for _ in 0..count {
        let id = TransientId::new();
        assert!(ids.insert(id.value()), "Duplicate ID found");
    }
    
    assert_eq!(ids.len(), count);
    println!("✓");
}

// Performance tests
fn performance_test_transient_id_creation() {
    print!("⚡ Performance test: TransientId creation speed... ");
    let start = std::time::Instant::now();
    
    for _ in 0..10000 {
        let _id = TransientId::new();
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(100), "ID creation too slow: {:?}", elapsed);
    println!("✓ ({}μs per ID)", elapsed.as_micros() / 10000);
}

fn performance_test_time_frame_updates() {
    print!("⚡ Performance test: TimeFrame update speed... ");
    let mut time_frame = TimeFrame::new();
    let start = std::time::Instant::now();
    let delta = Duration::from_millis(1);
    
    for _ in 0..10000 {
        time_frame.update(delta);
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(100), "TimeFrame updates too slow: {:?}", elapsed);
    println!("✓ ({}ns per update)", elapsed.as_nanos() / 10000);
}