// Working coroutine simulation test for RustFlow
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::mpsc::channel;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    println!("🚀 RustFlow Working Coroutine Test Suite");
    println!("=========================================");
    
    test_basic_coordination();
    test_channel_coordination();
    test_concurrent_tasks();
    test_producer_consumer();
    test_performance();
    
    println!("\n✅ All coroutine-style tests completed successfully!");
    println!("📊 Coroutine Test Summary:");
    println!("  - Basic coordination: ✓");
    println!("  - Channel coordination: ✓");
    println!("  - Concurrent tasks: ✓");
    println!("  - Producer-consumer: ✓");
    println!("  - Performance testing: ✓");
    println!("  - Total: 5 tests passed");
}

fn test_basic_coordination() {
    print!("🔍 Testing basic coordination... ");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(i * 2));
            counter.fetch_add((i + 1) as usize, Ordering::SeqCst);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 15); // 1+2+3+4+5
    println!("✓");
}

fn test_channel_coordination() {
    print!("🔍 Testing channel coordination... ");
    
    let (sender, receiver) = channel();
    
    let producer = thread::spawn(move || {
        for i in 0..10 {
            sender.send(i * 2).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    let consumer = thread::spawn(move || {
        let mut received = Vec::new();
        while let Ok(value) = receiver.recv() {
            received.push(value);
            if received.len() >= 10 {
                break;
            }
        }
        received
    });
    
    producer.join().unwrap();
    let received = consumer.join().unwrap();
    
    assert_eq!(received.len(), 10);
    assert_eq!(received, vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18]);
    println!("✓");
}

fn test_concurrent_tasks() {
    print!("🔍 Testing concurrent tasks... ");
    
    let shared_data = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    
    for i in 0..8 {
        let data = shared_data.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(i * 2));
            data.lock().unwrap().push(format!("task_{}", i));
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_data = shared_data.lock().unwrap();
    assert_eq!(final_data.len(), 8);
    
    for i in 0..8 {
        assert!(final_data.contains(&format!("task_{}", i)));
    }
    
    println!("✓");
}

fn test_producer_consumer() {
    print!("🔍 Testing producer-consumer pattern... ");
    
    let (sender, receiver) = channel();
    let produced = Arc::new(AtomicUsize::new(0));
    let consumed = Arc::new(AtomicUsize::new(0));
    let consumed_clone = consumed.clone();
    
    let mut producers = Vec::new();
    for i in 0..3 {
        let sender = sender.clone();
        let produced = produced.clone();
        let producer = thread::spawn(move || {
            for j in 0..10 {
                sender.send(i * 100 + j).unwrap();
                produced.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        });
        producers.push(producer);
    }
    
    drop(sender);
    
    let consumer = thread::spawn(move || {
        while let Ok(_value) = receiver.recv() {
            consumed_clone.fetch_add(1, Ordering::SeqCst);
        }
    });
    
    for producer in producers {
        producer.join().unwrap();
    }
    
    consumer.join().unwrap();
    
    assert_eq!(produced.load(Ordering::SeqCst), 30);
    assert_eq!(consumed.load(Ordering::SeqCst), 30);
    
    println!("✓");
}

fn test_performance() {
    print!("⚡ Testing performance with many tasks... ");
    
    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    for i in 0..100 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_nanos(i * 1000));
            counter.fetch_add(i as usize, Ordering::SeqCst);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    let final_count = counter.load(Ordering::SeqCst);
    
    assert_eq!(final_count, (0..100).sum::<usize>());
    assert!(elapsed < Duration::from_millis(1000));
    
    println!("✓ ({:?} for 100 tasks)", elapsed);
}