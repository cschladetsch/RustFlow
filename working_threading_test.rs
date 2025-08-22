// Working threading tests for RustFlow
use std::sync::{Arc, Barrier, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    println!("🧵 RustFlow Working Threading Test Suite");
    println!("========================================");
    
    test_basic_threading();
    test_mutex_synchronization();
    test_atomic_operations();
    test_std_channels();
    test_thread_barriers();
    test_producer_consumer();
    test_performance();
    
    println!("\n✅ All threading tests completed successfully!");
    println!("📊 Threading Test Summary:");
    println!("  - Basic threading: ✓");
    println!("  - Mutex synchronization: ✓");
    println!("  - Atomic operations: ✓");
    println!("  - Standard channels: ✓");
    println!("  - Thread barriers: ✓");
    println!("  - Producer-consumer: ✓");
    println!("  - Performance testing: ✓");
    println!("  - Total: 7 tests passed");
}

fn test_basic_threading() {
    print!("🔍 Testing basic thread spawning and joining... ");
    
    let mut handles = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));
    
    for i in 0..5 {
        let results = results.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(i * 2));
            let value = i * i;
            results.lock().unwrap().push(value);
            value
        });
        handles.push(handle);
    }
    
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
    
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 1000);
    
    println!("✓");
}

fn test_std_channels() {
    print!("🔍 Testing standard library channels... ");
    
    let (sender, receiver) = mpsc::channel();
    let mut handles = Vec::new();
    
    for i in 0..3 {
        let sender = sender.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                sender.send(i * 100 + j).unwrap();
            }
        });
        handles.push(handle);
    }
    
    drop(sender);
    
    let consumer_handle = thread::spawn(move || {
        let mut messages = Vec::new();
        while let Ok(msg) = receiver.recv() {
            messages.push(msg);
        }
        messages
    });
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let messages = consumer_handle.join().unwrap();
    assert_eq!(messages.len(), 30);
    
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
            thread::sleep(Duration::from_millis(i * 5));
            
            let before = Instant::now();
            barrier.wait();
            let after = Instant::now();
            
            results.lock().unwrap().push((i, after.duration_since(before)));
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let results = results.lock().unwrap();
    assert_eq!(results.len(), 5);
    
    // All threads should have waited at the barrier
    let max_wait = results.iter().map(|(_, duration)| *duration).max().unwrap();
    let min_wait = results.iter().map(|(_, duration)| *duration).min().unwrap();
    
    assert!(max_wait - min_wait < Duration::from_millis(50));
    
    println!("✓");
}

fn test_producer_consumer() {
    print!("🔍 Testing producer-consumer with threading... ");
    
    let (sender, receiver) = mpsc::channel();
    let produced = Arc::new(AtomicUsize::new(0));
    let consumed = Arc::new(AtomicUsize::new(0));
    
    let mut producers = Vec::new();
    for i in 0..3 {
        let sender = sender.clone();
        let produced = produced.clone();
        let producer = thread::spawn(move || {
            for j in 0..15 {
                sender.send(i * 1000 + j).unwrap();
                produced.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        });
        producers.push(producer);
    }
    
    drop(sender);
    
    let consumed_clone = consumed.clone();
    let consumer = thread::spawn(move || {
        while let Ok(_value) = receiver.recv() {
            consumed_clone.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    for producer in producers {
        producer.join().unwrap();
    }
    
    consumer.join().unwrap();
    
    assert_eq!(produced.load(Ordering::SeqCst), 45);
    assert_eq!(consumed.load(Ordering::SeqCst), 45);
    
    println!("✓");
}

fn test_performance() {
    print!("⚡ Testing threading performance... ");
    
    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    for _ in 0..50 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..200 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    
    assert_eq!(counter.load(Ordering::SeqCst), 10000);
    assert!(elapsed < Duration::from_millis(1000));
    
    println!("✓ ({:?} for 50 threads)", elapsed);
}