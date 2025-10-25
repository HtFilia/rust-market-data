use std::collections::HashSet;

use leptos::*;

use crate::{
    StreamStatus, TickStore,
    ticks::types::{Region, Sector, Tick},
};

#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use crate::connect_with_retry;

use super::{
    filters::FiltersPanel, history_chart::HistoryChart, summary::SummaryPanel,
    tick_table::TickTable,
};

#[derive(Clone, Copy)]
pub struct TickStoreSignal(pub RwSignal<TickStore>);

#[derive(Clone, Copy)]
pub struct SelectedSymbolSignal(pub RwSignal<Option<String>>);

#[derive(Clone)]
pub struct FilterState {
    pub regions: RwSignal<HashSet<Region>>,
    pub sectors: RwSignal<HashSet<Sector>>,
}

#[derive(Clone, Copy)]
pub struct ConnectionStatusSignal(pub RwSignal<StreamStatus>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Sepia,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Sepia];

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Sepia => "sepia",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Sepia => "Sepia",
        }
    }
}

impl std::str::FromStr for Theme {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(Theme::Dark),
            "light" => Ok(Theme::Light),
            "sepia" => Ok(Theme::Sepia),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ThemeSignal(pub RwSignal<Theme>);

/// Top-level dashboard wrapper providing shared application state via context.
#[component]
pub fn Dashboard() -> impl IntoView {
    let tick_store = create_rw_signal(TickStore::new(2_048));
    seed_demo_data(&tick_store);

    let selected_symbol = create_rw_signal(None::<String>);
    let selected_regions = create_rw_signal(HashSet::<Region>::new());
    let selected_sectors = create_rw_signal(HashSet::<Sector>::new());
    let connection_status = create_rw_signal(StreamStatus::Idle);
    let theme = create_rw_signal(Theme::Dark);

    #[cfg(target_arch = "wasm32")]
    {
        let store_for_ws = tick_store;
        let status_for_ws = connection_status;
        leptos::create_effect(move |_| init_live_updates(store_for_ws, status_for_ws));

        let theme_signal = theme;
        leptos::create_effect(move |_| {
            let theme = theme_signal.get();
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                if let Some(element) = document.document_element() {
                    let _ = element.set_attribute("data-theme", theme.as_str());
                }
            }
        });
    }

    provide_context(TickStoreSignal(tick_store));
    provide_context(SelectedSymbolSignal(selected_symbol));
    provide_context(FilterState {
        regions: selected_regions,
        sectors: selected_sectors,
    });
    provide_context(ConnectionStatusSignal(connection_status));
    provide_context(ThemeSignal(theme));

    view! {
        <div class="dashboard">
            <header class="dashboard__header">
                <h1>"Rust Market Dashboard"</h1>
                <p>"Live view of the last traded price for each symbol."</p>
            </header>
            <section class="dashboard__body">
                <div class="dashboard__main">
                    <SummaryPanel />
                    <TickTable />
                </div>
                <aside class="dashboard__sidebar">
                    <FiltersPanel />
                    <HistoryChart />
                </aside>
            </section>
        </div>
    }
}

fn seed_demo_data(tick_store: &RwSignal<TickStore>) {
    let seed_ticks = [
        Tick {
            symbol: "NATECH007".into(),
            price: 134.2875,
            timestamp_ms: 1_716_400_005_123,
            region: Region::NorthAmerica,
            sector: Sector::Technology,
        },
        Tick {
            symbol: "EUIND002".into(),
            price: 98.4401,
            timestamp_ms: 1_716_400_005_456,
            region: Region::Europe,
            sector: Sector::Industrials,
        },
        Tick {
            symbol: "APHLT009".into(),
            price: 154.9983,
            timestamp_ms: 1_716_400_005_789,
            region: Region::AsiaPacific,
            sector: Sector::Healthcare,
        },
        Tick {
            symbol: "SAENG001".into(),
            price: 134.7864,
            timestamp_ms: 1_716_400_005_999,
            region: Region::SouthAmerica,
            sector: Sector::Energy,
        },
    ];

    tick_store.update(|store| {
        for tick in seed_ticks {
            store.ingest(tick);
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn init_live_updates(tick_store: RwSignal<TickStore>, status: RwSignal<StreamStatus>) {
    let store_for_cb = tick_store;
    let on_tick = Rc::new(move |ticks: Vec<Tick>| {
        store_for_cb.update(|store| store.ingest_batch(ticks));
    });

    let status_for_cb = status;
    let on_status = Rc::new(move |state: StreamStatus| {
        status_for_cb.set(state);
    });

    let url = resolve_gateway_url();
    connect_with_retry(url, on_tick, on_status);
}

#[cfg(target_arch = "wasm32")]
fn resolve_gateway_url() -> String {
    let fallback = "127.0.0.1".to_string();
    let host = web_sys::window()
        .and_then(|window| window.location().hostname().ok())
        .filter(|hostname| !hostname.is_empty())
        .unwrap_or(fallback);

    format!("ws://{host}:9001/ws")
}
