// Simple coroutine simulation test for RustFlow
// Tests basic async patterns and Future trait implementation without external deps

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// Simple Future implementation for testing
struct SimpleTimer {
    duration: Duration,
    start: Option<Instant>,
    completed: Arc<AtomicBool>,
}

impl SimpleTimer {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            start: None,
            completed: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Future for SimpleTimer {
    type Output = ();
    
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.start.is_none() {
            self.start = Some(Instant::now());
        }
        
        let elapsed = self.start.unwrap().elapsed();
        if elapsed >= self.duration {
            self.completed.store(true, Ordering::SeqCst);
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

// Channel-based async simulation
struct AsyncChannel<T> {
    sender: Sender<T>,
    receiver: Arc<Mutex<Receiver<T>>>,
}

impl<T> AsyncChannel<T> {
    fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
    
    fn send(&self, value: T) -> Result<(), T> {
        match self.sender.send(value) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.0),
        }
    }
    
    fn try_recv(&self) -> Result<T, std::sync::mpsc::TryRecvError> {
        self.receiver.lock().unwrap().try_recv()
    }
}

// Simple async-like executor
struct SimpleExecutor {
    tasks: Arc<Mutex<Vec<Box<dyn FnOnce() + Send + 'static>>>>,
    completed: Arc<AtomicUsize>,
}

impl SimpleExecutor {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            completed: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    fn spawn<F>(&self, task: F) where F: FnOnce() + Send + 'static {
        self.tasks.lock().unwrap().push(Box::new(task));
    }
    
    fn run(&self) {
        let tasks = {
            let mut guard = self.tasks.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        
        let mut handles = Vec::new();
        for task in tasks {
            let completed = self.completed.clone();
            let handle = thread::spawn(move || {
                task();
                completed.fetch_add(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
    }
    
    fn completed_count(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }
}

fn main() {
    println!("🚀 RustFlow Simple Coroutine Test Suite");
    println!("========================================");
    
    // Test 1: Basic Future implementation
    test_simple_future();
    
    // Test 2: Channel-based coordination
    test_channel_coordination();
    
    // Test 3: Simple executor pattern
    test_simple_executor();
    
    // Test 4: Concurrent task coordination
    test_concurrent_coordination();
    
    // Test 5: Timer-based futures
    test_timer_futures();
    
    // Test 6: Producer-consumer pattern
    test_producer_consumer();
    
    // Test 7: Performance test with many tasks
    test_performance();
    
    println!("\n✅ All simple coroutine tests completed successfully!");
    println!("📊 Simple Coroutine Test Summary:");
    println!("  - Basic Future implementation: ✓");
    println!("  - Channel coordination: ✓");
    println!("  - Simple executor: ✓");
    println!("  - Concurrent coordination: ✓");
    println!("  - Timer futures: ✓");
    println!("  - Producer-consumer: ✓");
    println!("  - Performance testing: ✓");
    println!("  - Total: 7 tests passed");
}

fn test_simple_future() {
    print!("🔍 Testing basic Future implementation... ");
    
    // Simulate polling a future
    let mut timer = SimpleTimer::new(Duration::from_millis(10));
    let timer_pin = Pin::new(&mut timer);
    
    // Create a dummy context (we won't actually use the waker)
    let waker = Arc::new(DummyWaker).into();
    let mut context = Context::from_waker(&waker);
    
    // First poll should be Pending
    match timer_pin.poll(&mut context) {
        Poll::Pending => {},
        Poll::Ready(()) => panic!("Timer completed too early"),
    }
    
    // Wait and poll again - test a timer that should complete quickly
    let mut timer2 = SimpleTimer::new(Duration::from_millis(1));
    thread::sleep(Duration::from_millis(5)); // Wait longer than timer duration
    let timer_pin2 = Pin::new(&mut timer2);
    
    match timer_pin2.poll(&mut context) {
        Poll::Ready(()) => {},
        Poll::Pending => panic!("Timer should have completed"),
    }
    
    println!("✓");
}

fn test_channel_coordination() {
    print!("🔍 Testing channel-based coordination... ");
    
    let channel = AsyncChannel::new();
    let sender_channel = channel.sender.clone();
    let receiver = channel.receiver.clone();
    
    // Spawn producer thread
    let producer = thread::spawn(move || {
        for i in 0..10 {
            sender_channel.send(i).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    // Consumer in main thread
    let mut received = Vec::new();
    let start = Instant::now();
    
    while received.len() < 10 && start.elapsed() < Duration::from_millis(500) {
        match receiver.lock().unwrap().try_recv() {
            Ok(value) => received.push(value),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(1));
            },
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }
    }
    
    producer.join().unwrap();
    
    assert_eq!(received.len(), 10);
    assert_eq!(received, (0..10).collect::<Vec<_>>());
    
    println!("✓");
}

fn test_simple_executor() {
    print!("🔍 Testing simple executor pattern... ");
    
    let executor = SimpleExecutor::new();
    
    // Spawn multiple tasks
    for i in 0..5 {
        executor.spawn(move || {
            thread::sleep(Duration::from_millis(i * 2));
            // Task completed (counted by executor)
        });
    }
    
    // Execute all tasks
    executor.run();
    
    assert_eq!(executor.completed_count(), 5);
    
    println!("✓");
}

fn test_concurrent_coordination() {
    print!("🔍 Testing concurrent task coordination... ");
    
    let shared_counter = Arc::new(AtomicUsize::new(0));
    let barrier_count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    // Spawn multiple "async" tasks (simulated with threads)
    for i in 0..8 {
        let counter = shared_counter.clone();
        let barrier = barrier_count.clone();
        
        let handle = thread::spawn(move || {
            // Simulate async work
            thread::sleep(Duration::from_millis(i * 2));
            
            // Update shared state
            counter.fetch_add((i + 1) as usize, Ordering::SeqCst);
            
            // Signal completion
            barrier.fetch_add(1, Ordering::SeqCst);
        });
        
        handles.push(handle);
    }
    
    // Wait for all "async" tasks to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(barrier_count.load(Ordering::SeqCst), 8);
    
    // Sum should be 1+2+3+4+5+6+7+8 = 36
    let expected_sum: usize = (1..=8).sum();
    assert_eq!(shared_counter.load(Ordering::SeqCst), expected_sum);
    
    println!("✓");
}

fn test_timer_futures() {
    print!("🔍 Testing timer-based futures... ");
    
    let start_time = Instant::now();
    
    // Create multiple timers with different durations
    let mut timers = Vec::new();
    let durations = [10, 20, 15, 5, 25];
    
    for &duration_ms in &durations {
        timers.push(SimpleTimer::new(Duration::from_millis(duration_ms)));
    }
    
    // Simulate polling them until completion
    let mut completed = vec![false; timers.len()];
    let waker = Arc::new(DummyWaker).into();
    let mut context = Context::from_waker(&waker);
    
    while !completed.iter().all(|&c| c) && start_time.elapsed() < Duration::from_millis(100) {
        for (i, timer) in timers.iter_mut().enumerate() {
            if !completed[i] {
                let timer_pin = Pin::new(timer);
                match timer_pin.poll(&mut context) {
                    Poll::Ready(()) => completed[i] = true,
                    Poll::Pending => {},
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    
    assert!(completed.iter().all(|&c| c), "All timers should complete");
    
    println!("✓");
}

fn test_producer_consumer() {
    print!("🔍 Testing producer-consumer pattern... ");
    
    let (sender, receiver) = channel();
    let receiver = Arc::new(Mutex::new(receiver));
    
    let produced_count = Arc::new(AtomicUsize::new(0));
    let consumed_count = Arc::new(AtomicUsize::new(0));
    
    // Spawn producers
    let mut producers = Vec::new();
    for i in 0..3 {
        let sender = sender.clone();
        let produced = produced_count.clone();
        
        let producer = thread::spawn(move || {
            for j in 0..10 {
                let value = i * 100 + j;
                sender.send(value).unwrap();
                produced.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        });
        
        producers.push(producer);
    }
    
    drop(sender); // Close the channel when producers are done
    
    // Consumer
    let consumer_receiver = receiver.clone();
    let consumed = consumed_count.clone();
    
    let consumer = thread::spawn(move || {
        loop {
            match consumer_receiver.lock().unwrap().recv() {
                Ok(_value) => {
                    consumed.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(1));
                },
                Err(_) => break, // Channel closed
            }
        }
    });
    
    // Wait for all producers
    for producer in producers {
        producer.join().unwrap();
    }
    
    // Wait for consumer
    consumer.join().unwrap();
    
    let total_produced = produced_count.load(Ordering::SeqCst);
    let total_consumed = consumed_count.load(Ordering::SeqCst);
    
    assert_eq!(total_produced, 30);
    assert_eq!(total_consumed, 30);
    
    println!("✓");
}

fn test_performance() {
    print!("⚡ Testing performance with many tasks... ");
    
    let start = Instant::now();
    let executor = SimpleExecutor::new();
    let counter = Arc::new(AtomicUsize::new(0));
    
    // Spawn many lightweight tasks
    for i in 0..100 {
        let counter = counter.clone();
        executor.spawn(move || {
            // Simulate minimal async work
            thread::sleep(Duration::from_nanos(100));
            counter.fetch_add(i, Ordering::SeqCst);
        });
    }
    
    executor.run();
    
    let elapsed = start.elapsed();
    let final_count = counter.load(Ordering::SeqCst);
    
    assert_eq!(executor.completed_count(), 100);
    assert_eq!(final_count, (0..100).sum::<usize>());
    assert!(elapsed < Duration::from_millis(1000), "Performance test took too long: {:?}", elapsed);
    
    println!("✓ ({:?} for 100 tasks)", elapsed);
}

// Dummy waker implementation for testing
struct DummyWaker;

impl std::task::Wake for DummyWaker {
    fn wake(self: Arc<Self>) {
        // No-op for testing
    }
}