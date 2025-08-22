use rust_flow::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    
    {
        let factory = kernel.read().await.factory().clone();
        let root = kernel.read().await.root();
        
        let async_workflow = create_async_workflow(&factory).await;
        
        let mut root_guard = root.write().await;
        root_guard.add(async_workflow).await;
    }
    
    runtime.run_until_complete(Duration::from_millis(10)).await?;
    
    Ok(())
}

async fn create_async_workflow(factory: &Factory) -> Arc<RwLock<dyn Transient>> {
    let node = factory.node();
    
    let data_future = factory.future::<String>();
    let process_future = factory.future::<i32>();
    let result_future = factory.future::<bool>();
    
    {
        let mut future = data_future.write().await;
        future.set_name("DataFuture".to_string());
    }
    
    {
        let mut future = process_future.write().await;
        future.set_name("ProcessFuture".to_string());
    }
    
    {
        let mut future = result_future.write().await;
        future.set_name("ResultFuture".to_string());
    }
    
    tokio::spawn({
        let data_future = Arc::clone(&data_future);
        async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut future = data_future.write().await;
            let _ = future.set_value("Hello, RustFlow!".to_string()).await;
            info!("Data loaded");
        }
    });
    
    tokio::spawn({
        let process_future = Arc::clone(&process_future);
        async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            let mut future = process_future.write().await;
            let _ = future.set_value(42).await;
            info!("Processing complete");
        }
    });
    
    tokio::spawn({
        let result_future = Arc::clone(&result_future);
        async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let mut future = result_future.write().await;
            let _ = future.set_value(true).await;
            info!("Final result ready");
        }
    });
    
    {
        let mut node_guard = node.write().await;
        node_guard.add(data_future as Arc<RwLock<dyn Transient>>).await;
        node_guard.add(process_future as Arc<RwLock<dyn Transient>>).await;
        node_guard.add(result_future as Arc<RwLock<dyn Transient>>).await;
        node_guard.set_name("AsyncWorkflow".to_string());
    }
    
    info!("Starting async workflow...");
    
    node as Arc<RwLock<dyn Transient>>
}