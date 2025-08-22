// Comprehensive test suite demonstrating coroutine and threading concepts
// Using only std library to avoid dependency issues

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Barrier, mpsc, Condvar};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// Simulated coroutine state machine
#[derive(Debug, Clone, PartialEq)]
enum CoroutineState {
    Ready,
    Running,
    Suspended,
    Completed,
}

struct SimpleCoroutine {
    id: usize,
    state: CoroutineState,
    step_count: usize,
    max_steps: usize,
}

impl SimpleCoroutine {
    fn new(id: usize, max_steps: usize) -> Self {
        Self {
            id,
            state: CoroutineState::Ready,
            step_count: 0,
            max_steps,
        }
    }
    
    fn step(&mut self) -> bool {
        match self.state {
            CoroutineState::Ready => {
                self.state = CoroutineState::Running;
                true
            }
            CoroutineState::Running => {
                self.step_count += 1;
                
                // Simulate yielding occasionally
                if self.step_count % 3 == 0 {
                    self.state = CoroutineState::Suspended;
                } else if self.step_count >= self.max_steps {
                    self.state = CoroutineState::Completed;
                    return false;
                }
                true
            }
            CoroutineState::Suspended => {
                self.state = CoroutineState::Running;
                true
            }
            CoroutineState::Completed => false,
        }
    }
    
    fn is_completed(&self) -> bool {
        self.state == CoroutineState::Completed
    }
}

// Thread pool implementation
struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: Option<mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        assert!(size > 0);
        
        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send + 'static>>();
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        
        for id in 0..size {
            let receiver = Arc::clone(&receiver);
            
            let worker = thread::spawn(move || {
                loop {
                    let job = receiver.lock().unwrap().recv();
                    
                    match job {
                        Ok(job) => {
                            // println!("Worker {} got a job; executing.", id);
                            job();
                        }
                        Err(_) => {
                            // println!("Worker {} disconnected; shutting down.", id);
                            break;
                        }
                    }
                }
            });
            
            workers.push(worker);
        }
        
        Self {
            workers,
            sender: Some(sender),
        }
    }
    
    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        
        while let Some(worker) = self.workers.pop() {
            worker.join().unwrap();
        }
    }
}

// Producer-Consumer pattern
struct ProducerConsumer {
    buffer: Arc<Mutex<Vec<i32>>>,
    capacity: usize,
    not_full: Arc<Condvar>,
    not_empty: Arc<Condvar>,
}

impl ProducerConsumer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            capacity,
            not_full: Arc::new(Condvar::new()),
            not_empty: Arc::new(Condvar::new()),
        }
    }
    
    fn produce(&self, item: i32) {
        let mut buffer = self.buffer.lock().unwrap();
        
        while buffer.len() >= self.capacity {
            buffer = self.not_full.wait(buffer).unwrap();
        }
        
        buffer.push(item);
        self.not_empty.notify_one();
    }
    
    fn consume(&self) -> Option<i32> {
        let mut buffer = self.buffer.lock().unwrap();
        
        while buffer.is_empty() {
            buffer = self.not_empty.wait_timeout(buffer, Duration::from_millis(100)).unwrap().0;
            if buffer.is_empty() {
                return None; // Timeout
            }
        }
        
        let item = buffer.remove(0);
        self.not_full.notify_one();
        Some(item)
    }
}

fn main() {
    println!("🚀 RustFlow Comprehensive Test Suite (Coroutines + Threading)");
    println!("==============================================================");
    
    // Test 1: Simulated coroutine execution
    test_coroutine_simulation();
    
    // Test 2: Basic threading with std::thread
    test_basic_threading();
    
    // Test 3: Atomic operations and lock-free programming
    test_atomic_operations();
    
    // Test 4: Thread synchronization with mutexes
    test_mutex_synchronization();
    
    // Test 5: Thread pool implementation
    test_thread_pool();
    
    // Test 6: Producer-consumer pattern
    test_producer_consumer();
    
    // Test 7: Thread barriers and coordination
    test_thread_barriers();
    
    // Test 8: Mixed coordination patterns
    test_mixed_coordination();
    
    // Test 9: Performance comparison
    test_performance_comparison();
    
    // Test 10: Complex workflow simulation
    test_workflow_simulation();
    
    println!("\n✅ All comprehensive tests completed successfully!");
    println!("📊 Comprehensive Test Summary:");
    println!("  - Coroutine simulation: ✓");
    println!("  - Basic threading: ✓");
    println!("  - Atomic operations: ✓");
    println!("  - Mutex synchronization: ✓");
    println!("  - Thread pool: ✓");
    println!("  - Producer-consumer: ✓");
    println!("  - Thread barriers: ✓");
    println!("  - Mixed coordination: ✓");
    println!("  - Performance testing: ✓");
    println!("  - Workflow simulation: ✓");
    println!("  - Total: 10 tests passed");
}

fn test_coroutine_simulation() {
    print!("🔍 Testing coroutine simulation... ");
    
    let mut coroutines = Vec::new();
    for i in 0..5 {
        coroutines.push(SimpleCoroutine::new(i, 10));
    }
    
    // Simulate cooperative multitasking
    let mut active_count = coroutines.len();
    let mut total_steps = 0;
    
    while active_count > 0 {
        for coroutine in &mut coroutines {
            if !coroutine.is_completed() {
                if !coroutine.step() {
                    active_count -= 1;
                }
                total_steps += 1;
            }
        }
    }
    
    // All coroutines should be completed
    assert!(coroutines.iter().all(|c| c.is_completed()));
    assert!(total_steps > 50); // Should take more steps due to yielding
    
    println!("✓");
}

fn test_basic_threading() {
    print!("🔍 Testing basic threading... ");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    // Spawn 5 threads
    for i in 0..5 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                counter.fetch_add(1, Ordering::SeqCst);
                if j % 25 == 0 {
                    thread::sleep(Duration::from_millis(1));
                }
            }
            i * 100 // Return thread ID * 100
        });
        handles.push(handle);
    }
    
    // Collect results
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.join().unwrap());
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 500);
    assert_eq!(results.len(), 5);
    
    println!("✓");
}

fn test_atomic_operations() {
    print!("🔍 Testing atomic operations... ");
    
    let counters = Arc::new((
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicBool::new(false),
    ));
    
    let mut handles = Vec::new();
    
    for i in 0..8 {
        let counters = Arc::clone(&counters);
        let handle = thread::spawn(move || {
            // Test different atomic operations
            counters.0.fetch_add(i, Ordering::SeqCst);
            counters.1.fetch_sub(i, Ordering::SeqCst);
            
            if i == 3 {
                counters.2.store(true, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(counters.0.load(Ordering::SeqCst), 28); // 0+1+2+...+7
    assert_eq!(counters.1.load(Ordering::SeqCst), 0_usize.wrapping_sub(28));
    assert!(counters.2.load(Ordering::SeqCst));
    
    println!("✓");
}

fn test_mutex_synchronization() {
    print!("🔍 Testing mutex synchronization... ");
    
    let data = Arc::new(Mutex::new(HashMap::new()));
    let mut handles = Vec::new();
    
    for i in 0..6 {
        let data = Arc::clone(&data);
        let handle = thread::spawn(move || {
            let key = format!("key_{}", i % 3);
            let mut map = data.lock().unwrap();
            *map.entry(key).or_insert(0) += i + 1;
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_map = data.lock().unwrap();
    assert_eq!(final_map.len(), 3);
    
    // key_0: 1 + 4 = 5, key_1: 2 + 5 = 7, key_2: 3 + 6 = 9
    assert_eq!(final_map["key_0"], 5);
    assert_eq!(final_map["key_1"], 7);
    assert_eq!(final_map["key_2"], 9);
    
    println!("✓");
}

fn test_thread_pool() {
    print!("🔍 Testing thread pool... ");
    
    let pool = ThreadPool::new(4);
    let results = Arc::new(Mutex::new(Vec::new()));
    let completed = Arc::new(AtomicUsize::new(0));
    
    // Submit 12 tasks
    for i in 0..12 {
        let results = Arc::clone(&results);
        let completed = Arc::clone(&completed);
        
        pool.execute(move || {
            // Simulate work
            thread::sleep(Duration::from_millis(10));
            results.lock().unwrap().push(i * i);
            completed.fetch_add(1, Ordering::SeqCst);
        });
    }
    
    // Wait for all tasks to complete
    while completed.load(Ordering::SeqCst) < 12 {
        thread::sleep(Duration::from_millis(5));
    }
    
    let final_results = results.lock().unwrap();
    assert_eq!(final_results.len(), 12);
    
    let expected_sum: i32 = (0..12).map(|i| i * i).sum();
    let actual_sum: i32 = final_results.iter().sum();
    assert_eq!(actual_sum, expected_sum);
    
    println!("✓");
}

fn test_producer_consumer() {
    print!("🔍 Testing producer-consumer pattern... ");
    
    let pc = Arc::new(ProducerConsumer::new(5));
    let produced_count = Arc::new(AtomicUsize::new(0));
    let consumed_count = Arc::new(AtomicUsize::new(0));
    
    // Producer thread
    let producer = {
        let pc = Arc::clone(&pc);
        let produced = Arc::clone(&produced_count);
        thread::spawn(move || {
            for i in 0..20 {
                pc.produce(i);
                produced.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        })
    };
    
    // Consumer thread
    let consumer = {
        let pc = Arc::clone(&pc);
        let consumed = Arc::clone(&consumed_count);
        thread::spawn(move || {
            let mut sum = 0;
            loop {
                if let Some(item) = pc.consume() {
                    sum += item;
                    consumed.fetch_add(1, Ordering::SeqCst);
                } else {
                    // Timeout, check if we're done
                    if consumed.load(Ordering::SeqCst) >= 20 {
                        break;
                    }
                }
            }
            sum
        })
    };
    
    producer.join().unwrap();
    let sum = consumer.join().unwrap();
    
    assert_eq!(produced_count.load(Ordering::SeqCst), 20);
    assert_eq!(consumed_count.load(Ordering::SeqCst), 20);
    assert_eq!(sum, 190); // Sum of 0..20
    
    println!("✓");
}

fn test_thread_barriers() {
    print!("🔍 Testing thread barriers... ");
    
    let barrier = Arc::new(Barrier::new(4));
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    
    for i in 0..4 {
        let barrier = Arc::clone(&barrier);
        let results = Arc::clone(&results);
        
        let handle = thread::spawn(move || {
            // Phase 1: Different amounts of work
            thread::sleep(Duration::from_millis(i * 10));
            let phase1_time = Instant::now();
            
            // Wait for all threads to complete phase 1
            barrier.wait();
            
            let phase2_time = Instant::now();
            
            // Phase 2: More work after synchronization
            thread::sleep(Duration::from_millis(5));
            
            results.lock().unwrap().push((i, phase1_time, phase2_time));
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_results = results.lock().unwrap();
    assert_eq!(final_results.len(), 4);
    
    // All phase2 times should be very close (within a few ms)
    let phase2_times: Vec<_> = final_results.iter().map(|(_, _, t2)| *t2).collect();
    let min_time = phase2_times.iter().min().unwrap();
    let max_time = phase2_times.iter().max().unwrap();
    
    assert!(max_time.duration_since(*min_time) < Duration::from_millis(10));
    
    println!("✓");
}

fn test_mixed_coordination() {
    print!("🔍 Testing mixed coordination patterns... ");
    
    // Simulate a system with both cooperative (coroutine-like) and preemptive (threaded) tasks
    let shared_state = Arc::new(Mutex::new((0i32, Vec::new())));
    let cooperative_done = Arc::new(AtomicBool::new(false));
    
    // Cooperative task simulation
    let cooperative_thread = {
        let shared_state = Arc::clone(&shared_state);
        let done_flag = Arc::clone(&cooperative_done);
        
        thread::spawn(move || {
            let mut coroutines = Vec::new();
            for i in 0..3 {
                coroutines.push(SimpleCoroutine::new(i, 5));
            }
            
            while !coroutines.iter().all(|c| c.is_completed()) {
                for coroutine in &mut coroutines {
                    if !coroutine.is_completed() {
                        coroutine.step();
                        
                        // Update shared state
                        let mut state = shared_state.lock().unwrap();
                        state.0 += 1;
                        state.1.push(format!("coro_{}", coroutine.id));
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
            
            done_flag.store(true, Ordering::SeqCst);
        })
    };
    
    // Preemptive tasks
    let mut preemptive_handles = Vec::new();
    for i in 0..3 {
        let shared_state = Arc::clone(&shared_state);
        let handle = thread::spawn(move || {
            for j in 0..5 {
                thread::sleep(Duration::from_millis(2));
                let mut state = shared_state.lock().unwrap();
                state.0 += 10;
                state.1.push(format!("thread_{}_{}", i, j));
            }
        });
        preemptive_handles.push(handle);
    }
    
    // Wait for all to complete
    cooperative_thread.join().unwrap();
    for handle in preemptive_handles {
        handle.join().unwrap();
    }
    
    let final_state = shared_state.lock().unwrap();
    
    // Should have contributions from both cooperative and preemptive tasks
    let coro_count = final_state.1.iter().filter(|s| s.starts_with("coro_")).count();
    let thread_count = final_state.1.iter().filter(|s| s.starts_with("thread_")).count();
    
    assert!(coro_count > 0);
    assert_eq!(thread_count, 15); // 3 threads * 5 iterations
    assert!(cooperative_done.load(Ordering::SeqCst));
    
    println!("✓");
}

fn test_performance_comparison() {
    print!("⚡ Testing performance comparison... ");
    
    let iterations = 10000;
    
    // Test 1: Single-threaded
    let single_start = Instant::now();
    let mut single_sum = 0u64;
    for i in 0..iterations {
        single_sum += (i * i) as u64;
    }
    let single_duration = single_start.elapsed();
    
    // Test 2: Multi-threaded
    let multi_start = Instant::now();
    let multi_sum = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    let chunk_size = iterations / 4;
    for chunk in 0..4 {
        let sum_ref = Arc::clone(&multi_sum);
        let handle = thread::spawn(move || {
            let start = chunk * chunk_size;
            let end = if chunk == 3 { iterations } else { (chunk + 1) * chunk_size };
            
            for i in start..end {
                sum_ref.fetch_add((i * i) as usize, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    let multi_duration = multi_start.elapsed();
    
    assert_eq!(single_sum, multi_sum.load(Ordering::SeqCst) as u64);
    
    println!("✓ (single: {:?}, multi: {:?})", single_duration, multi_duration);
}

fn test_workflow_simulation() {
    print!("🔍 Testing complex workflow simulation... ");
    
    // Simulate a processing pipeline with multiple stages
    let stage1_output = Arc::new(Mutex::new(Vec::new()));
    let stage2_output = Arc::new(Mutex::new(Vec::new()));
    let stage3_output = Arc::new(Mutex::new(Vec::new()));
    
    let (stage1_tx, stage1_rx) = mpsc::channel();
    let (stage2_tx, stage2_rx) = mpsc::channel();
    
    // Stage 1: Data generation
    let stage1 = {
        let output = Arc::clone(&stage1_output);
        thread::spawn(move || {
            for i in 0..10 {
                let data = format!("data_{}", i);
                output.lock().unwrap().push(data.clone());
                stage1_tx.send(data).unwrap();
                thread::sleep(Duration::from_millis(2));
            }
        })
    };
    
    // Stage 2: Data processing
    let stage2 = {
        let output = Arc::clone(&stage2_output);
        thread::spawn(move || {
            while let Ok(data) = stage1_rx.recv() {
                let processed = format!("processed_{}", data);
                output.lock().unwrap().push(processed.clone());
                stage2_tx.send(processed).unwrap();
                thread::sleep(Duration::from_millis(3));
            }
        })
    };
    
    // Stage 3: Final processing
    let stage3 = {
        let output = Arc::clone(&stage3_output);
        thread::spawn(move || {
            while let Ok(data) = stage2_rx.recv() {
                let final_data = format!("final_{}", data);
                output.lock().unwrap().push(final_data);
                thread::sleep(Duration::from_millis(1));
            }
        })
    };
    
    // Wait for pipeline to complete
    stage1.join().unwrap();
    stage2.join().unwrap();
    stage3.join().unwrap();
    
    assert_eq!(stage1_output.lock().unwrap().len(), 10);
    assert_eq!(stage2_output.lock().unwrap().len(), 10);
    assert_eq!(stage3_output.lock().unwrap().len(), 10);
    
    // Verify data flow
    let final_data = stage3_output.lock().unwrap();
    assert!(final_data[0].contains("final_processed_data_0"));
    
    println!("✓");
}