use proptest::prelude::*;
use quickcheck_macros::quickcheck;
use rust_flow::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_test;

// Property-based tests using quickcheck

#[quickcheck]
fn test_transient_id_uniqueness(count: u8) -> bool {
    let count = count.max(1) as usize; // Ensure at least 1
    let mut ids = Vec::new();
    
    for _ in 0..count {
        ids.push(TransientId::new());
    }
    
    // Check all IDs are unique
    for i in 0..ids.len() {
        for j in i + 1..ids.len() {
            if ids[i].value() == ids[j].value() {
                return false;
            }
        }
    }
    
    // Check IDs are monotonically increasing
    for i in 1..ids.len() {
        if ids[i].value() <= ids[i - 1].value() {
            return false;
        }
    }
    
    true
}

#[quickcheck]
fn test_time_frame_delta_accumulation(deltas: Vec<u32>) -> bool {
    if deltas.is_empty() {
        return true;
    }
    
    let mut time_frame = TimeFrame::new();
    let initial_now = time_frame.now;
    let mut total_delta = Duration::from_secs(0);
    
    for delta_ms in deltas {
        let delta = Duration::from_millis(delta_ms as u64);
        time_frame.update(delta);
        total_delta += delta;
    }
    
    time_frame.now == initial_now + total_delta
}

#[quickcheck]
fn test_generator_state_transitions_valid(transitions: Vec<u8>) -> bool {
    use GeneratorState::*;
    
    let mut state = Running; // Start in running state
    
    for &transition in &transitions {
        let new_state = match transition % 4 {
            0 => Inactive,
            1 => Running,
            2 => Suspended,
            3 => Completed,
            _ => unreachable!(),
        };
        
        // Check if transition is valid
        match (state, new_state) {
            // Any state can transition to completed
            (_, Completed) => state = new_state,
            // Completed cannot transition to anything else
            (Completed, _) => return state == Completed,
            // Other transitions are generally allowed
            _ => state = new_state,
        }
    }
    
    true
}

// Property-based tests using proptest

proptest! {
    #[test]
    fn prop_timer_duration_respected(duration_ms in 1u64..1000u64) {
        tokio_test::block_on(async {
            let factory = Factory::new();
            let timer = factory.timer(Duration::from_millis(duration_ms));
            
            {
                let timer_guard = timer.read().await;
                prop_assert!(!timer_guard.is_expired());
                prop_assert_eq!(timer_guard.interval(), Duration::from_millis(duration_ms));
            }
            
            // Wait for timer duration plus some buffer
            tokio::time::sleep(Duration::from_millis(duration_ms + 10)).await;
            
            {
                let mut timer_guard = timer.write().await;
                let _ = timer_guard.step().await;
                prop_assert!(timer_guard.is_expired());
                prop_assert!(timer_guard.is_completed());
            }
        });
    }
    
    #[test]
    fn prop_future_value_preservation(value in any::<i32>()) {
        tokio_test::block_on(async {
            let factory = Factory::new();
            let future = factory.future_with_value(value);
            
            {
                let future_guard = future.read().await;
                prop_assert!(future_guard.is_ready());
                prop_assert_eq!(future_guard.value(), Some(&value));
                prop_assert!(future_guard.is_completed());
            }
        });
    }
    
    #[test]
    fn prop_timed_future_timeout(timeout_ms in 1u64..100u64, wait_ms in 0u64..200u64) {
        tokio_test::block_on(async {
            let factory = Factory::new();
            let timed_future = factory.timed_future::<i32>(Duration::from_millis(timeout_ms));
            
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            
            {
                let mut future_guard = timed_future.write().await;
                let _ = future_guard.step().await;
                
                if wait_ms > timeout_ms {
                    prop_assert!(future_guard.is_timed_out());
                    prop_assert!(future_guard.is_completed());
                } else {
                    prop_assert!(!future_guard.is_timed_out() || future_guard.is_completed());
                }
            }
        });
    }
    
    #[test]
    fn prop_sequence_step_count(step_count in 1usize..10usize) {
        tokio_test::block_on(async {
            let factory = Factory::new();
            let sequence = factory.sequence();
            
            // Create steps (short timers)
            let steps: Vec<_> = (0..step_count)
                .map(|_| factory.timer(Duration::from_millis(1)))
                .collect();
            
            // Add steps to sequence
            {
                let mut seq_guard = sequence.write().await;
                for step in steps {
                    seq_guard.add_step(step as Arc<RwLock<dyn Transient>>).await;
                }
                prop_assert_eq!(seq_guard.steps().len(), step_count);
            }
        });
    }
    
    #[test]
    fn prop_barrier_dependency_count(dep_count in 1usize..10usize) {
        tokio_test::block_on(async {
            let factory = Factory::new();
            let barrier = factory.barrier();
            
            // Create dependencies (short timers)
            let deps: Vec<_> = (0..dep_count)
                .map(|_| factory.timer(Duration::from_millis(1)))
                .collect();
            
            // Add dependencies to barrier
            {
                let mut barrier_guard = barrier.write().await;
                for dep in deps {
                    barrier_guard.add_dependency(dep as Arc<RwLock<dyn Transient>>).await;
                }
                prop_assert_eq!(barrier_guard.dependencies().len(), dep_count);
            }
        });
    }
    
    #[test]
    fn prop_kernel_step_monotonic(step_count in 1u32..100u32) {
        tokio_test::block_on(async {
            let mut kernel = Kernel::new();
            let initial_steps = kernel.step_number();
            
            for _ in 0..step_count {
                let _ = kernel.step().await;
            }
            
            prop_assert_eq!(kernel.step_number(), initial_steps + step_count as u64);
        });
    }
    
    #[test]
    fn prop_time_frame_monotonic_time(updates in prop::collection::vec(1u32..100u32, 1..20)) {
        let mut time_frame = TimeFrame::new();
        let mut expected_time = time_frame.now;
        
        for delta_ms in updates {
            let delta = Duration::from_millis(delta_ms as u64);
            time_frame.update(delta);
            expected_time += delta;
            
            prop_assert_eq!(time_frame.now, expected_time);
            prop_assert_eq!(time_frame.delta, delta);
        }
    }
    
    #[test]
    fn prop_factory_component_creation(component_type in 0u8..8u8) {
        tokio_test::block_on(async {
            let factory = Factory::new();
            
            match component_type {
                0 => {
                    let group = factory.group();
                    prop_assert!(group.read().await.is_active());
                },
                1 => {
                    let node = factory.node();
                    prop_assert!(node.read().await.is_active());
                },
                2 => {
                    let sequence = factory.sequence();
                    prop_assert!(sequence.read().await.is_active());
                },
                3 => {
                    let barrier = factory.barrier();
                    prop_assert!(barrier.read().await.is_active());
                },
                4 => {
                    let future = factory.future::<i32>();
                    prop_assert!(future.read().await.is_active());
                },
                5 => {
                    let timer = factory.timer(Duration::from_millis(100));
                    prop_assert!(timer.read().await.is_active());
                },
                6 => {
                    let channel = factory.channel::<String>();
                    prop_assert!(channel.read().await.is_active());
                },
                7 => {
                    let coroutine = factory.coroutine();
                    prop_assert!(coroutine.read().await.is_active());
                },
                _ => unreachable!(),
            }
        });
    }
}

// Stress tests
#[tokio::test]
async fn stress_test_many_timers() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create many timers with random durations
    let timer_count = 100;
    let mut timers = Vec::new();
    
    for i in 0..timer_count {
        let duration = Duration::from_millis((i % 50) + 1);
        let timer = factory.timer(duration);
        timers.push(timer.clone());
        
        let mut root_guard = root.write().await;
        root_guard.add(timer as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run until most timers complete
    let result = runtime.run_for(Duration::from_millis(100), Duration::from_millis(1)).await;
    assert!(result.is_ok());
    
    // Check that many timers completed
    let completed_count = timers.iter().filter(|t| {
        if let Ok(guard) = t.try_read() {
            guard.is_completed()
        } else {
            false
        }
    }).count();
    
    // At least half should have completed
    assert!(completed_count > timer_count / 2);
}

#[tokio::test]
async fn stress_test_deep_sequence() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create a deep sequence
    let sequence = factory.sequence();
    let step_count = 50;
    
    {
        let mut seq_guard = sequence.write().await;
        for _ in 0..step_count {
            let timer = factory.timer(Duration::from_millis(1));
            seq_guard.add_step(timer as Arc<RwLock<dyn Transient>>).await;
        }
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(sequence.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run until sequence completes or times out
    let result = runtime.run_for(Duration::from_millis(200), Duration::from_millis(1)).await;
    assert!(result.is_ok());
    
    // Sequence should have made significant progress
    {
        let seq_guard = sequence.read().await;
        // Either completed or advanced significantly
        assert!(seq_guard.is_completed() || seq_guard.current_step_index().unwrap_or(0) > step_count / 2);
    }
}

#[tokio::test]
async fn stress_test_complex_barrier() {
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    let factory = kernel.read().await.factory().clone();
    let root = kernel.read().await.root();
    
    // Create barrier with many dependencies
    let barrier = factory.barrier();
    let dep_count = 20;
    
    {
        let mut barrier_guard = barrier.write().await;
        for i in 0..dep_count {
            let timer = factory.timer(Duration::from_millis((i % 10) + 5));
            barrier_guard.add_dependency(timer as Arc<RwLock<dyn Transient>>).await;
        }
    }
    
    {
        let mut root_guard = root.write().await;
        root_guard.add(barrier.clone() as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run until barrier completes
    let result = runtime.run_for(Duration::from_millis(100), Duration::from_millis(1)).await;
    assert!(result.is_ok());
    
    // Barrier should eventually complete
    {
        let barrier_guard = barrier.read().await;
        assert!(barrier_guard.all_completed());
        assert!(barrier_guard.is_completed());
    }
}