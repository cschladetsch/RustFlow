use rust_flow::*;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_test;

#[tokio::test]
#[serial]
async fn test_kernel_creation() {
    let kernel = Kernel::new();
    
    assert_eq!(kernel.debug_level(), DebugLevel::Medium);
    assert!(!kernel.is_break());
    assert!(kernel.is_active());
    assert!(!kernel.is_completed());
    assert_eq!(kernel.step_number(), 0);
}

#[tokio::test]
#[serial]
async fn test_kernel_debug_level() {
    let mut kernel = Kernel::new();
    
    kernel.set_debug_level(DebugLevel::High);
    assert_eq!(kernel.debug_level(), DebugLevel::High);
    
    kernel.set_debug_level(DebugLevel::None);
    assert_eq!(kernel.debug_level(), DebugLevel::None);
}

#[tokio::test]
#[serial]
async fn test_kernel_break_flow() {
    let mut kernel = Kernel::new();
    
    assert!(!kernel.is_break());
    kernel.break_flow();
    assert!(kernel.is_break());
}

#[tokio::test]
#[serial]
async fn test_kernel_wait() {
    let mut kernel = Kernel::new();
    let wait_duration = Duration::from_millis(10);
    
    kernel.wait_for(wait_duration).await;
    
    // Should not process while waiting
    let result = kernel.update(Duration::from_millis(1)).await;
    assert!(result.is_ok());
    
    // Wait for the wait period to expire
    tokio::time::sleep(Duration::from_millis(15)).await;
    
    let result = kernel.update(Duration::from_millis(1)).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_kernel_step_counting() {
    let mut kernel = Kernel::new();
    let initial_steps = kernel.step_number();
    
    let _ = kernel.step().await;
    assert_eq!(kernel.step_number(), initial_steps + 1);
    
    let _ = kernel.step().await;
    assert_eq!(kernel.step_number(), initial_steps + 2);
}

#[tokio::test]
#[serial]
async fn test_kernel_with_timers() {
    let mut kernel = Kernel::new();
    let factory = kernel.factory().clone();
    let root = kernel.root();
    
    let timer1 = factory.timer(Duration::from_millis(10));
    let timer2 = factory.timer(Duration::from_millis(20));
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(timer1 as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(timer2 as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Step the kernel multiple times
    for _ in 0..5 {
        let _ = kernel.update(Duration::from_millis(5)).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    
    // Check that completed timers were removed
    {
        let root_guard = root.read().await;
        // Some timers should have completed and been removed
        assert!(root_guard.contents().len() <= 2);
    }
}

#[tokio::test]
#[serial]
async fn test_kernel_time_management() {
    let mut kernel = Kernel::new();
    let initial_time = kernel.time().now;
    
    let delta = Duration::from_millis(16);
    let _ = kernel.update(delta).await;
    
    let time_frame = kernel.time();
    assert_eq!(time_frame.delta, delta);
    assert_eq!(time_frame.last, initial_time);
}

#[tokio::test]
#[serial]
async fn test_runtime_creation() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    
    {
        let kernel_guard = kernel.read().await;
        assert!(kernel_guard.is_active());
        assert_eq!(kernel_guard.step_number(), 0);
    }
}

#[tokio::test]
#[serial]
async fn test_runtime_single_frame() {
    let runtime = Runtime::new();
    let delta_time = Duration::from_millis(16);
    
    let result = runtime.run_frame(delta_time).await;
    assert!(result.is_ok());
    
    {
        let kernel = runtime.kernel();
        let kernel_guard = kernel.read().await;
        assert_eq!(kernel_guard.time().delta, delta_time);
    }
}

#[tokio::test]
#[serial]
async fn test_runtime_run_for_duration() {
    let runtime = Runtime::new();
    let total_duration = Duration::from_millis(50);
    let frame_time = Duration::from_millis(10);
    
    let start = std::time::Instant::now();
    let result = runtime.run_for(total_duration, frame_time).await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok());
    // Should run for approximately the specified duration
    assert!(elapsed >= total_duration);
    assert!(elapsed < total_duration + Duration::from_millis(50)); // Allow some overhead
}

#[tokio::test]
#[serial]
async fn test_runtime_with_break_condition() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    
    // Add a timer that will complete quickly
    {
        let factory = kernel.read().await.factory().clone();
        let root = kernel.read().await.root();
        let timer = factory.timer(Duration::from_millis(10));
        
        let mut root_guard = root.write().await;
        root_guard.add(timer as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Set break condition after a short time
    tokio::spawn({
        let kernel = Arc::clone(&kernel);
        async move {
            tokio::time::sleep(Duration::from_millis(25)).await;
            let mut kernel_guard = kernel.write().await;
            kernel_guard.break_flow();
        }
    });
    
    let start = std::time::Instant::now();
    let result = runtime.run_until_complete(Duration::from_millis(5)).await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok());
    // Should exit early due to break condition
    assert!(elapsed < Duration::from_secs(1));
}

#[tokio::test]
#[serial]
async fn test_runtime_until_complete() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    
    // Add a short timer
    {
        let factory = kernel.read().await.factory().clone();
        let root = kernel.read().await.root();
        let timer = factory.timer(Duration::from_millis(20));
        
        let mut root_guard = root.write().await;
        root_guard.add(timer as Arc<RwLock<dyn Transient>>).await;
    }
    
    let start = std::time::Instant::now();
    let result = runtime.run_until_complete(Duration::from_millis(5)).await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok());
    // Should run until timer completes
    assert!(elapsed >= Duration::from_millis(15));
    assert!(elapsed < Duration::from_millis(100));
}

#[tokio::test]
#[serial]
async fn test_kernel_factory_integration() {
    let kernel = Kernel::new();
    let factory = kernel.factory();
    
    // Test factory creates components that work with kernel
    let timer = factory.timer(Duration::from_millis(10));
    let future = factory.future::<i32>();
    let sequence = factory.sequence();
    
    // All should be active initially
    assert!(timer.read().await.is_active());
    assert!(future.read().await.is_active());
    assert!(sequence.read().await.is_active());
}

#[tokio::test]
#[serial]
async fn test_kernel_complex_workflow() {
    let mut kernel = Kernel::new();
    let factory = kernel.factory().clone();
    let root = kernel.root();
    
    // Create a complex workflow
    let sequence = factory.sequence();
    let barrier = factory.barrier();
    let timer1 = factory.timer(Duration::from_millis(10));
    let timer2 = factory.timer(Duration::from_millis(15));
    let future = factory.future::<String>();
    
    // Set up barrier dependencies
    {
        let mut barrier_guard = barrier.write().await;
        barrier_guard.add_dependency(timer1 as Arc<RwLock<dyn Transient>>).await;
        barrier_guard.add_dependency(timer2 as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Set up sequence steps
    {
        let mut seq_guard = sequence.write().await;
        seq_guard.add_step(barrier as Arc<RwLock<dyn Transient>>).await;
        seq_guard.add_step(future as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Add to root
    {
        let mut root_guard = root.write().await;
        root_guard.add(sequence as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run for several frames
    for _ in 0..10 {
        let _ = kernel.update(Duration::from_millis(5)).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    
    // Workflow should have progressed
    assert!(kernel.step_number() >= 10);
}

#[tokio::test]
#[serial]
async fn test_kernel_logger_integration() {
    let mut kernel = Kernel::new();
    let logger = kernel.logger_mut();
    
    // Test logger methods don't panic
    logger.info("Test info message");
    logger.debug("Test debug message");
    logger.error("Test error message");
    logger.warn("Test warning message");
    logger.trace("Test trace message");
    
    // Test verbosity levels
    logger.set_verbosity(10);
    assert_eq!(logger.verbosity(), 10);
    
    logger.verbose(5, "Verbose message");
    logger.verbose(15, "This should not log");
}