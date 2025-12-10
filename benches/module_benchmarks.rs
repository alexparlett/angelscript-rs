//! Performance benchmarks for the AngelScript unit build pipeline.
//!
//! This benchmark suite measures build performance across different workloads:
//! - Size-based: Tiny to stress-test sized files (5 to 5000 lines)
//! - Feature-specific: Functions, classes, expressions, etc.
//! - Real-world: Game logic, utilities, data structures
//!
//! ## Profiling with Puffin
//!
//! Run with the `profile-with-puffin` feature to collect detailed phase timings:
//!
//! ```bash
//! cargo bench --features profile-with-puffin -- --profile-time 5
//! ```
//!
//! After running, check the generated puffin report for parsing vs compilation breakdown.
//!
//! ## Quick Phase Timing Test
//!
//! For a quick breakdown of parsing vs compilation time on the stress test:
//!
//! ```bash
//! cargo bench --features profile-with-puffin -- "stress_5000" --test
//! ```

#![allow(clippy::collapsible_if)]

use angelscript::Context;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;

#[cfg(feature = "profile-with-puffin")]
use std::collections::HashMap;

#[cfg(feature = "profile-with-puffin")]
static FRAME_VIEW: std::sync::OnceLock<puffin::GlobalFrameView> = std::sync::OnceLock::new();

/// Initialize puffin profiler.
#[cfg(feature = "profile-with-puffin")]
fn setup_profiler() {
    puffin::set_scopes_on(true);
    // Create the global frame view which registers itself as a sink
    FRAME_VIEW.get_or_init(puffin::GlobalFrameView::default);
}

#[cfg(not(feature = "profile-with-puffin"))]
fn setup_profiler() {}

/// Call at the end of each benchmark iteration to flush profiling data.
#[cfg(feature = "profile-with-puffin")]
fn end_profiling_frame() {
    puffin::GlobalProfiler::lock().new_frame();
}

#[cfg(not(feature = "profile-with-puffin"))]
fn end_profiling_frame() {}

/// Recursively collect all scopes (including nested ones)
#[cfg(feature = "profile-with-puffin")]
fn collect_scopes_recursive(
    stream: &puffin::Stream,
    scope: &puffin::Scope,
    scope_collection: &puffin::ScopeCollection,
    scope_timings: &mut HashMap<String, i64>,
) {
    use puffin::Reader;

    // Record this scope's timing
    if let Some(details) = scope_collection.fetch_by_id(&scope.id) {
        let name = details.name().to_string();
        *scope_timings.entry(name).or_insert(0) += scope.record.duration_ns;
    }

    // Read children if any
    if scope.child_begin_position < scope.child_end_position {
        if let Ok(reader) = Reader::with_offset(stream, scope.child_begin_position) {
            if let Ok(children) = reader.read_top_scopes() {
                for child in children {
                    collect_scopes_recursive(stream, &child, scope_collection, scope_timings);
                }
            }
        }
    }
}

/// Print accumulated profiling statistics for all scopes.
#[cfg(feature = "profile-with-puffin")]
fn print_profiling_stats() {
    use puffin::Reader;

    let Some(frame_view) = FRAME_VIEW.get() else {
        println!("Profiler not initialized");
        return;
    };

    let view = frame_view.lock();
    let scope_collection = view.scope_collection();

    // Accumulate timings across all frames
    let mut scope_timings: HashMap<String, i64> = HashMap::new();
    let mut frame_count = 0i64;

    for frame in view.recent_frames() {
        frame_count += 1;
        let unpacked = match frame.unpacked() {
            Ok(u) => u,
            Err(_) => continue,
        };
        for (_thread_info, stream_info) in unpacked.thread_streams.iter() {
            let reader = Reader::from_start(&stream_info.stream);
            if let Ok(scopes) = reader.read_top_scopes() {
                for scope in scopes {
                    collect_scopes_recursive(
                        &stream_info.stream,
                        &scope,
                        scope_collection,
                        &mut scope_timings,
                    );
                }
            }
        }
    }

    println!("\n=== Profiling Summary ({} frames) ===", frame_count);

    if scope_timings.is_empty() {
        println!("  No scopes recorded.");
        println!("  Make sure profiling::scope! calls exist in unit.rs");
    } else {
        // Sort by total time descending
        let mut entries: Vec<_> = scope_timings.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        let total_ns: i64 = entries.iter().map(|(_, ns)| **ns).sum();

        for (name, ns) in &entries {
            let ns = **ns;
            let avg_ns = if frame_count > 0 {
                ns / frame_count
            } else {
                ns
            };
            let pct = if total_ns > 0 {
                ns as f64 / total_ns as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "  {:30} {:>10.2?} avg ({:>5.1}%)",
                name,
                std::time::Duration::from_nanos(avg_ns as u64),
                pct
            );
        }

        if frame_count > 0 {
            let avg_total = total_ns / frame_count;
            println!(
                "  {:30} {:>10.2?}",
                "TOTAL",
                std::time::Duration::from_nanos(avg_total as u64)
            );
        }
    }
    println!("=====================================\n");
}

#[cfg(not(feature = "profile-with-puffin"))]
fn print_profiling_stats() {}

/// Benchmark build performance across different file sizes.
fn size_based_benchmarks(c: &mut Criterion) {
    setup_profiler();

    // Create context once for all benchmarks in this group
    let ctx = Context::with_default_modules().unwrap();
    let ctx = Arc::new(ctx);

    let mut group = c.benchmark_group("unit/file_sizes");

    // Tiny: 5 lines - baseline build overhead
    let hello_world = include_str!("../test_scripts/hello_world.as");
    group.throughput(Throughput::Bytes(hello_world.len() as u64));
    group.bench_function("tiny_5_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(hello_world)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Small: ~60 lines - typical small script
    let functions = include_str!("../test_scripts/functions.as");
    group.throughput(Throughput::Bytes(functions.len() as u64));
    group.bench_function("small_60_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(functions)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Medium: ~130 lines - typical medium script
    let expressions = include_str!("../test_scripts/expressions.as");
    group.throughput(Throughput::Bytes(expressions.len() as u64));
    group.bench_function("medium_130_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(expressions)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Large: ~266 lines - large script
    let data_structures = include_str!("../test_scripts/data_structures.as");
    group.throughput(Throughput::Bytes(data_structures.len() as u64));
    group.bench_function("large_266_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(data_structures))
                .unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // XLarge: ~500 lines - very large script
    let large_500 = include_str!("../test_scripts/performance/large_500.as");
    group.throughput(Throughput::Bytes(large_500.len() as u64));
    group.bench_function("xlarge_500_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(large_500)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // XXLarge: ~1000 lines - extremely large script
    let xlarge_1000 = include_str!("../test_scripts/performance/xlarge_1000.as");
    group.throughput(Throughput::Bytes(xlarge_1000.len() as u64));
    group.bench_function("xxlarge_1000_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(xlarge_1000)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Stress: ~5000 lines - stress test
    let xxlarge_5000 = include_str!("../test_scripts/performance/xxlarge_5000.as");
    group.throughput(Throughput::Bytes(xxlarge_5000.len() as u64));
    group.bench_function("stress_5000_lines", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(xxlarge_5000)).unwrap();
            unit.build().unwrap();
            end_profiling_frame();
            black_box(unit.function_count())
        });
    });

    group.finish();

    // Print profiling summary
    print_profiling_stats();
}

/// Benchmark build performance for specific language features.
fn feature_specific_benchmarks(c: &mut Criterion) {
    // Create context once for all benchmarks in this group
    let ctx = Context::with_default_modules().unwrap();
    let ctx = Arc::new(ctx);

    let mut group = c.benchmark_group("unit/features");

    // Many functions - throughput test
    let many_functions = include_str!("../test_scripts/many_functions.as");
    group.bench_function("many_functions", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(many_functions))
                .unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Large single function - deep nesting
    let large_function = include_str!("../test_scripts/large_function.as");
    group.bench_function("large_function", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(large_function))
                .unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Classes - OOP parsing
    let class_basic = include_str!("../test_scripts/class_basic.as");
    group.bench_function("classes", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(class_basic)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Inheritance - class hierarchies
    let inheritance = include_str!("../test_scripts/inheritance.as");
    group.bench_function("inheritance", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(inheritance)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Interfaces
    let interface = include_str!("../test_scripts/interface.as");
    group.bench_function("interfaces", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(interface)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Operators - expression parsing
    let operators = include_str!("../test_scripts/operators.as");
    group.bench_function("operators", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(operators)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Complex expressions
    let expressions = include_str!("../test_scripts/expressions.as");
    group.bench_function("expressions", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(expressions)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Control flow
    let control_flow = include_str!("../test_scripts/control_flow.as");
    group.bench_function("control_flow", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(control_flow)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Templates
    let templates = include_str!("../test_scripts/templates.as");
    group.bench_function("templates", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(templates)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Virtual properties
    let properties = include_str!("../test_scripts/properties.as");
    group.bench_function("properties", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(properties)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Nested structures
    let nested = include_str!("../test_scripts/nested.as");
    group.bench_function("nested", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(nested)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    group.finish();
}

/// Benchmark real-world use cases.
fn real_world_benchmarks(c: &mut Criterion) {
    // Create context once for all benchmarks in this group
    let ctx = Context::with_default_modules().unwrap();
    let ctx = Arc::new(ctx);

    let mut group = c.benchmark_group("unit/real_world");

    // Game logic - typical game scripting
    let game_logic = include_str!("../test_scripts/game_logic.as");
    group.throughput(Throughput::Bytes(game_logic.len() as u64));
    group.bench_function("game_logic", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(game_logic)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Utilities - utility functions
    let utilities = include_str!("../test_scripts/utilities.as");
    group.throughput(Throughput::Bytes(utilities.len() as u64));
    group.bench_function("utilities", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(utilities)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Data structures - data structure implementations
    let data_structures = include_str!("../test_scripts/data_structures.as");
    group.throughput(Throughput::Bytes(data_structures.len() as u64));
    group.bench_function("data_structures", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(data_structures))
                .unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    group.finish();
}

/// Benchmark build with different complexity characteristics.
fn complexity_benchmarks(c: &mut Criterion) {
    // Create context once for all benchmarks in this group
    let ctx = Context::with_default_modules().unwrap();
    let ctx = Arc::new(ctx);

    let mut group = c.benchmark_group("unit/complexity");

    // Wide: Many top-level items (many_functions.as has 67 functions)
    let many_functions = include_str!("../test_scripts/many_functions.as");
    group.bench_function("wide_many_items", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(many_functions))
                .unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Deep: Deeply nested structures
    let nested = include_str!("../test_scripts/nested.as");
    group.bench_function("deep_nesting", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(nested)).unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    // Complex: Complex expressions and control flow
    let large_function = include_str!("../test_scripts/large_function.as");
    group.bench_function("complex_logic", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit().unwrap();
            unit.add_source("test.as", black_box(large_function))
                .unwrap();
            unit.build().unwrap();
            black_box(unit.function_count())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    size_based_benchmarks,
    feature_specific_benchmarks,
    real_world_benchmarks,
    complexity_benchmarks
);

criterion_main!(benches);
