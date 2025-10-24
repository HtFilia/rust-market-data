use leptos::*;

use crate::{
    TickStore,
    ticks::types::{Region, Sector, Tick},
};

use super::{history_chart::HistoryChart, tick_table::TickTable};

#[derive(Clone, Copy)]
pub struct TickStoreSignal(pub RwSignal<TickStore>);

#[derive(Clone, Copy)]
pub struct SelectedSymbolSignal(pub RwSignal<Option<String>>);

/// Top-level dashboard wrapper providing shared application state via context.
#[component]
pub fn Dashboard() -> impl IntoView {
    let tick_store = create_rw_signal(TickStore::new(2_048));
    seed_demo_data(&tick_store);

    let selected_symbol = create_rw_signal(None::<String>);

    provide_context(TickStoreSignal(tick_store));
    provide_context(SelectedSymbolSignal(selected_symbol));

    view! {
        <div class="dashboard">
            <header class="dashboard__header">
                <h1>"Rust Market Dashboard"</h1>
                <p>"Live view of the last traded price for each symbol."</p>
            </header>
            <section class="dashboard__body">
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
