use rust_flow::*;
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_test;

#[tokio::test]
#[serial]
async fn integration_test_game_loop_simulation() {
    init_tracing();
    
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a complete game loop simulation
    let game_sequence = factory.sequence();
    
    // Phase 1: Initialization
    let init_barrier = factory.barrier();
    let init_systems = factory.timer(Duration::from_millis(10));
    let load_assets = factory.timer(Duration::from_millis(15));
    let setup_input = factory.timer(Duration::from_millis(5));
    
    {
        let mut barrier = init_barrier.write().await;
        barrier.add_dependency(init_systems as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(load_assets as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(setup_input as Arc<RwLock<dyn Transient>>).await;
        barrier.set_name("InitializationPhase".to_string());
    }
    
    // Phase 2: Main game loop with parallel systems
    let main_game_node = factory.node();
    let physics_timer = factory.timer(Duration::from_millis(20));
    let render_timer = factory.timer(Duration::from_millis(25));
    let input_timer = factory.timer(Duration::from_millis(30));
    
    {
        let mut node = main_game_node.write().await;
        node.add(physics_timer as Arc<RwLock<dyn Transient>>).await;
        node.add(render_timer as Arc<RwLock<dyn Transient>>).await;
        node.add(input_timer as Arc<RwLock<dyn Transient>>).await;
        node.set_name("MainGameSystems".to_string());
    }
    
    // Phase 3: Cleanup
    let cleanup_timer = factory.timer(Duration::from_millis(5));
    
    // Assemble the sequence
    {
        let mut sequence = game_sequence.write().await;
        sequence.add_step(init_barrier as Arc<RwLock<dyn Transient>>).await;
        sequence.add_step(main_game_node as Arc<RwLock<dyn Transient>>).await;
        sequence.add_step(cleanup_timer as Arc<RwLock<dyn Transient>>).await;
        sequence.set_name("GameLoop".to_string());
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(game_sequence.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run the game loop
    let start_time = std::time::Instant::now();
    let result = runtime.run_until_complete(Duration::from_millis(2)).await;
    let elapsed = start_time.elapsed();
    
    assert!(result.is_ok());
    assert!(elapsed >= Duration::from_millis(30)); // Should take at least as long as the longest phase
    assert!(elapsed < Duration::from_millis(200)); // But not too long
    
    // Verify the sequence completed
    {
        let sequence_guard = game_sequence.read().await;
        assert!(sequence_guard.is_completed());
    }
}

#[tokio::test]
#[serial]
async fn integration_test_async_data_pipeline() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a data processing pipeline
    let pipeline_sequence = factory.sequence();
    
    // Step 1: Data loading futures
    let data_future1 = factory.future::<String>();
    let data_future2 = factory.future::<i32>();
    let data_future3 = factory.future::<Vec<f64>>();
    
    // Step 2: Processing barrier (waits for all data)
    let processing_barrier = factory.barrier();
    {
        let mut barrier = processing_barrier.write().await;
        barrier.add_dependency(data_future1.clone() as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(data_future2.clone() as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(data_future3.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Step 3: Result aggregation
    let result_future = factory.future::<String>();
    
    // Assemble pipeline
    {
        let mut sequence = pipeline_sequence.write().await;
        sequence.add_step(processing_barrier as Arc<RwLock<dyn Transient>>).await;
        sequence.add_step(result_future.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(pipeline_sequence.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Simulate async data loading
    tokio::spawn({
        let data_future1 = Arc::clone(&data_future1);
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut f = data_future1.write().await;
            let _ = f.set_value("Hello World".to_string()).await;
        }
    });
    
    tokio::spawn({
        let data_future2 = Arc::clone(&data_future2);
        async move {
            tokio::time::sleep(Duration::from_millis(15)).await;
            let mut f = data_future2.write().await;
            let _ = f.set_value(42).await;
        }
    });
    
    tokio::spawn({
        let data_future3 = Arc::clone(&data_future3);
        async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let mut f = data_future3.write().await;
            let _ = f.set_value(vec![1.0, 2.0, 3.0]).await;
        }
    });
    
    // Simulate result processing
    tokio::spawn({
        let result_future = Arc::clone(&result_future);
        let data_future1 = Arc::clone(&data_future1);
        let data_future2 = Arc::clone(&data_future2);
        let data_future3 = Arc::clone(&data_future3);
        
        async move {
            // Wait a bit for the barrier to complete
            tokio::time::sleep(Duration::from_millis(30)).await;
            
            let str_data = data_future1.read().await.value().cloned().unwrap_or_default();
            let int_data = data_future2.read().await.value().cloned().unwrap_or(0);
            let vec_data = data_future3.read().await.value().cloned().unwrap_or_default();
            
            let result = format!("Processed: {} + {} + {} items", str_data, int_data, vec_data.len());
            
            let mut f = result_future.write().await;
            let _ = f.set_value(result).await;
        }
    });
    
    // Run the pipeline
    let result = runtime.run_until_complete(Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Verify results
    {
        let pipeline_guard = pipeline_sequence.read().await;
        assert!(pipeline_guard.is_completed());
    }
    
    {
        let result_guard = result_future.read().await;
        assert!(result_guard.is_ready());
        if let Some(result) = result_guard.value() {
            assert!(result.contains("Processed:"));
        }
    }
}

#[tokio::test]
#[serial]
async fn integration_test_mixed_threading_async() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create mixed workload coordinator
    let coordinator = factory.node();
    let thread_result_future = factory.future::<usize>();
    let async_result_future = factory.future::<String>();
    let timer = factory.timer(Duration::from_millis(30));
    
    {
        let mut node = coordinator.write().await;
        node.add(thread_result_future.clone() as Arc<RwLock<dyn Transient>>).await;
        node.add(async_result_future.clone() as Arc<RwLock<dyn Transient>>).await;
        node.add(timer as Arc<RwLock<dyn Transient>>).await;
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(coordinator.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Start threaded work
    let thread_pool = threading::ThreadPool::new(2);
    let thread_counter = Arc::new(AtomicUsize::new(0));
    
    for i in 0..10 {
        let counter = Arc::clone(&thread_counter);
        let _ = thread_pool.execute(move || {
            // Simulate CPU work
            for _ in 0..1000 {
                counter.fetch_add(i, Ordering::Relaxed);
            }
        });
    }
    
    // Set thread result when work completes
    tokio::spawn({
        let thread_result_future = Arc::clone(&thread_result_future);
        let thread_counter = Arc::clone(&thread_counter);
        async move {
            // Wait for thread work to complete
            tokio::time::sleep(Duration::from_millis(20)).await;
            let final_count = thread_counter.load(Ordering::Relaxed);
            
            let mut f = thread_result_future.write().await;
            let _ = f.set_value(final_count).await;
        }
    });
    
    // Start async work
    tokio::spawn({
        let async_result_future = Arc::clone(&async_result_future);
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut result = String::new();
            
            for i in 0..5 {
                tokio::time::sleep(Duration::from_millis(2)).await;
                result.push_str(&format!("async_{} ", i));
            }
            
            let mut f = async_result_future.write().await;
            let _ = f.set_value(result).await;
        }
    });
    
    // Run until completion
    let result = runtime.run_until_complete(Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Verify both async and threaded work completed
    {
        let thread_result = thread_result_future.read().await;
        let async_result = async_result_future.read().await;
        
        assert!(thread_result.is_ready());
        assert!(async_result.is_ready());
        
        if let Some(count) = thread_result.value() {
            assert!(*count > 0);
        }
        
        if let Some(text) = async_result.value() {
            assert!(text.contains("async_"));
        }
    }
}

#[tokio::test]
#[serial]
async fn integration_test_complex_dependency_graph() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a complex dependency graph
    //     A   B   C
    //     |   |   |
    //     D   E   F
    //      \ | /
    //        G
    //        |
    //        H
    
    let future_a = factory.future::<i32>();
    let future_b = factory.future::<i32>();
    let future_c = factory.future::<i32>();
    
    let timer_d = factory.timer(Duration::from_millis(10));
    let timer_e = factory.timer(Duration::from_millis(15));
    let timer_f = factory.timer(Duration::from_millis(20));
    
    let barrier_g = factory.barrier();
    let final_future_h = factory.future::<String>();
    
    // Set up dependencies
    {
        let mut barrier = barrier_g.write().await;
        barrier.add_dependency(timer_d as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(timer_e as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(timer_f as Arc<RwLock<dyn Transient>>).await;
    }
    
    let main_sequence = factory.sequence();
    {
        let mut sequence = main_sequence.write().await;
        sequence.add_step(barrier_g as Arc<RwLock<dyn Transient>>).await;
        sequence.add_step(final_future_h.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(future_a.clone() as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(future_b.clone() as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(future_c.clone() as Arc<RwLock<dyn Transient>>).await;
        root_guard.add(main_sequence.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Set initial values
    tokio::spawn({
        let future_a = Arc::clone(&future_a);
        async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let mut f = future_a.write().await;
            let _ = f.set_value(1).await;
        }
    });
    
    tokio::spawn({
        let future_b = Arc::clone(&future_b);
        async move {
            tokio::time::sleep(Duration::from_millis(7)).await;
            let mut f = future_b.write().await;
            let _ = f.set_value(2).await;
        }
    });
    
    tokio::spawn({
        let future_c = Arc::clone(&future_c);
        async move {
            tokio::time::sleep(Duration::from_millis(12)).await;
            let mut f = future_c.write().await;
            let _ = f.set_value(3).await;
        }
    });
    
    // Set final result when barrier completes
    tokio::spawn({
        let final_future_h = Arc::clone(&final_future_h);
        let future_a = Arc::clone(&future_a);
        let future_b = Arc::clone(&future_b);
        let future_c = Arc::clone(&future_c);
        
        async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            
            let a = future_a.read().await.value().cloned().unwrap_or(0);
            let b = future_b.read().await.value().cloned().unwrap_or(0);
            let c = future_c.read().await.value().cloned().unwrap_or(0);
            
            let result = format!("Final result: {} + {} + {} = {}", a, b, c, a + b + c);
            
            let mut f = final_future_h.write().await;
            let _ = f.set_value(result).await;
        }
    });
    
    // Run until completion
    let result = runtime.run_until_complete(Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Verify the entire dependency graph resolved
    {
        let sequence_guard = main_sequence.read().await;
        assert!(sequence_guard.is_completed());
    }
    
    {
        let final_guard = final_future_h.read().await;
        assert!(final_guard.is_ready());
        if let Some(result) = final_guard.value() {
            assert!(result.contains("Final result:"));
            assert!(result.contains("= 6"));
        }
    }
}

#[tokio::test]
#[serial]
async fn integration_test_error_recovery() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a system that can handle timeouts and failures
    let main_node = factory.node();
    let primary_future = factory.timed_future::<String>(Duration::from_millis(10)); // Will timeout
    let backup_future = factory.future::<String>();
    let final_barrier = factory.barrier();
    
    {
        let mut barrier = final_barrier.write().await;
        barrier.add_dependency(primary_future.clone() as Arc<RwLock<dyn Transient>>).await;
        barrier.add_dependency(backup_future.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    {
        let mut node = main_node.write().await;
        node.add(final_barrier.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(main_node as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Backup system activates when primary times out
    tokio::spawn({
        let backup_future = Arc::clone(&backup_future);
        let primary_future = Arc::clone(&primary_future);
        
        async move {
            tokio::time::sleep(Duration::from_millis(15)).await;
            
            // Check if primary timed out
            let primary_timed_out = {
                let guard = primary_future.read().await;
                guard.is_timed_out()
            };
            
            if primary_timed_out {
                let mut f = backup_future.write().await;
                let _ = f.set_value("Backup system activated!".to_string()).await;
            }
        }
    });
    
    // Run the system
    let result = runtime.run_for(Duration::from_millis(30), Duration::from_millis(2)).await;
    assert!(result.is_ok());
    
    // Verify error recovery worked
    {
        let primary_guard = primary_future.read().await;
        let backup_guard = backup_future.read().await;
        let barrier_guard = final_barrier.read().await;
        
        assert!(primary_guard.is_timed_out());
        assert!(backup_guard.is_ready());
        assert!(barrier_guard.all_completed());
        
        if let Some(backup_value) = backup_guard.value() {
            assert!(backup_value.contains("Backup"));
        }
    }
}

#[tokio::test]
#[serial]
async fn integration_test_performance_under_load() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a high-load scenario with many components
    let component_count = 100;
    let mut all_components = Vec::new();
    
    for i in 0..component_count {
        let component: Arc<RwLock<dyn Transient>> = match i % 5 {
            0 => factory.timer(Duration::from_millis((i % 20) + 1)) as Arc<RwLock<dyn Transient>>,
            1 => factory.future_with_value(i) as Arc<RwLock<dyn Transient>>,
            2 => {
                let group = factory.group();
                let timer = factory.timer(Duration::from_millis(5));
                {
                    let mut g = group.write().await;
                    g.add(timer as Arc<RwLock<dyn Transient>>).await;
                }
                group as Arc<RwLock<dyn Transient>>
            },
            3 => {
                let sequence = factory.sequence();
                let timer1 = factory.timer(Duration::from_millis(2));
                let timer2 = factory.timer(Duration::from_millis(3));
                {
                    let mut s = sequence.write().await;
                    s.add_step(timer1 as Arc<RwLock<dyn Transient>>).await;
                    s.add_step(timer2 as Arc<RwLock<dyn Transient>>).await;
                }
                sequence as Arc<RwLock<dyn Transient>>
            },
            4 => {
                let barrier = factory.barrier();
                let dep1 = factory.timer(Duration::from_millis(1));
                let dep2 = factory.timer(Duration::from_millis(2));
                {
                    let mut b = barrier.write().await;
                    b.add_dependency(dep1 as Arc<RwLock<dyn Transient>>).await;
                    b.add_dependency(dep2 as Arc<RwLock<dyn Transient>>).await;
                }
                barrier as Arc<RwLock<dyn Transient>>
            },
            _ => unreachable!(),
        };
        
        all_components.push(component);
    }
    
    // Add all components to root
    {
        let mut root_guard = root.write().await;
        for component in all_components {
            root_guard.add(component).await;
        }
    }
    
    // Run under load and measure performance
    let start_time = std::time::Instant::now();
    let result = runtime.run_for(Duration::from_millis(100), Duration::from_millis(1)).await;
    let elapsed = start_time.elapsed();
    
    assert!(result.is_ok());
    
    // Should complete within reasonable time even under load
    assert!(elapsed < Duration::from_millis(500));
    
    // Check that the system remained responsive
    {
        let kernel_guard = kernel.read().await;
        assert!(kernel_guard.step_number() > 50); // Should have stepped many times
    }
}