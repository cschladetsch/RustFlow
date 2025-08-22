// Comprehensive coroutine (async/await) testing for RustFlow
// Tests async functionality, futures, channels, and concurrent coordination

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Basic coroutine implementation for testing
struct AsyncCounter {
    count: Arc<Mutex<u32>>,
}

impl AsyncCounter {
    fn new() -> Self {
        Self {
            count: Arc::new(Mutex::new(0)),
        }
    }
    
    async fn increment(&self) -> u32 {
        // Simulate async work with a small delay
        sleep(Duration::from_millis(1)).await;
        let mut count = self.count.lock().unwrap();
        *count += 1;
        *count
    }
    
    async fn get(&self) -> u32 {
        *self.count.lock().unwrap()
    }
}

// Async future that completes after a timeout
struct TimedTask {
    duration: Duration,
    completed: Arc<RwLock<bool>>,
}

impl TimedTask {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            completed: Arc::new(RwLock::new(false)),
        }
    }
    
    async fn execute(&self) -> String {
        sleep(self.duration).await;
        *self.completed.write().await = true;
        format!("Task completed after {:?}", self.duration)
    }
    
    async fn is_completed(&self) -> bool {
        *self.completed.read().await
    }
}

// Channel-based coordinator
struct ChannelCoordinator {
    sender: mpsc::UnboundedSender<String>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
}

impl ChannelCoordinator {
    fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
    
    async fn send_message(&self, msg: String) -> Result<(), Box<dyn std::error::Error>> {
        self.sender.send(msg)?;
        Ok(())
    }
    
    async fn receive_message(&self) -> Option<String> {
        let mut receiver = self.receiver.lock().unwrap();
        receiver.recv().await
    }
}

// Test runner for async functions
#[tokio::main]
async fn main() {
    println!("🚀 RustFlow Coroutine (Async/Await) Test Suite");
    println!("===============================================");
    
    // Test 1: Basic async/await functionality
    test_basic_async_await().await;
    
    // Test 2: Concurrent coroutine execution
    test_concurrent_coroutines().await;
    
    // Test 3: Async channels and communication
    test_async_channels().await;
    
    // Test 4: Timeout handling with futures
    test_timeout_handling().await;
    
    // Test 5: Async synchronization patterns
    test_async_synchronization().await;
    
    // Test 6: Complex coroutine coordination
    test_complex_coordination().await;
    
    // Test 7: Performance testing with coroutines
    test_coroutine_performance().await;
    
    println!("\n✅ All coroutine tests completed successfully!");
    println!("📊 Coroutine Test Summary:");
    println!("  - Basic async/await: ✓");
    println!("  - Concurrent execution: ✓");
    println!("  - Async channels: ✓");
    println!("  - Timeout handling: ✓");
    println!("  - Async synchronization: ✓");
    println!("  - Complex coordination: ✓");
    println!("  - Performance testing: ✓");
    println!("  - Total: 7 tests passed");
}

async fn test_basic_async_await() {
    print!("🔍 Testing basic async/await functionality... ");
    
    let counter = AsyncCounter::new();
    
    // Test sequential async operations
    let count1 = counter.increment().await;
    let count2 = counter.increment().await;
    let count3 = counter.increment().await;
    
    assert_eq!(count1, 1);
    assert_eq!(count2, 2);
    assert_eq!(count3, 3);
    assert_eq!(counter.get().await, 3);
    
    println!("✓");
}

async fn test_concurrent_coroutines() {
    print!("🔍 Testing concurrent coroutine execution... ");
    
    let counter = Arc::new(AsyncCounter::new());
    let mut tasks = Vec::new();
    
    // Spawn 10 concurrent tasks
    for _ in 0..10 {
        let counter_clone = counter.clone();
        let task = tokio::spawn(async move {
            counter_clone.increment().await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    for task in tasks {
        let result = task.await.unwrap();
        results.push(result);
    }
    
    // All tasks should have completed
    assert_eq!(results.len(), 10);
    assert_eq!(counter.get().await, 10);
    
    println!("✓");
}

async fn test_async_channels() {
    print!("🔍 Testing async channels and communication... ");
    
    let coordinator = ChannelCoordinator::new();
    
    // Test sending and receiving messages
    let messages = vec![
        "Hello".to_string(),
        "World".to_string(), 
        "From".to_string(),
        "Coroutines".to_string(),
    ];
    
    // Send messages concurrently
    let send_tasks: Vec<_> = messages.iter()
        .map(|msg| {
            let coord = &coordinator;
            let msg = msg.clone();
            async move {
                coord.send_message(msg).await.unwrap();
            }
        })
        .collect();
    
    futures::future::join_all(send_tasks).await;
    
    // Receive all messages
    let mut received = Vec::new();
    for _ in 0..messages.len() {
        if let Some(msg) = coordinator.receive_message().await {
            received.push(msg);
        }
    }
    
    assert_eq!(received.len(), messages.len());
    
    println!("✓");
}

async fn test_timeout_handling() {
    print!("🔍 Testing timeout handling with futures... ");
    
    // Test successful completion within timeout
    let quick_task = TimedTask::new(Duration::from_millis(50));
    let result = timeout(Duration::from_millis(100), quick_task.execute()).await;
    assert!(result.is_ok());
    assert!(quick_task.is_completed().await);
    
    // Test timeout expiration
    let slow_task = TimedTask::new(Duration::from_millis(200));
    let result = timeout(Duration::from_millis(100), slow_task.execute()).await;
    assert!(result.is_err()); // Should timeout
    
    println!("✓");
}

async fn test_async_synchronization() {
    print!("🔍 Testing async synchronization patterns... ");
    
    let shared_data = Arc::new(RwLock::new(Vec::<i32>::new()));
    let mut tasks = Vec::new();
    
    // Spawn writers
    for i in 0..5 {
        let data = shared_data.clone();
        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(i * 10)).await;
            let mut writer = data.write().await;
            writer.push(i);
        });
        tasks.push(task);
    }
    
    // Spawn readers
    for _ in 0..3 {
        let data = shared_data.clone();
        let task = tokio::spawn(async move {
            sleep(Duration::from_millis(30)).await;
            let reader = data.read().await;
            reader.len()
        });
        tasks.push(task);
    }
    
    // Wait for all tasks
    for task in tasks {
        task.await.unwrap();
    }
    
    let final_data = shared_data.read().await;
    assert_eq!(final_data.len(), 5);
    
    println!("✓");
}

async fn test_complex_coordination() {
    print!("🔍 Testing complex coroutine coordination... ");
    
    // Producer-consumer pattern with multiple producers and consumers
    let (tx, mut rx) = mpsc::bounded(10);
    let produced_count = Arc::new(Mutex::new(0));
    let consumed_count = Arc::new(Mutex::new(0));
    
    // Spawn producers
    let mut producers = Vec::new();
    for i in 0..3 {
        let tx = tx.clone();
        let count = produced_count.clone();
        let producer = tokio::spawn(async move {
            for j in 0..10 {
                let value = i * 100 + j;
                tx.send(value).await.unwrap();
                *count.lock().unwrap() += 1;
                sleep(Duration::from_millis(1)).await;
            }
        });
        producers.push(producer);
    }
    
    // Drop original sender so receivers know when to stop
    drop(tx);
    
    // Spawn consumers
    let mut consumers = Vec::new();
    for _ in 0..2 {
        let consumed = consumed_count.clone();
        let mut rx = rx.clone();
        let consumer = tokio::spawn(async move {
            while let Some(_value) = rx.recv().await {
                *consumed.lock().unwrap() += 1;
                sleep(Duration::from_millis(1)).await;
            }
        });
        consumers.push(consumer);
    }
    
    // Wait for all producers
    for producer in producers {
        producer.await.unwrap();
    }
    
    // Close receiver channel
    rx.close();
    
    // Wait for all consumers
    for consumer in consumers {
        consumer.await.unwrap();
    }
    
    let total_produced = *produced_count.lock().unwrap();
    let total_consumed = *consumed_count.lock().unwrap();
    
    assert_eq!(total_produced, 30); // 3 producers * 10 items each
    assert_eq!(total_consumed, 30); // All items should be consumed
    
    println!("✓");
}

async fn test_coroutine_performance() {
    print!("⚡ Testing coroutine performance... ");
    
    let start = Instant::now();
    
    // Create many lightweight coroutines
    let mut tasks = Vec::new();
    for i in 0..1000 {
        let task = tokio::spawn(async move {
            // Simulate some async work
            sleep(Duration::from_nanos(100)).await;
            i * 2
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    for task in tasks {
        let result = task.await.unwrap();
        results.push(result);
    }
    
    let elapsed = start.elapsed();
    
    assert_eq!(results.len(), 1000);
    assert!(elapsed < Duration::from_millis(500), "Performance test too slow: {:?}", elapsed);
    
    println!("✓ ({:?} for 1000 coroutines)", elapsed);
}