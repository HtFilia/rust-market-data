use leptos::*;

use super::dashboard::{SelectedSymbolSignal, TickStoreSignal};

/// Placeholder chart container that will visualize the selected symbol history.
#[component]
pub fn HistoryChart() -> impl IntoView {
    let tick_store = use_context::<TickStoreSignal>().expect("tick store context missing");
    let selected_symbol =
        use_context::<SelectedSymbolSignal>().expect("selected symbol context missing");

    let history_state = create_memo(move |_| {
        selected_symbol.0.get().and_then(|symbol| {
            tick_store.0.with(|store| {
                store
                    .history_for(&symbol)
                    .map(|history| (symbol, history.len()))
            })
        })
    });

    view! {
        <section class="history-chart">
            <h2>"Price History"</h2>
            <Show
                when=move || history_state.get().is_some()
                fallback=|| view! { <p>"Select a symbol to view its recent price action."</p> }
            >
                {move || {
                    history_state.get().map(|(symbol, len)| {
                        view! {
                            <div class="history-chart__placeholder">
                                <strong>{symbol.clone()}</strong>
                                <p>{format!("{} samples ready for charting", len)}</p>
                            </div>
                        }
                    })
                }}
            </Show>
        </section>
    }
}
