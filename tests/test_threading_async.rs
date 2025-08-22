use rust_flow::*;
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_test;

#[tokio::test]
#[serial]
async fn test_threaded_executor_basic() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut executor = threading::ThreadedExecutor::new(2);
    
    let work_counter = Arc::clone(&counter);
    let result = executor.start(move |thread_id| {
        work_counter.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(1));
    });
    
    assert!(result.is_ok());
    
    // Let it run for a bit
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let result = executor.shutdown();
    assert!(result.is_ok());
    
    // Should have done some work
    assert!(counter.load(Ordering::Relaxed) > 0);
}

#[tokio::test]
#[serial]
async fn test_thread_pool_execution() {
    let pool = threading::ThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));
    
    // Submit multiple tasks
    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        let result = pool.execute(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert!(result.is_ok());
    }
    
    // Give tasks time to complete
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // All tasks should have executed
    assert_eq!(counter.load(Ordering::Relaxed), 10);
}

#[tokio::test]
#[serial]
async fn test_async_future_coordination() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    
    // Create multiple futures that will be set from different tasks
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    let future1 = factory.future::<i32>();
    let future2 = factory.future::<String>();
    let future3 = factory.future::<bool>();
    
    // Add futures to runtime
    {
        let mut root_guard = root.write().await;
        root_guard.add(future1.clone() as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(future2.clone() as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(future3.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Spawn tasks to set future values
    tokio::spawn({
        let future1 = Arc::clone(&future1);
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut f = future1.write().await;
            let _ = f.set_value(42).await;
        }
    });
    
    tokio::spawn({
        let future2 = Arc::clone(&future2);
        async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let mut f = future2.write().await;
            let _ = f.set_value("Hello".to_string()).await;
        }
    });
    
    tokio::spawn({
        let future3 = Arc::clone(&future3);
        async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            let mut f = future3.write().await;
            let _ = f.set_value(true).await;
        }
    });
    
    // Run until all futures complete
    let result = runtime.run_for(Duration::from_millis(100), Duration::from_millis(5)).await;
    assert!(result.is_ok());
    
    // Check all futures were set
    assert!(future1.read().await.is_ready());
    assert!(future2.read().await.is_ready());
    assert!(future3.read().await.is_ready());
    
    assert_eq!(future1.read().await.value(), Some(&42));
    assert_eq!(future2.read().await.value(), Some(&"Hello".to_string()));
    assert_eq!(future3.read().await.value(), Some(&true));
}

#[tokio::test]
#[serial]
async fn test_barrier_with_async_dependencies() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a barrier with multiple async dependencies
    let barrier = factory.barrier();
    let future1 = factory.future::<i32>();
    let future2 = factory.future::<String>();
    let timer = factory.timer(Duration::from_millis(25));
    
    // Set up barrier dependencies
    {
        let mut barrier_guard = barrier.write().await;
        barrier_guard.add_dependency(future1.clone() as Arc<RwLock<dyn Transient>>).await;
        barrier_guard.add_dependency(future2.clone() as Arc<RwLock<dyn Transient>>).await;
        barrier_guard.add_dependency(timer as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Add barrier to runtime
    {
        let mut root_guard = root.write().await;
        root_guard.add(barrier.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Complete dependencies at different times
    tokio::spawn({
        let future1 = Arc::clone(&future1);
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut f = future1.write().await;
            let _ = f.set_value(123).await;
        }
    });
    
    tokio::spawn({
        let future2 = Arc::clone(&future2);
        async move {
            tokio::time::sleep(Duration::from_millis(15)).await;
            let mut f = future2.write().await;
            let _ = f.set_value("World".to_string()).await;
        }
    });
    
    // Run until barrier completes
    let result = runtime.run_for(Duration::from_millis(50), Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Barrier should be completed
    {
        let barrier_guard = barrier.read().await;
        assert!(barrier_guard.all_completed());
        assert!(barrier_guard.is_completed());
    }
}

#[tokio::test]
#[serial]
async fn test_channel_async_communication() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    let channel = factory.channel::<i32>();
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(channel.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Spawn producer
    tokio::spawn({
        let channel = Arc::clone(&channel);
        async move {
            for i in 0..5 {
                tokio::time::sleep(Duration::from_millis(5)).await;
                let channel_guard = channel.read().await;
                let _ = channel_guard.send(i).await;
            }
        }
    });
    
    // Run and collect received values
    let mut received_values = Vec::new();
    
    for _ in 0..10 {
        {
            let mut channel_guard = channel.write().await;
            let _ = channel_guard.step().await;
            
            if let Some(Some(value)) = channel_guard.value() {
                received_values.push(*value);
            }
        }
        
        let _ = runtime.run_frame(Duration::from_millis(5)).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    
    // Should have received some values
    assert!(!received_values.is_empty());
}

#[tokio::test]
#[serial]
async fn test_timed_future_timeout_coordination() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create timed futures with different timeouts
    let fast_future = factory.timed_future::<i32>(Duration::from_millis(10));
    let slow_future = factory.timed_future::<i32>(Duration::from_millis(50));
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(fast_future.clone() as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(slow_future.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run for intermediate time
    let result = runtime.run_for(Duration::from_millis(25), Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Fast future should have timed out, slow one should not
    {
        let fast_guard = fast_future.read().await;
        let slow_guard = slow_future.read().await;
        
        assert!(fast_guard.is_timed_out());
        assert!(fast_guard.is_completed());
        assert!(!slow_guard.is_timed_out());
        assert!(slow_guard.is_active());
    }
}

#[tokio::test]
#[serial]
async fn test_sequence_with_async_steps() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create sequence with async steps
    let sequence = factory.sequence();
    let future1 = factory.future::<i32>();
    let timer = factory.timer(Duration::from_millis(20));
    let future2 = factory.future::<String>();
    
    {
        let mut seq_guard = sequence.write().await;
        seq_guard.add_step(future1.clone() as Arc<RwLock<dyn Transient>>).await;
        seq_guard.add_step(timer as Arc<RwLock<dyn Transient>>).await;
        seq_guard.add_step(future2.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(sequence.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Complete first future to advance sequence
    tokio::spawn({
        let future1 = Arc::clone(&future1);
        async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let mut f = future1.write().await;
            let _ = f.set_value(100).await;
        }
    });
    
    // Complete final future after timer should expire
    tokio::spawn({
        let future2 = Arc::clone(&future2);
        async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            let mut f = future2.write().await;
            let _ = f.set_value("Done".to_string()).await;
        }
    });
    
    // Run until sequence completes
    let result = runtime.run_for(Duration::from_millis(50), Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Sequence should have progressed through all steps
    {
        let seq_guard = sequence.read().await;
        // Should be completed or on the last step
        assert!(seq_guard.current_step_index().is_none() || seq_guard.current_step_index() == Some(2));
    }
}

#[tokio::test]
#[serial]
async fn test_mixed_sync_async_workload() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create mixed workload
    let node = factory.node();
    let future = factory.future::<i32>();
    let timer = factory.timer(Duration::from_millis(20));
    let barrier = factory.barrier();
    
    // Set up barrier with dependencies
    {
        let mut barrier_guard = barrier.write().await;
        barrier_guard.add_dependency(future.clone() as Arc<RwLock<dyn Transient>>).await;
        barrier_guard.add_dependency(timer as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Add components to node
    {
        let mut node_guard = node.write().await;
        node_guard.add(barrier as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Add node to root
    {
        let mut root_guard = root.write().await;
        root_guard.add(node as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Start thread pool work
    let thread_pool = threading::ThreadPool::new(2);
    let counter = Arc::new(AtomicUsize::new(0));
    
    for i in 0..5 {
        let counter = Arc::clone(&counter);
        let _ = thread_pool.execute(move || {
            // Simulate work
            std::thread::sleep(Duration::from_millis(5));
            counter.fetch_add(i, Ordering::Relaxed);
        });
    }
    
    // Complete future from async context
    tokio::spawn({
        let future = Arc::clone(&future);
        let counter = Arc::clone(&counter);
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let value = counter.load(Ordering::Relaxed) as i32;
            let mut f = future.write().await;
            let _ = f.set_value(value).await;
        }
    });
    
    // Run the flow system
    let result = runtime.run_for(Duration::from_millis(50), Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Both async and threaded work should have completed
    assert!(counter.load(Ordering::Relaxed) > 0);
    assert!(future.read().await.is_ready());
}