// Simple benchmark demonstration of the testing framework
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct BenchmarkResult {
    name: String,
    duration: Duration,
    operations: u64,
    ops_per_sec: u64,
}

impl BenchmarkResult {
    fn new(name: String, duration: Duration, operations: u64) -> Self {
        let ops_per_sec = if duration.as_secs() == 0 {
            operations * 1_000_000_000 / duration.as_nanos() as u64
        } else {
            operations / duration.as_secs()
        };
        
        Self {
            name,
            duration,
            operations,
            ops_per_sec,
        }
    }
}

fn benchmark<F>(name: &str, operations: u64, f: F) -> BenchmarkResult
where
    F: FnOnce(),
{
    let start = Instant::now();
    f();
    let duration = start.elapsed();
    
    BenchmarkResult::new(name.to_string(), duration, operations)
}

fn main() {
    println!("🚀 RustFlow Performance Benchmark Suite");
    println!("=======================================");
    
    let mut results = Vec::new();
    
    // Benchmark 1: HashMap operations
    results.push(benchmark("HashMap Insert/Lookup", 100_000, || {
        let mut map = HashMap::new();
        for i in 0..100_000 {
            map.insert(i, i * 2);
        }
        for i in 0..100_000 {
            let _ = map.get(&i);
        }
    }));
    
    // Benchmark 2: Vector operations
    results.push(benchmark("Vector Push/Pop", 100_000, || {
        let mut vec = Vec::new();
        for i in 0..100_000 {
            vec.push(i);
        }
        for _ in 0..100_000 {
            vec.pop();
        }
    }));
    
    // Benchmark 3: String operations
    results.push(benchmark("String Concatenation", 10_000, || {
        let mut s = String::new();
        for i in 0..10_000 {
            s.push_str(&format!("item_{}", i));
        }
    }));
    
    // Benchmark 4: Math operations
    results.push(benchmark("Floating Point Math", 1_000_000, || {
        let mut result = 0.0;
        for i in 0..1_000_000 {
            result += (i as f64).sqrt() * (i as f64).sin();
        }
        // Use result to prevent optimization
        std::hint::black_box(result);
    }));
    
    // Benchmark 5: Memory allocation
    results.push(benchmark("Memory Allocation", 10_000, || {
        let mut vecs = Vec::new();
        for _ in 0..10_000 {
            vecs.push(vec![0u8; 1024]);
        }
        std::hint::black_box(vecs);
    }));
    
    // Display results
    println!("\n📊 Benchmark Results:");
    println!("{:<25} {:>12} {:>15} {:>15}", "Benchmark", "Duration", "Operations", "Ops/sec");
    println!("{}", "-".repeat(70));
    
    for result in &results {
        println!("{:<25} {:>10}ms {:>15} {:>15}",
                result.name,
                result.duration.as_millis(),
                format_number(result.operations),
                format_number(result.ops_per_sec));
    }
    
    println!("\n🎯 Performance Analysis:");
    
    // Find fastest and slowest
    let fastest = results.iter().max_by_key(|r| r.ops_per_sec).unwrap();
    let slowest = results.iter().min_by_key(|r| r.ops_per_sec).unwrap();
    
    println!("  ⚡ Fastest: {} ({} ops/sec)", fastest.name, format_number(fastest.ops_per_sec));
    println!("  🐌 Slowest: {} ({} ops/sec)", slowest.name, format_number(slowest.ops_per_sec));
    
    let speedup = fastest.ops_per_sec / slowest.ops_per_sec;
    println!("  📈 Speed ratio: {}:1", speedup);
    
    println!("\n✅ All benchmarks completed successfully!");
    println!("   This demonstrates the comprehensive testing framework");
    println!("   that would be used with the full RustFlow implementation.");
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}