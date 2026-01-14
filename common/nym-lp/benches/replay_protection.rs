use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use nym_lp::replay::ReceivingKeyCounterValidator;
use parking_lot::Mutex;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::sync::Arc;

fn bench_sequential_counters(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_sequential");
    group.sample_size(1000);

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size));

        group.bench_with_input(
            BenchmarkId::new("sequential_counters", size),
            &size,
            |b, &size| {
                let validator = ReceivingKeyCounterValidator::default();
                let counters: Vec<u64> = (0..size).collect();

                b.iter(|| {
                    let mut validator = validator.clone();
                    for &counter in &counters {
                        let _ = black_box(validator.will_accept_branchless(counter));
                        let _ = black_box(validator.mark_did_receive_branchless(counter));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_out_of_order_counters(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_out_of_order");
    group.sample_size(1000);

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("out_of_order_counters", size),
            &size,
            |b, &size| {
                let validator = ReceivingKeyCounterValidator::default();

                // Create random counters within a valid window
                let mut rng = ChaCha8Rng::seed_from_u64(42);
                let counters: Vec<u64> = (0..size).map(|_| rng.gen_range(0..1024)).collect();

                b.iter(|| {
                    let mut validator = validator.clone();
                    for &counter in &counters {
                        let _ = black_box(validator.will_accept_branchless(counter));
                        let _ = black_box(validator.mark_did_receive_branchless(counter));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_thread_safety(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_thread_safety");
    group.sample_size(1000);

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size));

        group.bench_with_input(
            BenchmarkId::new("thread_safe_validator", size),
            &size,
            |b, &size| {
                let validator = Arc::new(Mutex::new(ReceivingKeyCounterValidator::default()));
                let counters: Vec<u64> = (0..size).collect();

                b.iter(|| {
                    for &counter in &counters {
                        let result = {
                            let guard = validator.lock();
                            black_box(guard.will_accept_branchless(counter))
                        };

                        if result.is_ok() {
                            let mut guard = validator.lock();
                            let _ = black_box(guard.mark_did_receive_branchless(counter));
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_window_sliding(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_window_sliding");
    group.sample_size(100);

    for window_size in [128, 512, 1024] {
        group.throughput(Throughput::Elements(window_size));

        group.bench_with_input(
            BenchmarkId::new("window_sliding", window_size),
            &window_size,
            |b, &window_size| {
                b.iter(|| {
                    let mut validator = ReceivingKeyCounterValidator::default();

                    // First fill the window with sequential packets
                    for i in 0..window_size {
                        let _ = black_box(validator.mark_did_receive_branchless(i));
                    }

                    // Then jump ahead to force window sliding
                    let _ = black_box(validator.mark_did_receive_branchless(window_size * 3));

                    // Try some packets in the new window
                    for i in (window_size * 2 + 1)..(window_size * 3) {
                        let _ = black_box(validator.will_accept_branchless(i));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark operations that would benefit from SIMD optimization
fn bench_core_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_core_operations");
    group.sample_size(1000);

    // Create validators with different states
    let empty_validator = ReceivingKeyCounterValidator::default();
    let mut half_full_validator = ReceivingKeyCounterValidator::default();
    let mut full_validator = ReceivingKeyCounterValidator::default();

    // Fill validators with different patterns
    for i in 0..512 {
        half_full_validator.mark_did_receive_branchless(i).unwrap();
    }

    for i in 0..1024 {
        full_validator.mark_did_receive_branchless(i).unwrap();
    }

    // Benchmark clearing operations
    group.bench_function("clear_empty_window", |b| {
        b.iter(|| {
            let mut validator = empty_validator.clone();
            // Force window sliding that will clear bitmap
            let _: () = validator.mark_did_receive_branchless(2000).unwrap();
            black_box(());
        })
    });

    group.bench_function("clear_half_full_window", |b| {
        b.iter(|| {
            let mut validator = half_full_validator.clone();
            // Force window sliding that will clear bitmap
            let _: () = validator.mark_did_receive_branchless(2000).unwrap();
            black_box(());
        })
    });

    group.bench_function("clear_full_window", |b| {
        b.iter(|| {
            let mut validator = full_validator.clone();
            // Force window sliding that will clear bitmap
            let _: () = validator.mark_did_receive_branchless(2000).unwrap();
            black_box(());
        })
    });

    group.finish();
}

/// Benchmark thread safety with different thread counts
fn bench_concurrency_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_concurrency_scaling");
    group.sample_size(50);

    for thread_count in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("mutex_threads", thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let validator = Arc::new(Mutex::new(ReceivingKeyCounterValidator::default()));
                    let mut handles = Vec::new();

                    for t in 0..thread_count {
                        let validator_clone = Arc::clone(&validator);
                        let handle = std::thread::spawn(move || {
                            let mut success_count = 0;
                            for i in 0..100 {
                                let counter = t * 1000 + i;
                                let mut guard = validator_clone.lock();
                                if guard.mark_did_receive_branchless(counter as u64).is_ok() {
                                    success_count += 1;
                                }
                            }
                            success_count
                        });
                        handles.push(handle);
                    }

                    let mut total = 0;
                    for handle in handles {
                        total += handle.join().unwrap();
                    }

                    black_box(total)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    replay_benches,
    bench_sequential_counters,
    bench_out_of_order_counters,
    bench_thread_safety,
    bench_window_sliding,
    bench_core_operations,
    bench_concurrency_scaling
);
criterion_main!(replay_benches);
