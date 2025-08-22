// Simple mixed coordination tests for RustFlow
// Tests interaction between Future-like patterns and standard threading

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Simple future that can be polled from threads
struct ThreadedFuture {
    duration: Duration,
    start: Option<Instant>,
    completed: Arc<AtomicBool>,
}

impl ThreadedFuture {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            start: None,
            completed: Arc::new(AtomicBool::new(false)),
        }
    }
    
    fn is_completed(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }
}

impl Future for ThreadedFuture {
    type Output = String;
    
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.start.is_none() {
            self.start = Some(Instant::now());
        }
        
        let elapsed = self.start.unwrap().elapsed();
        if elapsed >= self.duration {
            self.completed.store(true, Ordering::SeqCst);
            Poll::Ready(format!("Completed after {:?}", elapsed))
        } else {
            Poll::Pending
        }
    }
}

// Bridge between future-like and thread patterns
struct HybridCoordinator {
    futures: Arc<Mutex<Vec<ThreadedFuture>>>,
    results: Arc<Mutex<Vec<String>>>,
    thread_counter: Arc<AtomicUsize>,
}

impl HybridCoordinator {
    fn new() -> Self {
        Self {
            futures: Arc::new(Mutex::new(Vec::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            thread_counter: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    fn add_future(&self, future: ThreadedFuture) {
        self.futures.lock().unwrap().push(future);
    }
    
    fn run_mixed_workload(&self) {
        let mut thread_handles = Vec::new();
        
        // Start threads to poll futures
        for i in 0..3 {
            let futures = self.futures.clone();
            let results = self.results.clone();
            let counter = self.thread_counter.clone();
            
            let handle = thread::spawn(move || {
                // Create dummy waker for polling
                let waker = Arc::new(DummyWaker).into();
                let mut context = Context::from_waker(&waker);
                
                let mut local_results = Vec::new();
                let max_polls = 100;
                let mut polls = 0;
                
                while polls < max_polls {
                    let mut futures_guard = futures.lock().unwrap();
                    let mut completed_indices = Vec::new();
                    
                    for (idx, future) in futures_guard.iter_mut().enumerate() {
                        let future_pin = Pin::new(future);
                        match future_pin.poll(&mut context) {
                            Poll::Ready(result) => {
                                local_results.push(format!("Thread_{}: {}", i, result));
                                completed_indices.push(idx);
                            },
                            Poll::Pending => {},
                        }
                    }
                    
                    // Remove completed futures (in reverse order to maintain indices)
                    for &idx in completed_indices.iter().rev() {
                        futures_guard.remove(idx);
                    }
                    
                    let remaining = futures_guard.len();
                    drop(futures_guard);
                    
                    if remaining == 0 {
                        break;
                    }
                    
                    thread::sleep(Duration::from_millis(5));
                    polls += 1;
                }
                
                // Store results
                results.lock().unwrap().extend(local_results);
                counter.fetch_add(1, Ordering::SeqCst);
            });
            
            thread_handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in thread_handles {
            handle.join().unwrap();
        }
    }
    
    fn get_results(&self) -> Vec<String> {
        self.results.lock().unwrap().clone()
    }
    
    fn completed_threads(&self) -> usize {
        self.thread_counter.load(Ordering::SeqCst)
    }
}

fn main() {
    println!("🔄 RustFlow Simple Mixed Coordination Tests");
    println!("============================================");
    
    // Test 1: Basic future-thread interaction
    test_basic_mixed_coordination();
    
    // Test 2: Threaded future polling
    test_threaded_future_polling();
    
    // Test 3: Mixed synchronization patterns
    test_mixed_synchronization();
    
    // Test 4: Hybrid work distribution
    test_hybrid_work_distribution();
    
    // Test 5: Cross-pattern communication
    test_cross_pattern_communication();
    
    // Test 6: Performance comparison
    test_performance_comparison();
    
    // Test 7: Complex coordination
    test_complex_mixed_patterns();
    
    println!("\n✅ All mixed coordination tests completed successfully!");
    println!("📊 Mixed Coordination Test Summary:");
    println!("  - Basic mixed coordination: ✓");
    println!("  - Threaded future polling: ✓");
    println!("  - Mixed synchronization: ✓");
    println!("  - Hybrid work distribution: ✓");
    println!("  - Cross-pattern communication: ✓");
    println!("  - Performance comparison: ✓");
    println!("  - Complex mixed patterns: ✓");
    println!("  - Total: 7 tests passed");
}

fn test_basic_mixed_coordination() {
    print!("🔍 Testing basic mixed coordination... ");
    
    let shared_data = Arc::new(Mutex::new(0));
    
    // Create a future
    let future = ThreadedFuture::new(Duration::from_millis(20));
    let future_completed = future.completed.clone();
    
    // Thread that monitors the future
    let monitor_data = shared_data.clone();
    let monitor_handle = thread::spawn(move || {
        while !future_completed.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(5));
        }
        *monitor_data.lock().unwrap() = 42;
    });
    
    // Main thread polls the future
    let waker = Arc::new(DummyWaker).into();
    let mut context = Context::from_waker(&waker);
    let mut future = future;
    
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(100) {
        let future_pin = Pin::new(&mut future);
        match future_pin.poll(&mut context) {
            Poll::Ready(_result) => break,
            Poll::Pending => thread::sleep(Duration::from_millis(1)),
        }
    }
    
    monitor_handle.join().unwrap();
    
    assert!(future.is_completed());
    assert_eq!(*shared_data.lock().unwrap(), 42);
    
    println!("✓");
}

fn test_threaded_future_polling() {
    print!("🔍 Testing threaded future polling... ");
    
    let coordinator = HybridCoordinator::new();
    
    // Add multiple futures with different durations
    let durations = [10, 20, 15, 25, 5];
    for &duration_ms in &durations {
        coordinator.add_future(ThreadedFuture::new(Duration::from_millis(duration_ms)));
    }
    
    // Run the mixed workload
    coordinator.run_mixed_workload();
    
    let results = coordinator.get_results();
    
    // Should have completed all futures
    assert_eq!(results.len(), durations.len());
    assert_eq!(coordinator.completed_threads(), 3);
    
    // All results should contain "Completed after"
    for result in &results {
        assert!(result.contains("Completed after"));
        assert!(result.contains("Thread_"));
    }
    
    println!("✓");
}

fn test_mixed_synchronization() {
    print!("🔍 Testing mixed synchronization patterns... ");
    
    let shared_counter = Arc::new(AtomicUsize::new(0));
    let future_results = Arc::new(Mutex::new(Vec::new()));
    
    let mut thread_handles = Vec::new();
    
    // Threads that increment counter and create futures
    for i in 0..4 {
        let counter = shared_counter.clone();
        let results = future_results.clone();
        
        let handle = thread::spawn(move || {
            // Thread work
            for _ in 0..25 {
                counter.fetch_add(1, Ordering::SeqCst);
            }
            
            // Create and poll a future
            let mut future = ThreadedFuture::new(Duration::from_millis(i * 5));
            let waker = Arc::new(DummyWaker).into();
            let mut context = Context::from_waker(&waker);
            
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                let future_pin = Pin::new(&mut future);
                match future_pin.poll(&mut context) {
                    Poll::Ready(result) => {
                        results.lock().unwrap().push(format!("Thread_{}: {}", i, result));
                        break;
                    },
                    Poll::Pending => thread::sleep(Duration::from_millis(1)),
                }
            }
        });
        
        thread_handles.push(handle);
    }
    
    // Wait for all threads
    for handle in thread_handles {
        handle.join().unwrap();
    }
    
    assert_eq!(shared_counter.load(Ordering::SeqCst), 100);
    assert_eq!(future_results.lock().unwrap().len(), 4);
    
    println!("✓");
}

fn test_hybrid_work_distribution() {
    print!("🔍 Testing hybrid work distribution... ");
    
    let (sender, receiver) = mpsc::channel();
    let future_counter = Arc::new(AtomicUsize::new(0));
    let thread_counter = Arc::new(AtomicUsize::new(0));
    
    // Producer thread that creates work items
    let producer = thread::spawn(move || {
        for i in 0..10 {
            sender.send((i, i % 3 == 0)).unwrap(); // (work_item, is_future_work)
            thread::sleep(Duration::from_millis(2));
        }
    });
    
    // Consumer thread that handles both regular work and future work
    // (cannot clone receiver, so using single consumer)
    let future_counter_clone = future_counter.clone();
    let thread_counter_clone = thread_counter.clone();
    
    let consumer = thread::spawn(move || {
        while let Ok((work_item, is_future_work)) = receiver.recv() {
                if is_future_work {
                    // Handle as future-like work
                    let mut future = ThreadedFuture::new(Duration::from_millis(5));
                    let waker = Arc::new(DummyWaker).into();
                    let mut context = Context::from_waker(&waker);
                    
                    let start = Instant::now();
                    while start.elapsed() < Duration::from_millis(50) {
                        let future_pin = Pin::new(&mut future);
                        match future_pin.poll(&mut context) {
                            Poll::Ready(_) => {
                                future_counter_clone.fetch_add(work_item, Ordering::SeqCst);
                                break;
                            },
                            Poll::Pending => thread::sleep(Duration::from_millis(1)),
                        }
                    }
                } else {
                    // Handle as regular thread work
                    thread::sleep(Duration::from_millis(5));
                    thread_counter_clone.fetch_add(work_item, Ordering::SeqCst);
                }
            }
        });
    
    producer.join().unwrap();
    
    consumer.join().unwrap();
    
    // Verify work was distributed correctly
    let future_work_sum = future_counter.load(Ordering::SeqCst);
    let thread_work_sum = thread_counter.load(Ordering::SeqCst);
    
    // Future work: 0, 3, 6, 9 (items where i % 3 == 0)
    // Thread work: 1, 2, 4, 5, 7, 8
    assert_eq!(future_work_sum, 0 + 3 + 6 + 9);
    assert_eq!(thread_work_sum, 1 + 2 + 4 + 5 + 7 + 8);
    
    println!("✓");
}

fn test_cross_pattern_communication() {
    print!("🔍 Testing cross-pattern communication... ");
    
    let (thread_to_future, future_receiver) = mpsc::channel();
    let (future_to_thread, thread_receiver) = mpsc::channel();
    
    let future_results = Arc::new(Mutex::new(Vec::new()));
    let thread_results = Arc::new(Mutex::new(Vec::new()));
    
    // Thread that sends to future-polling thread
    let sender_thread = {
        let sender = thread_to_future.clone();
        thread::spawn(move || {
            for i in 0..5 {
                sender.send(format!("thread_msg_{}", i)).unwrap();
                thread::sleep(Duration::from_millis(10));
            }
        })
    };
    
    // Thread that polls futures AND receives messages
    let future_thread = {
        let results = future_results.clone();
        let sender = future_to_thread.clone();
        thread::spawn(move || {
            let mut futures = Vec::new();
            
            // Create some futures
            for i in 0..3 {
                futures.push(ThreadedFuture::new(Duration::from_millis(i * 10 + 20)));
            }
            
            let waker = Arc::new(DummyWaker).into();
            let mut context = Context::from_waker(&waker);
            let start = Instant::now();
            
            while start.elapsed() < Duration::from_millis(200) {
                // Poll futures
                let mut completed_indices = Vec::new();
                for (idx, future) in futures.iter_mut().enumerate() {
                    let future_pin = Pin::new(future);
                    if let Poll::Ready(result) = future_pin.poll(&mut context) {
                        results.lock().unwrap().push(result);
                        completed_indices.push(idx);
                    }
                }
                
                // Remove completed futures
                for &idx in completed_indices.iter().rev() {
                    futures.remove(idx);
                }
                
                // Check for messages from thread
                if let Ok(msg) = future_receiver.try_recv() {
                    sender.send(format!("processed_{}", msg)).unwrap();
                }
                
                // Break if all futures are done
                if futures.is_empty() {
                    break;
                }
                
                thread::sleep(Duration::from_millis(5));
            }
        })
    };
    
    // Thread that receives from future thread
    let receiver_thread = {
        let results = thread_results.clone();
        thread::spawn(move || {
            while let Ok(msg) = thread_receiver.recv() {
                results.lock().unwrap().push(msg);
            }
        })
    };
    
    sender_thread.join().unwrap();
    future_thread.join().unwrap();
    
    // Close channels
    drop(future_to_thread);
    receiver_thread.join().unwrap();
    
    let future_res = future_results.lock().unwrap();
    let thread_res = thread_results.lock().unwrap();
    
    assert_eq!(future_res.len(), 3); // 3 futures completed
    assert_eq!(thread_res.len(), 5); // 5 messages processed
    
    println!("✓");
}

fn test_performance_comparison() {
    print!("⚡ Testing performance comparison... ");
    
    let iterations = 1000;
    
    // Pure threading approach
    let thread_start = Instant::now();
    let thread_counter = Arc::new(AtomicUsize::new(0));
    
    let mut thread_handles = Vec::new();
    for _ in 0..10 {
        let counter = thread_counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..iterations / 10 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        thread_handles.push(handle);
    }
    
    for handle in thread_handles {
        handle.join().unwrap();
    }
    let thread_duration = thread_start.elapsed();
    
    // Mixed future+thread approach
    let mixed_start = Instant::now();
    let mixed_counter = Arc::new(AtomicUsize::new(0));
    
    let mut mixed_handles = Vec::new();
    for i in 0..10 {
        let counter = mixed_counter.clone();
        let handle = thread::spawn(move || {
            if i % 2 == 0 {
                // Future-like work
                let mut future = ThreadedFuture::new(Duration::from_nanos(100));
                let waker = Arc::new(DummyWaker).into();
                let mut context = Context::from_waker(&waker);
                
                // Poll the future a few times
                for _ in 0..iterations / 20 {
                    let future_pin = Pin::new(&mut future);
                    let _ = future_pin.poll(&mut context);
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                // Regular thread work
                for _ in 0..iterations / 10 {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        mixed_handles.push(handle);
    }
    
    for handle in mixed_handles {
        handle.join().unwrap();
    }
    let mixed_duration = mixed_start.elapsed();
    
    // Verify both approaches did work
    assert_eq!(thread_counter.load(Ordering::SeqCst), iterations);
    // Mixed approach does different amounts of work, just verify it's reasonable
    assert!(mixed_counter.load(Ordering::SeqCst) > 0);
    
    println!("✓ (threads: {:?}, mixed: {:?})", thread_duration, mixed_duration);
}

fn test_complex_mixed_patterns() {
    print!("🔍 Testing complex mixed coordination patterns... ");
    
    // Multi-stage pipeline: thread -> future -> thread
    let stage1_results = Arc::new(Mutex::new(Vec::new()));
    let stage2_results = Arc::new(Mutex::new(Vec::new()));
    let stage3_results = Arc::new(Mutex::new(Vec::new()));
    
    // Stage 1: Thread generates data
    let stage1_data = Arc::new(Mutex::new(Vec::new()));
    let stage1_thread = {
        let data = stage1_data.clone();
        let results = stage1_results.clone();
        thread::spawn(move || {
            for i in 0..6 {
                thread::sleep(Duration::from_millis(5));
                let value = i * 10;
                data.lock().unwrap().push(value);
                results.lock().unwrap().push(format!("stage1_{}", value));
            }
        })
    };
    
    // Stage 2: Future-like processing
    let stage2_thread = {
        let input_data = stage1_data.clone();
        let results = stage2_results.clone();
        thread::spawn(move || {
            let waker = Arc::new(DummyWaker).into();
            let mut context = Context::from_waker(&waker);
            
            while results.lock().unwrap().len() < 6 {
                let data_to_process = {
                    let mut data = input_data.lock().unwrap();
                    if data.is_empty() {
                        None
                    } else {
                        Some(data.remove(0))
                    }
                };
                
                if let Some(value) = data_to_process {
                    // Process with future-like pattern
                    let mut future = ThreadedFuture::new(Duration::from_millis(5));
                    let start = Instant::now();
                    
                    while start.elapsed() < Duration::from_millis(50) {
                        let future_pin = Pin::new(&mut future);
                        match future_pin.poll(&mut context) {
                            Poll::Ready(_) => {
                                results.lock().unwrap().push(format!("stage2_{}", value * 2));
                                break;
                            },
                            Poll::Pending => thread::sleep(Duration::from_millis(1)),
                        }
                    }
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        })
    };
    
    // Wait for stages to complete
    stage1_thread.join().unwrap();
    stage2_thread.join().unwrap();
    
    // Stage 3: Final thread processing
    let stage3_thread = {
        let stage2_res = stage2_results.clone();
        let results = stage3_results.clone();
        thread::spawn(move || {
            let stage2_data = stage2_res.lock().unwrap().clone();
            for item in stage2_data {
                thread::sleep(Duration::from_millis(2));
                results.lock().unwrap().push(format!("stage3_{}", item));
            }
        })
    };
    
    stage3_thread.join().unwrap();
    
    // Verify pipeline worked
    assert_eq!(stage1_results.lock().unwrap().len(), 6);
    assert_eq!(stage2_results.lock().unwrap().len(), 6);
    assert_eq!(stage3_results.lock().unwrap().len(), 6);
    
    // Verify data transformation
    let final_results = stage3_results.lock().unwrap();
    assert!(final_results.iter().all(|s| s.starts_with("stage3_stage2_")));
    
    println!("✓");
}

// Dummy waker for testing
struct DummyWaker;

impl std::task::Wake for DummyWaker {
    fn wake(self: Arc<Self>) {
        // No-op for testing
    }
}