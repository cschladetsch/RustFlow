use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

use crate::types::LogLevel;

#[derive(Clone)]
pub struct Logger {
    prefix: String,
    verbosity: u8,
}

impl Logger {
    pub fn new(prefix: String) -> Self {
        Self {
            prefix,
            verbosity: 5,
        }
    }
    
    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }
    
    pub fn set_verbosity(&mut self, verbosity: u8) {
        self.verbosity = verbosity;
    }
    
    pub fn error(&self, message: &str) {
        error!("[{}] {}", self.prefix, message);
    }
    
    pub fn warn(&self, message: &str) {
        warn!("[{}] {}", self.prefix, message);
    }
    
    pub fn info(&self, message: &str) {
        info!("[{}] {}", self.prefix, message);
    }
    
    pub fn debug(&self, message: &str) {
        debug!("[{}] {}", self.prefix, message);
    }
    
    pub fn trace(&self, message: &str) {
        trace!("[{}] {}", self.prefix, message);
    }
    
    pub fn verbose(&self, level: u8, message: &str) {
        if level <= self.verbosity {
            trace!("[{}] {}", self.prefix, message);
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("RustFlow".to_string())
    }
}