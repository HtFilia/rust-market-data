use std::str::FromStr;

use leptos::event_target_value;
use leptos::{ev, *};

use crate::StreamStatus;

use super::dashboard::{ConnectionStatusSignal, Theme, ThemeSignal, TickStoreSignal};

const MOVERS_COUNT: usize = 3;

#[component]
pub fn SummaryPanel() -> impl IntoView {
    let tick_store = use_context::<TickStoreSignal>().expect("tick store context missing");
    let connection =
        use_context::<ConnectionStatusSignal>().expect("connection status context missing");
    let theme_signal = use_context::<ThemeSignal>().expect("theme signal context missing");

    let summary = create_memo(move |_| {
        tick_store.0.with(|store| {
            let total = store.latest().len();
            let (advancers, decliners) = store.movers(MOVERS_COUNT);
            (total, advancers, decliners)
        })
    });

    let theme_select_value = move || theme_signal.0.get().as_str().to_string();

    let on_theme_change = {
        let theme = theme_signal;
        move |ev: ev::Event| {
            let value = event_target_value(&ev);
            if let Ok(theme_value) = Theme::from_str(&value) {
                theme.0.set(theme_value);
            }
        }
    };

    let status_badge = move || match connection.0.get() {
        StreamStatus::Connecting => ("status--connecting", "Connecting"),
        StreamStatus::Connected => ("status--connected", "Live"),
        StreamStatus::Reconnecting { .. } => ("status--reconnecting", "Reconnecting"),
        StreamStatus::Failed => ("status--failed", "Disconnected"),
        StreamStatus::Idle => ("status--idle", "Idle"),
    };

    view! {
        <section class="summary-panel">
            <header class="summary-panel__header">
                <div class="summary-panel__status">
                    {move || {
                        let (class, label) = status_badge();
                        view! { <span class=format!("status-badge {class}")>{label}</span> }
                    }}
                    <span class="summary-panel__total">
                        {move || {
                            let (total, _, _) = summary.get();
                            format!("{} Symbols", total)
                        }}
                    </span>
                </div>
                <label class="summary-panel__theme">
                    <span>"Theme"</span>
                    <select class="theme-select" on:change=on_theme_change prop:value=theme_select_value>
                        <For
                            each=move || Theme::ALL.into_iter()
                            key=|theme| theme.as_str()
                            children=move |theme: Theme| {
                                view! {
                                    <option value=theme.as_str()>{theme.label()}</option>
                                }
                            }
                        />
                    </select>
                </label>
            </header>
            <div class="summary-panel__body">
                <div>
                    <h3>"Top Advancers"</h3>
                    <SummaryList items=move || summary.get().1.clone() empty_label="Waiting for data" />
                </div>
                <div>
                    <h3>"Top Decliners"</h3>
                    <SummaryList items=move || summary.get().2.clone() empty_label="Waiting for data" />
                </div>
            </div>
        </section>
    }
}

#[component]
fn SummaryList<F>(items: F, empty_label: &'static str) -> impl IntoView
where
    F: Fn() -> Vec<(String, f64)> + 'static,
{
    let data = create_memo(move |_| items());

    view! {
        <ul class="summary-list">
            <Show
                when=move || data.with(|items| !items.is_empty())
                fallback=move || view! { <li class="summary-list__empty">{empty_label}</li> }
            >
                <For
                    each=move || data.get()
                    key=|(symbol, _)| symbol.clone()
                    children=move |(symbol, change): (String, f64)| {
                        let sign = if change >= 0.0 { '+' } else { '-' };
                        let magnitude = change.abs();
                        let positive = change >= 0.0;
                        view! {
                            <li>
                                <span class="summary-list__symbol">{symbol.clone()}</span>
                                <span class="summary-list__change" class:positive=positive class:negative=!positive>
                                    {format!("{}{:.1}%", sign, magnitude)}
                                </span>
                            </li>
                        }
                    }
                />
            </Show>
        </ul>
    }
}
