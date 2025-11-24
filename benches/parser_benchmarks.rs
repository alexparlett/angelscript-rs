//! Performance benchmarks for the AngelScript parser.
//!
//! This benchmark suite measures parsing performance across different workloads:
//! - Size-based: Tiny to stress-test sized files (5 to 5000 lines)
//! - Feature-specific: Functions, classes, expressions, etc.
//! - Real-world: Game logic, utilities, data structures
//! - Parsing modes: Strict vs lenient parsing

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use angelscript::parse_lenient;
use std::hint::black_box;

/// Benchmark parsing performance across different file sizes
fn size_based_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_sizes");

    // Tiny: 5 lines - baseline parsing overhead
    let hello_world = include_str!("../test_scripts/hello_world.as");
    group.throughput(Throughput::Bytes(hello_world.len() as u64));
    group.bench_function("tiny_5_lines", |b| {
        b.iter(|| parse_lenient(black_box(hello_world)));
    });

    // Small: ~60 lines - typical small script
    let functions = include_str!("../test_scripts/functions.as");
    group.throughput(Throughput::Bytes(functions.len() as u64));
    group.bench_function("small_60_lines", |b| {
        b.iter(|| parse_lenient(black_box(functions)));
    });

    // Medium: ~130 lines - typical medium script
    let expressions = include_str!("../test_scripts/expressions.as");
    group.throughput(Throughput::Bytes(expressions.len() as u64));
    group.bench_function("medium_130_lines", |b| {
        b.iter(|| parse_lenient(black_box(expressions)));
    });

    // Large: ~266 lines - large script
    let data_structures = include_str!("../test_scripts/data_structures.as");
    group.throughput(Throughput::Bytes(data_structures.len() as u64));
    group.bench_function("large_266_lines", |b| {
        b.iter(|| parse_lenient(black_box(data_structures)));
    });

    // XLarge: ~500 lines - very large script
    let large_500 = include_str!("../test_scripts/performance/large_500.as");
    group.throughput(Throughput::Bytes(large_500.len() as u64));
    group.bench_function("xlarge_500_lines", |b| {
        b.iter(|| parse_lenient(black_box(large_500)));
    });

    // XXLarge: ~1000 lines - extremely large script
    let xlarge_1000 = include_str!("../test_scripts/performance/xlarge_1000.as");
    group.throughput(Throughput::Bytes(xlarge_1000.len() as u64));
    group.bench_function("xxlarge_1000_lines", |b| {
        b.iter(|| parse_lenient(black_box(xlarge_1000)));
    });

    // Stress: ~5000 lines - stress test
    let xxlarge_5000 = include_str!("../test_scripts/performance/xxlarge_5000.as");
    group.throughput(Throughput::Bytes(xxlarge_5000.len() as u64));
    group.bench_function("stress_5000_lines", |b| {
        b.iter(|| parse_lenient(black_box(xxlarge_5000)));
    });

    group.finish();
}

/// Benchmark parsing performance for specific language features
fn feature_specific_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("features");

    // Many functions - throughput test
    let many_functions = include_str!("../test_scripts/many_functions.as");
    group.bench_function("many_functions", |b| {
        b.iter(|| {
            parse_lenient(black_box(many_functions))
        });
    });

    // Large single function - deep nesting
    let large_function = include_str!("../test_scripts/large_function.as");
    group.bench_function("large_function", |b| {
        b.iter(|| {
            parse_lenient(black_box(large_function))
        });
    });

    // Classes - OOP parsing
    let class_basic = include_str!("../test_scripts/class_basic.as");
    group.bench_function("classes", |b| {
        b.iter(|| {
            parse_lenient(black_box(class_basic))
        });
    });

    // Inheritance - class hierarchies
    let inheritance = include_str!("../test_scripts/inheritance.as");
    group.bench_function("inheritance", |b| {
        b.iter(|| {
            parse_lenient(black_box(inheritance))
        });
    });

    // Interfaces
    let interface = include_str!("../test_scripts/interface.as");
    group.bench_function("interfaces", |b| {
        b.iter(|| {
            parse_lenient(black_box(interface))
        });
    });

    // Operators - expression parsing
    let operators = include_str!("../test_scripts/operators.as");
    group.bench_function("operators", |b| {
        b.iter(|| {
            parse_lenient(black_box(operators))
        });
    });

    // Complex expressions
    let expressions = include_str!("../test_scripts/expressions.as");
    group.bench_function("expressions", |b| {
        b.iter(|| {
            parse_lenient(black_box(expressions))
        });
    });

    // Control flow
    let control_flow = include_str!("../test_scripts/control_flow.as");
    group.bench_function("control_flow", |b| {
        b.iter(|| {
            parse_lenient(black_box(control_flow))
        });
    });

    // Templates
    let templates = include_str!("../test_scripts/templates.as");
    group.bench_function("templates", |b| {
        b.iter(|| {
            parse_lenient(black_box(templates))
        });
    });

    // Virtual properties
    let properties = include_str!("../test_scripts/properties.as");
    group.bench_function("properties", |b| {
        b.iter(|| {
            parse_lenient(black_box(properties))
        });
    });

    // Nested structures
    let nested = include_str!("../test_scripts/nested.as");
    group.bench_function("nested", |b| {
        b.iter(|| {
            parse_lenient(black_box(nested))
        });
    });

    group.finish();
}

/// Benchmark real-world use cases
fn real_world_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world");

    // Game logic - typical game scripting
    let game_logic = include_str!("../test_scripts/game_logic.as");
    group.throughput(Throughput::Bytes(game_logic.len() as u64));
    group.bench_function("game_logic", |b| {
        b.iter(|| {
            parse_lenient(black_box(game_logic))
        });
    });

    // Utilities - utility functions
    let utilities = include_str!("../test_scripts/utilities.as");
    group.throughput(Throughput::Bytes(utilities.len() as u64));
    group.bench_function("utilities", |b| {
        b.iter(|| {
            parse_lenient(black_box(utilities))
        });
    });

    // Data structures - data structure implementations
    let data_structures = include_str!("../test_scripts/data_structures.as");
    group.throughput(Throughput::Bytes(data_structures.len() as u64));
    group.bench_function("data_structures", |b| {
        b.iter(|| {
            parse_lenient(black_box(data_structures))
        });
    });

    group.finish();
}

/// Benchmark strict vs lenient parsing modes
fn parsing_mode_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_modes");

    let game_logic = include_str!("../test_scripts/game_logic.as");

    // Strict parsing
    group.bench_function("strict_mode", |b| {
        b.iter(|| {
            parse_lenient(black_box(game_logic))
        });
    });

    // Lenient parsing (always returns AST even with errors)
    group.bench_function("lenient_mode", |b| {
        b.iter(|| {
            parse_lenient(black_box(game_logic))
            // In lenient mode, we'd still get partial AST
            // For now, same as strict since we don't have lenient API yet
        });
    });

    group.finish();
}

/// Benchmark parsing with different complexity characteristics
fn complexity_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("complexity");

    // Wide: Many top-level items (many_functions.as has 67 functions)
    let many_functions = include_str!("../test_scripts/many_functions.as");
    group.bench_function("wide_many_items", |b| {
        b.iter(|| {
            parse_lenient(black_box(many_functions))
        });
    });

    // Deep: Deeply nested structures
    let nested = include_str!("../test_scripts/nested.as");
    group.bench_function("deep_nesting", |b| {
        b.iter(|| {
            parse_lenient(black_box(nested))
        });
    });

    // Complex: Complex expressions and control flow
    let large_function = include_str!("../test_scripts/large_function.as");
    group.bench_function("complex_logic", |b| {
        b.iter(|| {
            parse_lenient(black_box(large_function))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    size_based_benchmarks,
    feature_specific_benchmarks,
    real_world_benchmarks,
    parsing_mode_comparison,
    complexity_benchmarks
);

criterion_main!(benches);
