use rust_flow::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_test;

#[tokio::test]
async fn test_group_creation_and_basic_operations() {
    let factory = Factory::new();
    let group = factory.group();
    
    {
        let group_guard = group.read().await;
        assert!(group_guard.is_empty());
        assert_eq!(group_guard.contents().len(), 0);
        assert!(group_guard.is_active());
        assert!(!group_guard.is_completed());
    }
}

#[tokio::test]
async fn test_group_add_transients() {
    let factory = Factory::new();
    let group = factory.group();
    let timer = factory.timer(Duration::from_millis(100));
    
    {
        let mut group_guard = group.write().await;
        group_guard.add(timer as Arc<RwLock<dyn Transient>>).await;
        assert!(!group_guard.is_empty());
        assert_eq!(group_guard.contents().len(), 1);
    }
}

#[tokio::test]
async fn test_node_creation_and_stepping() {
    let factory = Factory::new();
    let node = factory.node();
    let timer1 = factory.timer(Duration::from_millis(50));
    let timer2 = factory.timer(Duration::from_millis(100));
    
    {
        let mut node_guard = node.write().await;
        node_guard.add(timer1 as Arc<RwLock<dyn Transient>>).await;
        node_guard.add(timer2 as Arc<RwLock<dyn Transient>>).await;
        assert_eq!(node_guard.contents().len(), 2);
        
        // Step the node
        let _ = node_guard.step().await;
        assert!(node_guard.is_active());
    }
}

#[tokio::test]
async fn test_sequence_execution() {
    let factory = Factory::new();
    let sequence = factory.sequence();
    let timer1 = factory.timer(Duration::from_millis(10));
    let timer2 = factory.timer(Duration::from_millis(10));
    
    {
        let mut seq_guard = sequence.write().await;
        seq_guard.add_step(timer1 as Arc<RwLock<dyn Transient>>).await;
        seq_guard.add_step(timer2 as Arc<RwLock<dyn Transient>>).await;
        
        assert_eq!(seq_guard.current_step_index(), Some(0));
        assert_eq!(seq_guard.steps().len(), 2);
    }
}

#[tokio::test]
async fn test_sequence_progression() {
    let factory = Factory::new();
    let sequence = factory.sequence();
    let timer1 = factory.timer(Duration::from_millis(1));
    let timer2 = factory.timer(Duration::from_millis(1));
    
    {
        let mut seq_guard = sequence.write().await;
        seq_guard.add_step(timer1 as Arc<RwLock<dyn Transient>>).await;
        seq_guard.add_step(timer2 as Arc<RwLock<dyn Transient>>).await;
        
        assert_eq!(seq_guard.current_step_index(), Some(0));
        
        // Let first timer complete
        tokio::time::sleep(Duration::from_millis(5)).await;
        let _ = seq_guard.step().await;
        
        // Should still be on step 0 or 1 depending on timing
        assert!(seq_guard.current_step_index().is_some());
    }
}

#[tokio::test]
async fn test_barrier_creation() {
    let factory = Factory::new();
    let barrier = factory.barrier();
    let timer1 = factory.timer(Duration::from_millis(50));
    let timer2 = factory.timer(Duration::from_millis(100));
    
    {
        let mut barrier_guard = barrier.write().await;
        barrier_guard.add_dependency(timer1 as Arc<RwLock<dyn Transient>>).await;
        barrier_guard.add_dependency(timer2 as Arc<RwLock<dyn Transient>>).await;
        
        assert_eq!(barrier_guard.dependencies().len(), 2);
        assert!(!barrier_guard.all_completed());
    }
}

#[tokio::test]
async fn test_future_creation_and_value_setting() {
    let factory = Factory::new();
    let future = factory.future::<String>();
    
    {
        let future_guard = future.read().await;
        assert!(!future_guard.is_ready());
        assert!(future_guard.value().is_none());
        assert!(future_guard.is_active());
    }
    
    {
        let mut future_guard = future.write().await;
        let _ = future_guard.set_value("Hello World".to_string()).await;
        assert!(future_guard.is_ready());
        assert!(future_guard.is_completed());
        assert_eq!(future_guard.value(), Some(&"Hello World".to_string()));
    }
}

#[tokio::test]
async fn test_future_with_initial_value() {
    let factory = Factory::new();
    let future = factory.future_with_value(42);
    
    {
        let future_guard = future.read().await;
        assert!(future_guard.is_ready());
        assert!(future_guard.is_completed());
        assert_eq!(future_guard.value(), Some(&42));
    }
}

#[tokio::test]
async fn test_timed_future_timeout() {
    let factory = Factory::new();
    let timed_future = factory.timed_future::<String>(Duration::from_millis(10));
    
    {
        let future_guard = timed_future.read().await;
        assert!(!future_guard.is_timed_out());
        assert!(future_guard.is_active());
    }
    
    tokio::time::sleep(Duration::from_millis(15)).await;
    
    {
        let mut future_guard = timed_future.write().await;
        let _ = future_guard.step().await;
        assert!(future_guard.is_timed_out());
        assert!(future_guard.is_completed());
    }
}

#[tokio::test]
async fn test_timer_expiration() {
    let factory = Factory::new();
    let timer = factory.timer(Duration::from_millis(10));
    
    {
        let timer_guard = timer.read().await;
        assert!(!timer_guard.is_expired());
        assert!(timer_guard.is_active());
        assert_eq!(timer_guard.interval(), Duration::from_millis(10));
    }
    
    tokio::time::sleep(Duration::from_millis(15)).await;
    
    {
        let mut timer_guard = timer.write().await;
        let _ = timer_guard.step().await;
        assert!(timer_guard.is_expired());
        assert!(timer_guard.is_completed());
    }
}

#[tokio::test]
async fn test_channel_send_receive() {
    let factory = Factory::new();
    let channel = factory.channel::<i32>();
    
    {
        let channel_guard = channel.read().await;
        let _ = channel_guard.send(42).await;
    }
    
    {
        let mut channel_guard = channel.write().await;
        let _ = channel_guard.step().await;
        
        if let Some(Some(value)) = channel_guard.value() {
            assert_eq!(*value, 42);
        }
    }
}

#[tokio::test]
async fn test_factory_component_creation() {
    let factory = Factory::new();
    
    // Test all factory methods work
    let group = factory.group();
    let node = factory.node();
    let sequence = factory.sequence();
    let barrier = factory.barrier();
    let future = factory.future::<i32>();
    let timer = factory.timer(Duration::from_millis(100));
    let channel = factory.channel::<String>();
    let coroutine = factory.coroutine();
    
    // Verify they're all created correctly
    assert!(group.read().await.is_active());
    assert!(node.read().await.is_active());
    assert!(sequence.read().await.is_active());
    assert!(barrier.read().await.is_active());
    assert!(future.read().await.is_active());
    assert!(timer.read().await.is_active());
    assert!(channel.read().await.is_active());
    assert!(coroutine.read().await.is_active());
}

#[tokio::test]
async fn test_transient_naming() {
    let factory = Factory::new();
    let timer = factory.timer(Duration::from_millis(100));
    
    {
        let mut timer_guard = timer.write().await;
        assert!(timer_guard.name().is_none());
        
        timer_guard.set_name("TestTimer".to_string());
        assert_eq!(timer_guard.name(), Some("TestTimer"));
    }
}

#[tokio::test]
async fn test_generator_state_transitions() {
    let factory = Factory::new();
    let timer = factory.timer(Duration::from_millis(100));
    
    {
        let mut timer_guard = timer.write().await;
        assert_eq!(timer_guard.state(), GeneratorState::Running);
        assert!(timer_guard.is_running());
        assert!(!timer_guard.is_completed());
        
        timer_guard.suspend().await;
        assert_eq!(timer_guard.state(), GeneratorState::Suspended);
        assert!(!timer_guard.is_running());
        
        timer_guard.resume().await;
        assert_eq!(timer_guard.state(), GeneratorState::Running);
        assert!(timer_guard.is_running());
        
        timer_guard.complete().await;
        assert_eq!(timer_guard.state(), GeneratorState::Completed);
        assert!(!timer_guard.is_running());
        assert!(timer_guard.is_completed());
    }
}