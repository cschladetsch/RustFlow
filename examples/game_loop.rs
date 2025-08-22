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
        
        let game_loop = create_game_loop(&factory).await;
        
        let mut root_guard = root.write().await;
        root_guard.add(game_loop).await;
    }
    
    runtime.run_for(Duration::from_secs(5), Duration::from_millis(16)).await?;
    
    Ok(())
}

async fn create_game_loop(factory: &Factory) -> Arc<RwLock<dyn Transient>> {
    let sequence = factory.sequence();
    
    let start_game = create_start_game_phase(factory).await;
    let main_game = create_main_game_phase(factory).await;
    let end_game = create_end_game_phase(factory).await;
    
    {
        let mut seq = sequence.write().await;
        seq.add_step(start_game).await;
        seq.add_step(main_game).await;
        seq.add_step(end_game).await;
        seq.set_name("GameLoop".to_string());
    }
    
    sequence as Arc<RwLock<dyn Transient>>
}

async fn create_start_game_phase(factory: &Factory) -> Arc<RwLock<dyn Transient>> {
    let barrier = factory.barrier();
    
    let init_systems = factory.timer(Duration::from_millis(500));
    let load_assets = factory.timer(Duration::from_millis(300));
    
    {
        let mut bar = barrier.write().await;
        bar.add_dependency(init_systems as Arc<RwLock<dyn Transient>>).await;
        bar.add_dependency(load_assets as Arc<RwLock<dyn Transient>>).await;
        bar.set_name("StartGame".to_string());
    }
    
    info!("Game starting...");
    
    barrier as Arc<RwLock<dyn Transient>>
}

async fn create_main_game_phase(factory: &Factory) -> Arc<RwLock<dyn Transient>> {
    let main_timer = factory.timer(Duration::from_secs(3));
    
    {
        let mut timer = main_timer.write().await;
        timer.set_name("MainGame".to_string());
    }
    
    info!("Main game running...");
    
    main_timer as Arc<RwLock<dyn Transient>>
}

async fn create_end_game_phase(factory: &Factory) -> Arc<RwLock<dyn Transient>> {
    let cleanup_timer = factory.timer(Duration::from_millis(200));
    
    {
        let mut timer = cleanup_timer.write().await;
        timer.set_name("EndGame".to_string());
    }
    
    info!("Game ending...");
    
    cleanup_timer as Arc<RwLock<dyn Transient>>
}