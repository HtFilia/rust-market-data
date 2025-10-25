use leptos::*;

use crate::ticks::types::HistoryPoint;

use super::dashboard::{SelectedSymbolSignal, TickStoreSignal};

const CHART_WIDTH: f64 = 620.0;
const CHART_HEIGHT: f64 = 260.0;

#[component]
pub fn HistoryChart() -> impl IntoView {
    let tick_store = use_context::<TickStoreSignal>().expect("tick store context missing");
    let selected_symbol =
        use_context::<SelectedSymbolSignal>().expect("selected symbol context missing");

    let history_state = create_memo(move |_| {
        selected_symbol.0.get().and_then(|symbol| {
            tick_store.0.with(|store| {
                store.history_for(&symbol).map(|history| {
                    (
                        symbol.clone(),
                        history.iter().cloned().collect::<Vec<HistoryPoint>>(),
                    )
                })
            })
        })
    });

    view! {
        <section class="history-chart">
            <h2>"Price History"</h2>
            <Show
                when=move || history_state.get().is_some_and(|(_, ref history)| history.len() >= 2)
                fallback=move || {
                    history_state.get().map(|(symbol, history)| {
                        if history.is_empty() {
                            view! { <p>"Waiting for live data for "{symbol.clone()}...</p> }
                        } else {
                            view! { <p>"Collecting more samples for "{symbol.clone()}...</p> }
                        }
                    }).unwrap_or_else(|| view! { <p>"Select a symbol to view its recent price action."</p> })
                }
            >
                {move || {
                    history_state.get().and_then(|(symbol, history)| {
                        compute_chart_geometry(&history, CHART_WIDTH, CHART_HEIGHT).map(|geometry| {
                            view! {
                                <div class="history-chart__content">
                                    <header class="history-chart__header">
                                        <strong>{symbol.clone()}</strong>
                                        <span>{format!("Latest: {:.4}", history.last().map(|point| point.price).unwrap_or_default())}</span>
                                    </header>
                                    <svg
                                        width=CHART_WIDTH
                                        height=CHART_HEIGHT
                                        viewBox=format!("0 0 {} {}", CHART_WIDTH, CHART_HEIGHT)
                                        class="history-chart__svg"
                                    >
                                        <defs>
                                            <linearGradient id="priceFill" x1="0" x2="0" y1="0" y2="1">
                                                <stop offset="0%" stop-color="#38bdf8" stop-opacity="0.35" />
                                                <stop offset="100%" stop-color="#38bdf8" stop-opacity="0.02" />
                                            </linearGradient>
                                        </defs>
                                        <polyline
                                            class="history-chart__line"
                                            points=geometry.points.clone()
                                        />
                                        <polygon
                                            class="history-chart__area"
                                            points=geometry.area_points.clone()
                                        />
                                    </svg>
                                    <footer class="history-chart__footer">
                                        <span>{format!("High: {:.4}", geometry.max_price)}</span>
                                        <span>{format!("Low: {:.4}", geometry.min_price)}</span>
                                    </footer>
                                </div>
                            }
                        })
                    })
                }}
            </Show>
        </section>
    }
}

#[derive(Debug, PartialEq)]
struct ChartGeometry {
    points: String,
    area_points: String,
    min_price: f64,
    max_price: f64,
}

fn compute_chart_geometry(
    history: &[HistoryPoint],
    width: f64,
    height: f64,
) -> Option<ChartGeometry> {
    if history.len() < 2 || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let min_price = history
        .iter()
        .map(|point| point.price)
        .fold(f64::INFINITY, f64::min);
    let max_price = history
        .iter()
        .map(|point| point.price)
        .fold(f64::NEG_INFINITY, f64::max);

    if !min_price.is_finite()
        || !max_price.is_finite()
        || (max_price - min_price).abs() < f64::EPSILON
    {
        return None;
    }

    let min_ts = history.first()?.timestamp_ms as f64;
    let max_ts = history.last()?.timestamp_ms as f64;
    let ts_span = (max_ts - min_ts).max(1.0);
    let price_span = (max_price - min_price).max(1e-9);

    let points_vec: Vec<String> = history
        .iter()
        .map(|point| {
            let x = ((point.timestamp_ms as f64 - min_ts) / ts_span) * width;
            let y = height - ((point.price - min_price) / price_span) * height;
            format!("{:.2},{:.2}", x, y)
        })
        .collect();
    let points = points_vec.join(" ");
    let area_points = format!("{} {:.2},{:.2} 0,{:.2}", points, width, height, height);

    Some(ChartGeometry {
        points,
        area_points,
        min_price,
        max_price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_chart_geometry_produces_points() {
        let history = vec![
            HistoryPoint {
                timestamp_ms: 0,
                price: 10.0,
            },
            HistoryPoint {
                timestamp_ms: 1,
                price: 11.0,
            },
            HistoryPoint {
                timestamp_ms: 2,
                price: 9.5,
            },
        ];

        let geometry = compute_chart_geometry(&history, 100.0, 50.0).expect("geometry");
        assert!(geometry.points.contains(','));
        assert!(geometry.max_price > geometry.min_price);
        assert!(geometry.area_points.contains("100.00,50.00"));
    }

    #[test]
    fn compute_chart_geometry_rejects_insufficient_data() {
        let history = vec![HistoryPoint {
            timestamp_ms: 0,
            price: 10.0,
        }];

        assert!(compute_chart_geometry(&history, 100.0, 50.0).is_none());
    }
}
