# RustFlow

A Rust-based coroutine and flow control system inspired by CsharpFlow, designed for both async/await and threading workloads.

## Features

- **Async/Await Support**: Built on top of Tokio for asynchronous execution
- **Threading Support**: Crossbeam-based threading utilities for CPU-bound tasks  
- **Flow Control**: Sequences, barriers, triggers, and more for coordinating execution
- **Timers and Futures**: Time-based and value-based coordination primitives
- **Clean API**: Fluent, composable API similar to the original CsharpFlow

## Core Concepts

### Transients
Base objects that can be active, completed, named, and stepped through time.

### Generators  
Transients that produce values and can be resumed, suspended, and stepped.

### Flow Components
- **Node**: Executes all child generators when stepped
- **Group**: Contains transients but doesn't step them automatically  
- **Sequence**: Executes generators one after another
- **Barrier**: Waits for all dependencies to complete
- **Future**: Represents a value that will be available later
- **Timer**: Completes after a specified duration
- **Channel**: Message passing between flow components

## Quick Start

Add to your `Cargo.toml`:
```toml
[dependencies]
rust_flow = { path = "." }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
```

Basic example:
```rust
use rust_flow::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    
    let runtime = Runtime::new();
    let kernel = runtime.kernel();
    
    {
        let factory = kernel.read().await.factory().clone();
        let root = kernel.read().await.root();
        
        // Create a simple sequence
        let sequence = factory.sequence();
        let timer1 = factory.timer(Duration::from_secs(1));
        let timer2 = factory.timer(Duration::from_secs(2));
        
        {
            let mut seq = sequence.write().await;
            seq.add_step(timer1 as Arc<RwLock<dyn Transient>>).await;
            seq.add_step(timer2 as Arc<RwLock<dyn Transient>>).await;
        }
        
        let mut root_guard = root.write().await;
        root_guard.add(sequence as Arc<RwLock<dyn Transient>>).await;
    }
    
    // Run for 5 seconds with 16ms frame time
    runtime.run_for(Duration::from_secs(5), Duration::from_millis(16)).await?;
    
    Ok(())
}
```

## Examples

See the `examples/` directory for more comprehensive examples:

- `game_loop.rs`: Shows a typical game loop structure with start/main/end phases
- `async_tasks.rs`: Demonstrates async task coordination with futures

Run examples with:
```bash
cargo run --example game_loop
cargo run --example async_tasks
```

## Architecture

RustFlow is built around several core concepts:

1. **Kernel**: The main execution engine that steps through flow components
2. **Runtime**: Provides frame-based execution with configurable timing
3. **Factory**: Creates all flow components with proper lifecycle management
4. **Flow Components**: Various primitives for coordinating execution

### Threading vs Async

RustFlow supports both paradigms:
- Use async/await for I/O-bound tasks and natural async coordination
- Use the threading module for CPU-bound parallel work
- Mix both approaches as needed in the same application

## Comparison with CsharpFlow

| CsharpFlow | RustFlow | Notes |
|------------|----------|--------|
| IKernel | Kernel | Main execution engine |
| IFactory | Factory | Component creation |
| IGenerator | Generator trait | Base execution unit |
| INode | Node | Executes child components |
| ISequence | Sequence | Sequential execution |
| IBarrier | Barrier | Wait for dependencies |
| IFuture | Future | Async value container |
| ITimer | Timer | Time-based completion |

## Testing

RustFlow includes a comprehensive test suite with multiple testing strategies:

### Unit Tests
Core functionality tests embedded in source files:
```bash
cargo test --lib
```

### Integration Tests  
Full system tests demonstrating real-world scenarios:
```bash
cargo test --test test_integration
```

### Component Tests
Detailed tests for individual flow components:
```bash  
cargo test --test test_flow_components
cargo test --test test_kernel_runtime
cargo test --test test_threading_async
```

### Property-Based Tests
Automated testing with generated inputs using quickcheck and proptest:
```bash
cargo test --test test_property_based
```

### Benchmarks
Performance testing and optimization:
```bash
cargo bench
```

### Run All Tests
```bash
cargo test
```

## Test Coverage

The test suite covers:

- **Unit Tests**: Core types, traits, and basic functionality
- **Component Tests**: All flow components (timers, futures, sequences, barriers, etc.)
- **Kernel Tests**: Core execution engine functionality  
- **Runtime Tests**: Frame-based execution and lifecycle management
- **Threading Tests**: Multi-threaded execution and coordination
- **Async Tests**: Future-based coordination and async/await patterns
- **Integration Tests**: Complete workflows and complex scenarios
- **Property Tests**: Invariant checking with generated test cases
- **Stress Tests**: High-load scenarios and performance validation
- **Benchmarks**: Performance measurement and regression detection

## Development

Build and test:
```bash
cargo build
cargo test
cargo clippy
cargo bench
```

## License

MIT License - see LICENSE file for details.