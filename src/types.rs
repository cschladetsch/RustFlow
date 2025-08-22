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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_transient_id_creation() {
        let id1 = TransientId::new();
        let id2 = TransientId::new();
        
        assert_ne!(id1.value(), id2.value());
        assert!(id2.value() > id1.value());
    }

    #[test]
    fn test_transient_id_display() {
        let id = TransientId::new();
        let display_string = format!("{}", id);
        assert!(display_string.starts_with('#'));
        assert!(display_string.len() > 1);
    }

    #[test]
    fn test_generator_state_display() {
        assert_eq!(format!("{}", GeneratorState::Inactive), "Inactive");
        assert_eq!(format!("{}", GeneratorState::Running), "Running");
        assert_eq!(format!("{}", GeneratorState::Suspended), "Suspended");
        assert_eq!(format!("{}", GeneratorState::Completed), "Completed");
    }

    #[test]
    fn test_time_frame_creation() {
        let time_frame = TimeFrame::new();
        assert_eq!(time_frame.delta, Duration::from_secs(0));
        assert_eq!(time_frame.now, time_frame.last);
    }

    #[test]
    fn test_time_frame_update() {
        let mut time_frame = TimeFrame::new();
        let original_now = time_frame.now;
        
        let delta = Duration::from_millis(16);
        time_frame.update(delta);
        
        assert_eq!(time_frame.delta, delta);
        assert_eq!(time_frame.last, original_now);
        assert_eq!(time_frame.now, original_now + delta);
    }

    #[test]
    fn test_time_frame_step() {
        let mut time_frame = TimeFrame::new();
        let original_now = time_frame.now;
        
        std::thread::sleep(Duration::from_millis(1));
        time_frame.step();
        
        assert!(time_frame.delta > Duration::from_secs(0));
        assert_eq!(time_frame.last, original_now);
        assert!(time_frame.now > original_now);
    }

    #[test]
    fn test_debug_level_values() {
        assert_eq!(DebugLevel::None as u8, 0);
        assert_eq!(DebugLevel::Low as u8, 1);
        assert_eq!(DebugLevel::Medium as u8, 2);
        assert_eq!(DebugLevel::High as u8, 3);
    }

    #[test]
    fn test_log_level_values() {
        assert_eq!(LogLevel::Error as u8, 0);
        assert_eq!(LogLevel::Warn as u8, 1);
        assert_eq!(LogLevel::Info as u8, 2);
        assert_eq!(LogLevel::Debug as u8, 3);
        assert_eq!(LogLevel::Trace as u8, 4);
    }
}