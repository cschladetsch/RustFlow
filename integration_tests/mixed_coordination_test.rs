// Mixed coroutine+threading coordination tests for RustFlow
// Tests interaction between async/await and standard threading

use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock as AsyncRwLock};
use tokio::time::sleep;
use std::sync::atomic::{AtomicUsize, Ordering};
use crossbeam::channel::{bounded, unbounded};

// Hybrid executor that runs both sync and async work
struct HybridExecutor {
    thread_pool_size: usize,
    async_runtime: tokio::runtime::Runtime,
}

impl HybridExecutor {
    fn new(thread_pool_size: usize) -> Self {
        let async_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
            
        Self {
            thread_pool_size,
            async_runtime,
        }
    }
    
    fn execute_mixed_workload(&self) -> (Vec<i32>, Vec<i32>) {
        let sync_results = Arc::new(Mutex::new(Vec::new()));
        let async_results = Arc::new(AsyncRwLock::new(Vec::new()));
        
        // Launch sync threads
        let mut sync_handles = Vec::new();
        for i in 0..self.thread_pool_size {
            let sync_results = sync_results.clone();
            let handle = thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                let result = i * i;
                sync_results.lock().unwrap().push(result);
                result
            });
            sync_handles.push(handle);
        }
        
        // Launch async tasks
        let async_future = {
            let async_results = async_results.clone();
            self.async_runtime.spawn(async move {
                let mut tasks = Vec::new();
                
                for i in 0..4 {
                    let async_results = async_results.clone();
                    let task = tokio::spawn(async move {
                        sleep(Duration::from_millis(15)).await;
                        let result = (i + 10) * (i + 10);
                        async_results.write().await.push(result);
                        result
                    });
                    tasks.push(task);
                }
                
                // Wait for all async tasks
                for task in tasks {
                    task.await.unwrap();
                }
            })
        };
        
        // Wait for all sync threads
        for handle in sync_handles {
            handle.join().unwrap();
        }
        
        // Wait for async tasks
        self.async_runtime.block_on(async_future).unwrap();
        
        let sync_final = sync_results.lock().unwrap().clone();
        let async_final = self.async_runtime.block_on(async move {
            async_results.read().await.clone()
        });
        
        (sync_final, async_final)
    }
}

// Bridge between sync and async worlds
struct SyncAsyncBridge {
    sync_to_async_sender: crossbeam::channel::Sender<i32>,
    async_to_sync_receiver: std::sync::mpsc::Receiver<i32>,
}

impl SyncAsyncBridge {
    fn new() -> (Self, crossbeam::channel::Receiver<i32>, tokio::sync::mpsc::UnboundedSender<i32>) {
        let (sync_to_async_sender, sync_to_async_receiver) = unbounded();
        let (async_to_sync_sender, async_to_sync_receiver) = std::sync::mpsc::channel();
        let (async_sender, _) = tokio::sync::mpsc::unbounded_channel();
        
        (
            Self {
                sync_to_async_sender,
                async_to_sync_receiver,
            },
            sync_to_async_receiver,
            async_sender,
        )
    }
    
    fn send_to_async(&self, value: i32) {
        self.sync_to_async_sender.send(value).unwrap();
    }
    
    fn receive_from_async(&self) -> Result<i32, std::sync::mpsc::RecvError> {
        self.async_to_sync_receiver.recv()
    }
}

#[tokio::main]
async fn main() {
    println!("🔄 RustFlow Mixed Coroutine+Threading Coordination Tests");
    println!("=========================================================");
    
    // Test 1: Basic sync-async interoperation
    test_basic_sync_async_interop().await;
    
    // Test 2: Hybrid executor
    test_hybrid_executor().await;
    
    // Test 3: Cross-boundary communication
    test_cross_boundary_communication().await;
    
    // Test 4: Mixed blocking and non-blocking operations
    test_mixed_blocking_operations().await;
    
    // Test 5: Tokio runtime with thread spawning
    test_tokio_with_threads().await;
    
    // Test 6: Performance comparison between approaches
    test_performance_comparison().await;
    
    // Test 7: Complex coordination patterns
    test_complex_mixed_patterns().await;
    
    println!("\n✅ All mixed coordination tests completed successfully!");
    println!("📊 Mixed Coordination Test Summary:");
    println!("  - Basic sync-async interop: ✓");
    println!("  - Hybrid executor: ✓");
    println!("  - Cross-boundary communication: ✓");
    println!("  - Mixed blocking operations: ✓");
    println!("  - Tokio with threads: ✓");
    println!("  - Performance comparison: ✓");
    println!("  - Complex mixed patterns: ✓");
    println!("  - Total: 7 tests passed");
}

async fn test_basic_sync_async_interop() {
    print!("🔍 Testing basic sync-async interoperation... ");
    
    let sync_result = Arc::new(Mutex::new(0));
    let async_result = Arc::new(AsyncRwLock::new(0));
    
    // Spawn sync thread that modifies shared state
    let sync_handle = {
        let sync_result = sync_result.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            *sync_result.lock().unwrap() = 42;
        })
    };
    
    // Run async task that also modifies shared state
    let async_task = {
        let async_result = async_result.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            *async_result.write().await = 24;
        })
    };
    
    // Wait for both to complete
    sync_handle.join().unwrap();
    async_task.await.unwrap();
    
    assert_eq!(*sync_result.lock().unwrap(), 42);
    assert_eq!(*async_result.read().await, 24);
    
    println!("✓");
}

async fn test_hybrid_executor() {
    print!("🔍 Testing hybrid executor... ");
    
    let executor = HybridExecutor::new(3);
    let (sync_results, async_results) = executor.execute_mixed_workload();
    
    assert_eq!(sync_results.len(), 3);
    assert_eq!(async_results.len(), 4);
    
    // Verify sync results (i^2 for i in 0..3)
    let expected_sync: Vec<i32> = (0..3).map(|i| i * i).collect();
    let mut sorted_sync = sync_results.clone();
    sorted_sync.sort();
    assert_eq!(sorted_sync, expected_sync);
    
    // Verify async results ((i+10)^2 for i in 0..4)
    let expected_async: Vec<i32> = (0..4).map(|i| (i + 10) * (i + 10)).collect();
    let mut sorted_async = async_results.clone();
    sorted_async.sort();
    assert_eq!(sorted_async, expected_async);
    
    println!("✓");
}

async fn test_cross_boundary_communication() {
    print!("🔍 Testing cross-boundary communication... ");
    
    let (tx_to_async, rx_from_sync) = tokio::sync::mpsc::unbounded_channel();
    let (tx_to_sync, rx_from_async) = std::sync::mpsc::channel();
    
    // Spawn sync thread that sends to async and receives from async
    let sync_handle = thread::spawn(move || {
        // Send messages to async
        for i in 0..5 {
            tx_to_async.send(i * 2).unwrap();
        }
        
        // Receive messages from async
        let mut received = Vec::new();
        for _ in 0..5 {
            received.push(rx_from_async.recv().unwrap());
        }
        received
    });
    
    // Async task that receives from sync and sends to sync
    let async_task = tokio::spawn(async move {
        let mut rx_from_sync = rx_from_sync;
        let mut received = Vec::new();
        
        // Receive from sync thread
        for _ in 0..5 {
            if let Some(value) = rx_from_sync.recv().await {
                received.push(value);
                // Send processed value back to sync
                tx_to_sync.send(value * 3).unwrap();
            }
        }
        received
    });
    
    let sync_received = sync_handle.join().unwrap();
    let async_received = async_task.await.unwrap();
    
    // Verify message passing worked correctly
    assert_eq!(async_received, vec![0, 2, 4, 6, 8]);
    assert_eq!(sync_received, vec![0, 6, 12, 18, 24]);
    
    println!("✓");
}

async fn test_mixed_blocking_operations() {
    print!("🔍 Testing mixed blocking and non-blocking operations... ");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();
    
    // Spawn blocking thread operations
    let mut sync_handles = Vec::new();
    for i in 0..3 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            // Simulate blocking I/O
            thread::sleep(Duration::from_millis(30));
            counter.fetch_add(i + 1, Ordering::SeqCst);
        });
        sync_handles.push(handle);
    }
    
    // Run non-blocking async operations concurrently
    let mut async_tasks = Vec::new();
    for i in 0..4 {
        let counter = counter.clone();
        let task = tokio::spawn(async move {
            // Non-blocking delay
            sleep(Duration::from_millis(10)).await;
            counter.fetch_add((i + 1) * 10, Ordering::SeqCst);
        });
        async_tasks.push(task);
    }
    
    // Wait for all operations
    for handle in sync_handles {
        handle.join().unwrap();
    }
    
    for task in async_tasks {
        task.await.unwrap();
    }
    
    let elapsed = start_time.elapsed();
    let final_count = counter.load(Ordering::SeqCst);
    
    // Sync threads added: 1 + 2 + 3 = 6
    // Async tasks added: 10 + 20 + 30 + 40 = 100
    // Total: 106
    assert_eq!(final_count, 106);
    
    // Should complete faster than if everything was sequential
    assert!(elapsed < Duration::from_millis(100));
    
    println!("✓");
}

async fn test_tokio_with_threads() {
    print!("🔍 Testing Tokio runtime with thread spawning... ");
    
    let shared_data = Arc::new(Mutex::new(Vec::new()));
    let barrier = Arc::new(Barrier::new(6)); // 3 threads + 3 async tasks
    
    // Spawn regular threads within tokio context
    let mut thread_handles = Vec::new();
    for i in 0..3 {
        let shared_data = shared_data.clone();
        let barrier = barrier.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(i * 5));
            shared_data.lock().unwrap().push(format!("thread_{}", i));
            barrier.wait();
        });
        thread_handles.push(handle);
    }
    
    // Spawn async tasks
    let mut async_tasks = Vec::new();
    for i in 0..3 {
        let shared_data = shared_data.clone();
        let barrier = barrier.clone();
        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(i * 5)).await;
            shared_data.lock().unwrap().push(format!("async_{}", i));
            barrier.wait();
        });
        async_tasks.push(task);
    }
    
    // Wait for all to complete
    for handle in thread_handles {
        handle.join().unwrap();
    }
    
    for task in async_tasks {
        task.await.unwrap();
    }
    
    let data = shared_data.lock().unwrap();
    assert_eq!(data.len(), 6);
    
    // Should have both thread and async entries
    let thread_count = data.iter().filter(|s| s.starts_with("thread_")).count();
    let async_count = data.iter().filter(|s| s.starts_with("async_")).count();
    assert_eq!(thread_count, 3);
    assert_eq!(async_count, 3);
    
    println!("✓");
}

async fn test_performance_comparison() {
    print!("⚡ Testing performance comparison between approaches... ");
    
    let iterations = 1000;
    
    // Test pure threading approach
    let thread_start = Instant::now();
    let counter_threads = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    for _ in 0..10 {
        let counter = counter_threads.clone();
        let handle = thread::spawn(move || {
            for _ in 0..iterations / 10 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let thread_duration = thread_start.elapsed();
    
    // Test pure async approach
    let async_start = Instant::now();
    let counter_async = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    
    for _ in 0..10 {
        let counter = counter_async.clone();
        let task = tokio::spawn(async move {
            for _ in 0..iterations / 10 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        tasks.push(task);
    }
    
    for task in tasks {
        task.await.unwrap();
    }
    
    let async_duration = async_start.elapsed();
    
    // Test mixed approach
    let mixed_start = Instant::now();
    let counter_mixed = Arc::new(AtomicUsize::new(0));
    
    // Half threads, half async
    let mut mixed_handles = Vec::new();
    for _ in 0..5 {
        let counter = counter_mixed.clone();
        let handle = thread::spawn(move || {
            for _ in 0..iterations / 10 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        mixed_handles.push(handle);
    }
    
    let mut mixed_tasks = Vec::new();
    for _ in 0..5 {
        let counter = counter_mixed.clone();
        let task = tokio::spawn(async move {
            for _ in 0..iterations / 10 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        mixed_tasks.push(task);
    }
    
    for handle in mixed_handles {
        handle.join().unwrap();
    }
    
    for task in mixed_tasks {
        task.await.unwrap();
    }
    
    let mixed_duration = mixed_start.elapsed();
    
    // Verify all approaches computed the same result
    assert_eq!(counter_threads.load(Ordering::SeqCst), iterations);
    assert_eq!(counter_async.load(Ordering::SeqCst), iterations);
    assert_eq!(counter_mixed.load(Ordering::SeqCst), iterations);
    
    println!("✓ (threads: {:?}, async: {:?}, mixed: {:?})", 
             thread_duration, async_duration, mixed_duration);
}

async fn test_complex_mixed_patterns() {
    print!("🔍 Testing complex mixed coordination patterns... ");
    
    // Producer-consumer with both sync and async components
    let (async_tx, mut async_rx) = tokio::sync::mpsc::unbounded_channel();
    let (sync_tx, sync_rx) = crossbeam::channel::unbounded();
    
    let results = Arc::new(Mutex::new((Vec::new(), Vec::new())));
    
    // Sync producer
    let sync_producer = {
        let sync_tx = sync_tx.clone();
        thread::spawn(move || {
            for i in 0..10 {
                sync_tx.send(format!("sync_{}", i)).unwrap();
                thread::sleep(Duration::from_millis(2));
            }
        })
    };
    
    // Async producer
    let async_producer = tokio::spawn(async move {
        for i in 0..10 {
            async_tx.send(format!("async_{}", i)).unwrap();
            sleep(Duration::from_millis(1)).await;
        }
    });
    
    // Mixed consumer (async task that also spawns threads)
    let consumer = {
        let results = results.clone();
        tokio::spawn(async move {
            let mut async_messages = Vec::new();
            let sync_messages = Arc::new(Mutex::new(Vec::new()));
            
            // Spawn thread to consume sync messages
            let sync_consumer = {
                let sync_messages = sync_messages.clone();
                thread::spawn(move || {
                    while let Ok(msg) = sync_rx.recv() {
                        sync_messages.lock().unwrap().push(msg);
                    }
                })
            };
            
            // Consume async messages
            while let Some(msg) = async_rx.recv().await {
                async_messages.push(msg);
                if async_messages.len() >= 10 {
                    break;
                }
            }
            
            // Wait for sync consumer
            sync_consumer.join().unwrap();
            
            let sync_final = sync_messages.lock().unwrap().clone();
            results.lock().unwrap().0 = sync_final;
            results.lock().unwrap().1 = async_messages;
        })
    };
    
    // Wait for producers and consumer
    sync_producer.join().unwrap();
    async_producer.await.unwrap();
    consumer.await.unwrap();
    
    let (sync_results, async_results) = &*results.lock().unwrap();
    
    assert_eq!(sync_results.len(), 10);
    assert_eq!(async_results.len(), 10);
    
    // Verify message format
    assert!(sync_results.iter().all(|s| s.starts_with("sync_")));
    assert!(async_results.iter().all(|s| s.starts_with("async_")));
    
    println!("✓");
}