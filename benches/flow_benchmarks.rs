use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rust_flow::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

fn bench_transient_id_creation(c: &mut Criterion) {
    c.bench_function("transient_id_creation", |b| {
        b.iter(|| {
            black_box(TransientId::new());
        })
    });
}

fn bench_time_frame_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("time_frame");
    
    group.bench_function("creation", |b| {
        b.iter(|| {
            black_box(TimeFrame::new());
        })
    });
    
    group.bench_function("update", |b| {
        let mut time_frame = TimeFrame::new();
        let delta = Duration::from_millis(16);
        
        b.iter(|| {
            time_frame.update(black_box(delta));
        })
    });
    
    group.bench_function("step", |b| {
        let mut time_frame = TimeFrame::new();
        
        b.iter(|| {
            time_frame.step();
        })
    });
    
    group.finish();
}

fn bench_factory_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("factory_creation");
    
    group.bench_function("group", |b| {
        let factory = Factory::new();
        b.iter(|| {
            rt.block_on(async {
                black_box(factory.group());
            })
        })
    });
    
    group.bench_function("node", |b| {
        let factory = Factory::new();
        b.iter(|| {
            rt.block_on(async {
                black_box(factory.node());
            })
        })
    });
    
    group.bench_function("timer", |b| {
        let factory = Factory::new();
        let duration = Duration::from_millis(100);
        b.iter(|| {
            rt.block_on(async {
                black_box(factory.timer(duration));
            })
        })
    });
    
    group.bench_function("future", |b| {
        let factory = Factory::new();
        b.iter(|| {
            rt.block_on(async {
                black_box(factory.future::<i32>());
            })
        })
    });
    
    group.finish();
}

fn bench_kernel_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("kernel_operations");
    
    group.bench_function("creation", |b| {
        b.iter(|| {
            black_box(Kernel::new());
        })
    });
    
    group.bench_function("single_step", |b| {
        b.to_async(&rt).iter(|| async {
            let mut kernel = Kernel::new();
            black_box(kernel.step().await).unwrap();
        })
    });
    
    group.bench_function("update", |b| {
        let delta = Duration::from_millis(16);
        b.to_async(&rt).iter(|| async {
            let mut kernel = Kernel::new();
            black_box(kernel.update(delta).await).unwrap();
        })
    });
    
    group.finish();
}

fn bench_flow_component_stepping(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("component_stepping");
    
    for size in [1, 10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("node_with_timers", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let factory = Factory::new();
                let node = factory.node();
                
                // Add timers to node
                {
                    let mut node_guard = node.write().await;
                    for _ in 0..size {
                        let timer = factory.timer(Duration::from_millis(1000)); // Long duration
                        node_guard.add(timer as Arc<RwLock<dyn Transient>>).await;
                    }
                }
                
                // Step the node
                {
                    let mut node_guard = node.write().await;
                    black_box(node_guard.step().await).unwrap();
                }
            })
        });
    }
    
    group.finish();
}

fn bench_sequence_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("sequence_operations");
    
    for size in [5, 20, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("sequence_creation", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let factory = Factory::new();
                let sequence = factory.sequence();
                
                {
                    let mut seq_guard = sequence.write().await;
                    for _ in 0..size {
                        let timer = factory.timer(Duration::from_millis(100));
                        seq_guard.add_step(timer as Arc<RwLock<dyn Transient>>).await;
                    }
                }
                
                black_box(sequence);
            })
        });
        
        group.bench_with_input(BenchmarkId::new("sequence_stepping", size), size, |b, &size| {
            b.to_async(&rt).iter_batched(
                || {
                    rt.block_on(async {
                        let factory = Factory::new();
                        let sequence = factory.sequence();
                        
                        {
                            let mut seq_guard = sequence.write().await;
                            for _ in 0..size {
                                let timer = factory.timer(Duration::from_millis(1));
                                seq_guard.add_step(timer as Arc<RwLock<dyn Transient>>).await;
                            }
                        }
                        
                        sequence
                    })
                },
                |sequence| async move {
                    let mut seq_guard = sequence.write().await;
                    black_box(seq_guard.step().await).unwrap();
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    
    group.finish();
}

fn bench_barrier_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("barrier_operations");
    
    for size in [5, 20, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("barrier_dependency_check", size), size, |b, &size| {
            b.to_async(&rt).iter_batched(
                || {
                    rt.block_on(async {
                        let factory = Factory::new();
                        let barrier = factory.barrier();
                        
                        {
                            let mut barrier_guard = barrier.write().await;
                            for _ in 0..size {
                                let timer = factory.timer(Duration::from_millis(1000)); // Won't complete
                                barrier_guard.add_dependency(timer as Arc<RwLock<dyn Transient>>).await;
                            }
                        }
                        
                        barrier
                    })
                },
                |barrier| async move {
                    let barrier_guard = barrier.read().await;
                    black_box(barrier_guard.all_completed());
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    
    group.finish();
}

fn bench_runtime_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("runtime_performance");
    
    group.bench_function("single_frame", |b| {
        b.to_async(&rt).iter(|| async {
            let runtime = Runtime::new();
            let delta = Duration::from_millis(16);
            black_box(runtime.run_frame(delta).await).unwrap();
        })
    });
    
    for component_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("frame_with_components", component_count), 
            component_count, 
            |b, &count| {
                b.to_async(&rt).iter_batched(
                    || {
                        rt.block_on(async {
                            let runtime = Runtime::new();
                            let kernel = runtime.kernel();
                            let factory = kernel.read().await.factory().clone();
                            let root = kernel.read().await.root();
                            
                            // Add various components
                            {
                                let mut root_guard = root.write().await;
                                for i in 0..count {
                                    match i % 4 {
                                        0 => {
                                            let timer = factory.timer(Duration::from_secs(1));
                                            root_guard.add(timer as Arc<RwLock<dyn Transient>>).await;
                                        },
                                        1 => {
                                            let future = factory.future::<i32>();
                                            root_guard.add(future as Arc<RwLock<dyn Transient>>).await;
                                        },
                                        2 => {
                                            let group = factory.group();
                                            root_guard.add(group as Arc<RwLock<dyn Transient>>).await;
                                        },
                                        3 => {
                                            let sequence = factory.sequence();
                                            root_guard.add(sequence as Arc<RwLock<dyn Transient>>).await;
                                        },
                                        _ => unreachable!(),
                                    }
                                }
                            }
                            
                            runtime
                        })
                    },
                    |runtime| async move {
                        let delta = Duration::from_millis(16);
                        black_box(runtime.run_frame(delta).await).unwrap();
                    },
                    criterion::BatchSize::SmallInput,
                )
            }
        );
    }
    
    group.finish();
}

fn bench_async_coordination(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("async_coordination");
    
    group.bench_function("future_creation_and_completion", |b| {
        b.to_async(&rt).iter(|| async {
            let factory = Factory::new();
            let future = factory.future::<i32>();
            
            {
                let mut future_guard = future.write().await;
                black_box(future_guard.set_value(42).await).unwrap();
            }
            
            {
                let future_guard = future.read().await;
                black_box(future_guard.is_ready());
            }
        })
    });
    
    for future_count in [5, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("multiple_futures_coordination", future_count),
            future_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let factory = Factory::new();
                    let barrier = factory.barrier();
                    
                    // Create futures and add as dependencies
                    {
                        let mut barrier_guard = barrier.write().await;
                        for i in 0..count {
                            let future = factory.future_with_value(i);
                            barrier_guard.add_dependency(future as Arc<RwLock<dyn Transient>>).await;
                        }
                    }
                    
                    // Check completion
                    {
                        let barrier_guard = barrier.read().await;
                        black_box(barrier_guard.all_completed());
                    }
                })
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_transient_id_creation,
    bench_time_frame_operations,
    bench_factory_creation,
    bench_kernel_operations,
    bench_flow_component_stepping,
    bench_sequence_operations,
    bench_barrier_operations,
    bench_runtime_performance,
    bench_async_coordination
);

criterion_main!(benches);