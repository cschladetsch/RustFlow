// Comprehensive threading tests for RustFlow
// Tests std::thread, crossbeam channels, parking_lot, and thread coordination

use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crossbeam::channel::{bounded, unbounded, select};
use parking_lot::{RwLock, Mutex as ParkingMutex};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

// Thread pool implementation for testing
struct SimpleThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: crossbeam::channel::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl SimpleThreadPool {
    fn new(size: usize) -> Self {
        let (sender, receiver) = unbounded();
        let receiver = Arc::new(receiver);
        
        let mut workers = Vec::with_capacity(size);
        
        for _id in 0..size {
            let receiver = receiver.clone();
            let handle = thread::spawn(move || {
                while let Ok(job) = receiver.recv() {
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
    parking_count: ParkingMutex<usize>,
    rw_data: RwLock<Vec<i32>>,
}

impl SharedCounter {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            parking_count: ParkingMutex::new(0),
            rw_data: RwLock::new(Vec::new()),
        }
    }
    
    fn increment_atomic(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst)
    }
    
    fn increment_parking(&self) -> usize {
        let mut count = self.parking_count.lock();
        *count += 1;
        *count
    }
    
    fn add_data(&self, value: i32) {
        let mut data = self.rw_data.write();
        data.push(value);
    }
    
    fn read_data(&self) -> Vec<i32> {
        let data = self.rw_data.read();
        data.clone()
    }
}

fn main() {
    println!("🧵 RustFlow Threading Test Suite");
    println!("=================================");
    
    // Test 1: Basic thread spawning and joining
    test_basic_threading();
    
    // Test 2: Thread synchronization with Mutex
    test_mutex_synchronization();
    
    // Test 3: Atomic operations
    test_atomic_operations();
    
    // Test 4: Crossbeam channels
    test_crossbeam_channels();
    
    // Test 5: Parking lot synchronization
    test_parking_lot_sync();
    
    // Test 6: Thread barriers and coordination
    test_thread_barriers();
    
    // Test 7: Thread pool implementation
    test_thread_pool();
    
    // Test 8: Producer-consumer with threading
    test_producer_consumer();
    
    // Test 9: Performance comparison
    test_threading_performance();
    
    println!("\n✅ All threading tests completed successfully!");
    println!("📊 Threading Test Summary:");
    println!("  - Basic threading: ✓");
    println!("  - Mutex synchronization: ✓");
    println!("  - Atomic operations: ✓");
    println!("  - Crossbeam channels: ✓");
    println!("  - Parking lot sync: ✓");
    println!("  - Thread barriers: ✓");
    println!("  - Thread pool: ✓");
    println!("  - Producer-consumer: ✓");
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

fn test_crossbeam_channels() {
    print!("🔍 Testing crossbeam channels... ");
    
    // Test unbounded channel
    let (sender, receiver) = unbounded();
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
    
    // Test bounded channel
    let (sender, receiver) = bounded(5);
    
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

fn test_parking_lot_sync() {
    print!("🔍 Testing parking lot synchronization... ");
    
    let shared_data = Arc::new(SharedCounter::new());
    let mut handles = Vec::new();
    
    // Test parking_lot Mutex
    for _ in 0..5 {
        let data = shared_data.clone();
        let handle = thread::spawn(move || {
            for _ in 0..20 {
                data.increment_parking();
            }
        });
        handles.push(handle);
    }
    
    // Test RwLock with concurrent readers and writers
    for i in 0..3 {
        let data = shared_data.clone();
        let handle = thread::spawn(move || {
            // Writer
            for j in 0..5 {
                data.add_data(i * 10 + j);
            }
        });
        handles.push(handle);
    }
    
    for _ in 0..2 {
        let data = shared_data.clone();
        let handle = thread::spawn(move || {
            // Reader
            thread::sleep(Duration::from_millis(10));
            let _data = data.read_data();
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(*shared_data.parking_count.lock(), 100);
    assert_eq!(shared_data.read_data().len(), 15);
    
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
    assert!(max_wait - min_wait < Duration::from_millis(50));
    
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
    
    let (sender, receiver) = bounded(10);
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
    
    // Spawn consumers
    let mut consumers = Vec::new();
    for _ in 0..2 {
        let receiver = receiver.clone();
        let consumed = consumed.clone();
        let consumer = thread::spawn(move || {
            while let Ok(_value) = receiver.recv() {
                consumed.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        });
        consumers.push(consumer);
    }
    
    // Wait for producers
    for producer in producers {
        producer.join().unwrap();
    }
    
    // Wait for consumers
    for consumer in consumers {
        consumer.join().unwrap();
    }
    
    assert_eq!(produced.load(Ordering::SeqCst), 45);
    assert_eq!(consumed.load(Ordering::SeqCst), 45);
    
    println!("✓");
}

fn test_threading_performance() {
    print!("⚡ Testing threading performance... ");
    
    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    // Spawn 100 threads doing lightweight work
    for _ in 0..100 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
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
    assert!(elapsed < Duration::from_millis(1000), "Performance test too slow: {:?}", elapsed);
    
    println!("✓ ({:?} for 100 threads)", elapsed);
}