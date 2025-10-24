use std::time::Duration;

use rust_market_data::simulator::{self, SimulatorConfig};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn simulator_generates_ticks_without_socket() {
    let config = SimulatorConfig {
        tick_interval: Duration::from_millis(5),
        correlation_refresh: Duration::from_millis(50),
        enable_socket: false,
        max_ticks: None,
        ..SimulatorConfig::default()
    };

    let ticks = simulator::testkit::collect_ticks(config, 32)
        .await
        .expect("collect ticks");

    assert!(ticks.len() >= 32, "expected to capture at least 32 ticks");
    let mut last_ts = 0u128;
    for tick in ticks {
        assert!(!tick.symbol.is_empty(), "symbol should not be empty");
        assert!(
            tick.price.is_finite() && tick.price > 0.0,
            "price should be positive"
        );
        assert!(
            tick.timestamp_ms >= last_ts,
            "timestamps should be non-decreasing"
        );
        last_ts = tick.timestamp_ms;
    }
}
