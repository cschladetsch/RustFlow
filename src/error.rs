#[derive(thiserror::Error, Debug)]
pub enum FlowError {
    #[error("Generator error: {0}")]
    Generator(String),
    
    #[error("Kernel error: {0}")]
    Kernel(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Threading error: {0}")]
    Threading(String),
    
    #[error("Channel error: {0}")]
    Channel(String),
    
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    
    #[error("Send error")]
    Send,
    
    #[error("Receive error")]
    Recv,
}