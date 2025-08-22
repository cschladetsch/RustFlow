// Simple threading tests for RustFlow
// Tests std::thread, std::sync, and basic coordination without external deps

use std::sync::{Arc, Barrier, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};

// Simple thread pool implementation for testing
struct SimpleThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl SimpleThreadPool {
    fn new(size: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send + 'static>>();
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        
        for _id in 0..size {
            let receiver = receiver.clone();
            let handle = thread::spawn(move || {
                while let Ok(job) = receiver.lock().unwrap().recv() {
                    job();
                }
            });
            workers.push(handle);
        }
        
        Self { workers, sender }
    }
    
    fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        self.sender.send(Box::new(f)).unwrap();
    }
}

impl Drop for SimpleThreadPool {
    fn drop(&mut self) {
        drop(self.sender.clone());
        
        for worker in self.workers.drain(..) {
            worker.join().unwrap();
        }
    }
}

// Shared data structure for testing
struct SharedCounter {
    count: AtomicUsize,
    mutex_count: Mutex<usize>,
    data: Mutex<Vec<i32>>,
}

impl SharedCounter {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            mutex_count: Mutex::new(0),
            data: Mutex::new(Vec::new()),
        }
    }
    
    fn increment_atomic(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst)
    }
    
    fn increment_mutex(&self) -> usize {
        let mut count = self.mutex_count.lock().unwrap();
        *count += 1;
        *count
    }
    
    fn add_data(&self, value: i32) {
        let mut data = self.data.lock().unwrap();
        data.push(value);
    }
    
    fn get_data(&self) -> Vec<i32> {
        let data = self.data.lock().unwrap();
        data.clone()
    }
}

fn main() {
    println!("🧵 RustFlow Simple Threading Test Suite");
    println!("=======================================");
    
    // Test 1: Basic thread spawning and joining
    test_basic_threading();
    
    // Test 2: Thread synchronization with Mutex
    test_mutex_synchronization();
    
    // Test 3: Atomic operations
    test_atomic_operations();
    
    // Test 4: Standard library channels
    test_std_channels();
    
    // Test 5: Thread barriers and coordination
    test_thread_barriers();
    
    // Test 6: Thread pool implementation
    test_thread_pool();
    
    // Test 7: Producer-consumer with threading
    test_producer_consumer();
    
    // Test 8: Condition variables
    test_condition_variables();
    
    // Test 9: Performance comparison
    test_threading_performance();
    
    println!("\n✅ All simple threading tests completed successfully!");
    println!("📊 Threading Test Summary:");
    println!("  - Basic threading: ✓");
    println!("  - Mutex synchronization: ✓");
    println!("  - Atomic operations: ✓");
    println!("  - Standard channels: ✓");
    println!("  - Thread barriers: ✓");
    println!("  - Thread pool: ✓");
    println!("  - Producer-consumer: ✓");
    println!("  - Condition variables: ✓");
    println!("  - Performance testing: ✓");
    println!("  - Total: 9 tests passed");
}

fn test_basic_threading() {
    print!("🔍 Testing basic thread spawning and joining... ");
    
    let mut handles = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));
    
    // Spawn 5 threads
    for i in 0..5 {
        let results = results.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10 * i));
            let value = i * i;
            results.lock().unwrap().push(value);
            value
        });
        handles.push(handle);
    }
    
    // Join all threads
    let mut thread_results = Vec::new();
    for handle in handles {
        let result = handle.join().unwrap();
        thread_results.push(result);
    }
    
    assert_eq!(thread_results.len(), 5);
    assert_eq!(results.lock().unwrap().len(), 5);
    
    println!("✓");
}

fn test_mutex_synchronization() {
    print!("🔍 Testing mutex synchronization... ");
    
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();
    
    // Spawn 10 threads that increment a counter
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let mut num = counter.lock().unwrap();
                *num += 1;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(*counter.lock().unwrap(), 1000);
    
    println!("✓");
}

fn test_atomic_operations() {
    print!("🔍 Testing atomic operations... ");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    // Spawn 10 threads using atomic operations
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 1000);
    
    println!("✓");
}

fn test_std_channels() {
    print!("🔍 Testing standard library channels... ");
    
    // Test unbounded channel (using mpsc)
    let (sender, receiver) = mpsc::channel();
    let mut handles = Vec::new();
    
    // Spawn producer threads
    for i in 0..3 {
        let sender = sender.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                sender.send(i * 100 + j).unwrap();
            }
        });
        handles.push(handle);
    }
    
    // Drop original sender
    drop(sender);
    
    // Spawn consumer thread
    let consumer_handle = thread::spawn(move || {
        let mut messages = Vec::new();
        while let Ok(msg) = receiver.recv() {
            messages.push(msg);
        }
        messages
    });
    
    // Wait for producers
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Get results from consumer
    let messages = consumer_handle.join().unwrap();
    assert_eq!(messages.len(), 30);
    
    // Test sync channel (bounded)
    let (sender, receiver) = mpsc::sync_channel(5);
    
    let producer = thread::spawn(move || {
        for i in 0..10 {
            sender.send(i).unwrap();
        }
    });
    
    let consumer = thread::spawn(move || {
        let mut sum = 0;
        while let Ok(val) = receiver.recv() {
            sum += val;
        }
        sum
    });
    
    producer.join().unwrap();
    let sum = consumer.join().unwrap();
    assert_eq!(sum, 45); // 0+1+2+...+9
    
    println!("✓");
}

fn test_thread_barriers() {
    print!("🔍 Testing thread barriers and coordination... ");
    
    let barrier = Arc::new(Barrier::new(5));
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let barrier = barrier.clone();
        let results = results.clone();
        let handle = thread::spawn(move || {
            // Do some work with different timing
            thread::sleep(Duration::from_millis(i * 10));
            
            // Record time before barrier
            let before = Instant::now();
            
            // Wait at barrier
            barrier.wait();
            
            // Record time after barrier - all threads should be close
            let after = Instant::now();
            results.lock().unwrap().push((i, after.duration_since(before)));
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let results = results.lock().unwrap();
    assert_eq!(results.len(), 5);
    
    // All threads should have waited roughly the same amount of time at the barrier
    let max_wait = results.iter().map(|(_, duration)| *duration).max().unwrap();
    let min_wait = results.iter().map(|(_, duration)| *duration).min().unwrap();
    
    // The difference shouldn't be too large (allowing for scheduling variance)
    assert!(max_wait - min_wait < Duration::from_millis(100));
    
    println!("✓");
}

fn test_thread_pool() {
    print!("🔍 Testing thread pool implementation... ");
    
    let pool = SimpleThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    
    // Submit 20 tasks to the thread pool
    for i in 0..20 {
        let counter = counter.clone();
        let completed = completed.clone();
        pool.execute(move || {
            // Simulate some work
            thread::sleep(Duration::from_millis(5));
            counter.fetch_add(i, Ordering::SeqCst);
            completed.fetch_add(1, Ordering::SeqCst);
        });
    }
    
    // Wait for all tasks to complete
    while completed.load(Ordering::SeqCst) < 20 {
        thread::sleep(Duration::from_millis(10));
    }
    
    let final_count = counter.load(Ordering::SeqCst);
    let expected = (0..20).sum::<usize>();
    assert_eq!(final_count, expected);
    assert_eq!(completed.load(Ordering::SeqCst), 20);
    
    println!("✓");
}

fn test_producer_consumer() {
    print!("🔍 Testing producer-consumer with threading... ");
    
    let (sender, receiver) = mpsc::sync_channel(10);
    let produced = Arc::new(AtomicUsize::new(0));
    let consumed = Arc::new(AtomicUsize::new(0));
    
    // Spawn producers
    let mut producers = Vec::new();
    for i in 0..3 {
        let sender = sender.clone();
        let produced = produced.clone();
        let producer = thread::spawn(move || {
            for j in 0..15 {
                let value = i * 1000 + j;
                sender.send(value).unwrap();
                produced.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        });
        producers.push(producer);
    }
    
    // Drop original sender
    drop(sender);
    
    // Spawn consumer (only one consumer since receiver cannot be cloned)
    let consumed_clone = consumed.clone();
    let consumer = thread::spawn(move || {
        while let Ok(_value) = receiver.recv() {
            consumed_clone.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    // Wait for producers
    for producer in producers {
        producer.join().unwrap();
    }
    
    // Wait for consumer
    consumer.join().unwrap();
    
    assert_eq!(produced.load(Ordering::SeqCst), 45);
    assert_eq!(consumed.load(Ordering::SeqCst), 45);
    
    println!("✓");
}

fn test_condition_variables() {
    print!("🔍 Testing condition variables... ");
    
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);
    
    // Spawn a thread that will signal the condition
    let signaler = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        let (lock, cvar) = &*pair2;
        let mut started = lock.lock().unwrap();
        *started = true;
        cvar.notify_one();
    });
    
    // Wait for the condition
    let (lock, cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }
    
    signaler.join().unwrap();
    assert!(*started);
    
    println!("✓");
}

fn test_threading_performance() {
    print!("⚡ Testing threading performance... ");
    
    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    // Spawn 50 threads doing lightweight work
    for _ in 0..50 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..200 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    
    assert_eq!(counter.load(Ordering::SeqCst), 10000);
    assert!(elapsed < Duration::from_millis(2000), "Performance test too slow: {:?}", elapsed);
    
    println!("✓ ({:?} for 50 threads)", elapsed);
}