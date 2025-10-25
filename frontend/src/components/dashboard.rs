use std::collections::HashSet;

use leptos::*;

use crate::{
    TickStore,
    ticks::types::{Region, Sector, Tick},
};

#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use crate::spawn_tick_stream;

use super::filters::FiltersPanel;
use super::{history_chart::HistoryChart, tick_table::TickTable};

#[derive(Clone, Copy)]
pub struct TickStoreSignal(pub RwSignal<TickStore>);

#[derive(Clone, Copy)]
pub struct SelectedSymbolSignal(pub RwSignal<Option<String>>);

#[derive(Clone)]
pub struct FilterState {
    pub regions: RwSignal<HashSet<Region>>,
    pub sectors: RwSignal<HashSet<Sector>>,
}

/// Top-level dashboard wrapper providing shared application state via context.
#[component]
pub fn Dashboard() -> impl IntoView {
    let tick_store = create_rw_signal(TickStore::new(2_048));
    seed_demo_data(&tick_store);

    #[cfg(target_arch = "wasm32")]
    {
        let store_for_ws = tick_store;
        leptos::create_effect(move |_| init_live_updates(store_for_ws));
    }

    let selected_symbol = create_rw_signal(None::<String>);
    let selected_regions = create_rw_signal(HashSet::<Region>::new());
    let selected_sectors = create_rw_signal(HashSet::<Sector>::new());

    provide_context(TickStoreSignal(tick_store));
    provide_context(SelectedSymbolSignal(selected_symbol));
    provide_context(FilterState {
        regions: selected_regions,
        sectors: selected_sectors,
    });

    view! {
        <div class="dashboard">
            <header class="dashboard__header">
                <h1>"Rust Market Dashboard"</h1>
                <p>"Live view of the last traded price for each symbol."</p>
            </header>
            <section class="dashboard__body">
                <FiltersPanel />
                <TickTable />
                <HistoryChart />
            </section>
        </div>
    }
}

fn seed_demo_data(tick_store: &RwSignal<TickStore>) {
    let seed_ticks = [
        Tick {
            symbol: "NA_TECH007".into(),
            price: 134.2875,
            timestamp_ms: 1_716_400_005_123,
            region: Region::NorthAmerica,
            sector: Sector::Technology,
        },
        Tick {
            symbol: "EU_IND002".into(),
            price: 98.4401,
            timestamp_ms: 1_716_400_005_456,
            region: Region::Europe,
            sector: Sector::Industrials,
        },
        Tick {
            symbol: "AP_HEAL009".into(),
            price: 154.9983,
            timestamp_ms: 1_716_400_005_789,
            region: Region::AsiaPacific,
            sector: Sector::Healthcare,
        },
        Tick {
            symbol: "NA_TECH007".into(),
            price: 134.7864,
            timestamp_ms: 1_716_400_005_999,
            region: Region::NorthAmerica,
            sector: Sector::Technology,
        },
    ];

    tick_store.update(|store| {
        for tick in seed_ticks {
            store.ingest(tick);
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn init_live_updates(tick_store: RwSignal<TickStore>) {
    tick_store.update(|store| store.clear());

    let store_for_cb = tick_store;
    let on_tick = Rc::new(move |ticks: Vec<Tick>| {
        store_for_cb.update(|store| store.ingest_batch(ticks));
    });

    let url = resolve_gateway_url();
    if let Err(err) = spawn_tick_stream(&url, on_tick) {
        log::error!("failed to open websocket stream {url}: {err:?}");
    } else {
        log::info!("connected to market data gateway at {url}");
    }
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
