use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rust_market_data::{
    logging,
    simulator::{self, SimulatorConfig},
};
use tokio::runtime::Runtime;

fn bench_tick_generation(c: &mut Criterion) {
    logging::set_silent(true);
    let rt = Runtime::new().expect("failed to create Tokio runtime");
    let batch_size: usize = 50_000;

    let mut group = c.benchmark_group("tick_generation");
    group.throughput(Throughput::Elements(batch_size as u64));

    group.bench_function("collect_ticks", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let config = SimulatorConfig {
                    enable_socket: false,
                    tick_interval: Duration::from_micros(1),
                    correlation_refresh: Duration::from_secs(3600),
                    max_ticks: Some(batch_size),
                    ..SimulatorConfig::default()
                };

                let elapsed = rt.block_on(async {
                    let start = Instant::now();
                    let ticks = simulator::testkit::collect_ticks(config, batch_size)
                        .await
                        .expect("collect ticks");
                    assert!(
                        ticks.len() >= batch_size,
                        "expected to receive at least {batch_size} ticks"
                    );
                    start.elapsed()
                });
                total += elapsed;
            }
            let avg_per_tick = total.as_secs_f64() / (batch_size as f64 * iters as f64);
            println!("Average time per tick: {:.3} ns", avg_per_tick * 1e9);
            total
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tick_generation);
criterion_main!(benches);
