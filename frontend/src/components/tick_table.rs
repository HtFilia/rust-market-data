use std::collections::HashSet;

use leptos::*;

use crate::{
    StreamStatus, TickStore,
    ticks::{
        format::{region_label, sector_label},
        types::{Region, Sector, Tick},
    },
};

use super::dashboard::{
    ConnectionStatusSignal, FilterState, SelectedSymbolSignal, TickStoreSignal,
};

#[component]
pub fn TickTable() -> impl IntoView {
    let tick_store = use_context::<TickStoreSignal>().expect("tick store context missing");
    let selected_symbol =
        use_context::<SelectedSymbolSignal>().expect("selected symbol context missing");
    let filters = use_context::<FilterState>().expect("filter state context missing");
    let connection =
        use_context::<ConnectionStatusSignal>().expect("connection status context missing");
    let store_signal = tick_store.0;

    let rows = create_memo(move |_| {
        let selected_regions = filters.regions.get();
        let selected_sectors = filters.sectors.get();

        tick_store.0.with(|store| {
            if selected_regions.is_empty() && selected_sectors.is_empty() {
                return Vec::new();
            }

            store
                .latest()
                .values()
                .filter(|tick| matches_filters(&selected_regions, &selected_sectors, tick))
                .cloned()
                .collect::<Vec<Tick>>()
        })
    });

    view! {
        <section class="tick-table">
            <h2>"Live Quotes"</h2>
            <Show
                when=move || !rows.get().is_empty()
                fallback=move || {
                    let regions = filters.regions.get();
                    let sectors = filters.sectors.get();
                    let status = connection.0.get();
                    let message = if regions.is_empty() && sectors.is_empty() {
                        "Select a region and sector to display symbols.".to_string()
                    } else {
                        match status {
                            StreamStatus::Connecting => "Connecting to market data...".to_string(),
                            StreamStatus::Reconnecting { .. } => {
                                "Reconnecting to the gateway...".to_string()
                            }
                            StreamStatus::Failed => {
                                "Connection lost. Attempting to reconnect...".to_string()
                            }
                            StreamStatus::Connected => {
                                "Waiting for symbols matching your filters.".to_string()
                            }
                            StreamStatus::Idle => "Waiting for connection...".to_string(),
                        }
                    };

                    view! { <p class="tick-table__empty">{message}</p> }
                }
            >
                <table>
                    <thead>
                        <tr>
                            <th>"Symbol"</th>
                            <th>"Price"</th>
                            <th>"Region"</th>
                            <th>"Sector"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || rows.get()
                            key=|tick| tick.symbol.clone()
                            children=move |tick: Tick| {
                                let store_for_row = store_signal;
                                let selected = selected_symbol;
                                let symbol_display = tick.symbol.clone();
                                let symbol_for_click = symbol_display.clone();
                                let symbol_for_selection = symbol_display.clone();

                                let price = price_signal(store_for_row, symbol_display.clone(), tick.price);
                                let region =
                                    region_signal(store_for_row, symbol_display.clone(), tick.region);
                                let sector =
                                    sector_signal(store_for_row, symbol_display.clone(), tick.sector);

                                view! {
                                    <tr
                                        on:click=move |_| {
                                            selected.0.set(Some(symbol_for_click.clone()));
                                        }
                                        class:selected={
                                            let symbol_for_class = symbol_for_selection.clone();
                                            move || selected.0.get().as_deref() == Some(symbol_for_class.as_str())
                                        }
                                    >
                                        <td>{symbol_display}</td>
                                        <td>{move || price.get()}</td>
                                        <td>{move || region.get()}</td>
                                        <td>{move || sector.get()}</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </section>
    }
}

fn matches_filters(regions: &HashSet<Region>, sectors: &HashSet<Sector>, tick: &Tick) -> bool {
    if regions.is_empty() && sectors.is_empty() {
        return false;
    }
    let region_ok = regions.is_empty() || regions.contains(&tick.region);
    let sector_ok = sectors.is_empty() || sectors.contains(&tick.sector);
    region_ok && sector_ok
}

fn price_signal(store: RwSignal<TickStore>, symbol: String, fallback: f64) -> Memo<String> {
    create_memo(move |_| {
        store.with(|state| {
            state
                .latest()
                .get(&symbol)
                .map(|tick| format!("{:.4}", tick.price))
                .unwrap_or_else(|| format!("{fallback:.4}"))
        })
    })
}

fn region_signal(store: RwSignal<TickStore>, symbol: String, fallback: Region) -> Memo<String> {
    create_memo(move |_| {
        store.with(|state| {
            state
                .latest()
                .get(&symbol)
                .map(|tick| region_label(tick.region).to_string())
                .unwrap_or_else(|| region_label(fallback).to_string())
        })
    })
}

fn sector_signal(store: RwSignal<TickStore>, symbol: String, fallback: Sector) -> Memo<String> {
    create_memo(move |_| {
        store.with(|state| {
            state
                .latest()
                .get(&symbol)
                .map(|tick| sector_label(tick.sector).to_string())
                .unwrap_or_else(|| sector_label(fallback).to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::{create_runtime, create_rw_signal};

    #[test]
    fn matches_filters_respects_selected_sets() {
        let mut regions = HashSet::new();
        let mut sectors = HashSet::new();
        let tick = Tick {
            symbol: "AAA".into(),
            price: 10.0,
            timestamp_ms: 1,
            region: Region::NorthAmerica,
            sector: Sector::Technology,
        };

        assert!(!matches_filters(&regions, &sectors, &tick));

        regions.insert(Region::NorthAmerica);
        assert!(matches_filters(&regions, &sectors, &tick));

        sectors.insert(Sector::Technology);
        assert!(matches_filters(&regions, &sectors, &tick));

        sectors.clear();
        sectors.insert(Sector::Energy);
        assert!(!matches_filters(&regions, &sectors, &tick));
    }

    #[test]
    fn price_signal_updates_with_store_changes() {
        let runtime = create_runtime();
        let store = create_rw_signal(TickStore::new(16));
        let symbol = "AAA".to_string();

        store.update(|state| {
            state.ingest(Tick {
                symbol: symbol.clone(),
                price: 10.0,
                timestamp_ms: 1,
                region: Region::NorthAmerica,
                sector: Sector::Technology,
            });
        });

        let price = price_signal(store, symbol.clone(), 0.0);
        assert_eq!(price.get(), "10.0000");

        store.update(|state| {
            state.ingest(Tick {
                symbol: symbol.clone(),
                price: 12.5,
                timestamp_ms: 2,
                region: Region::NorthAmerica,
                sector: Sector::Technology,
            });
        });

        assert_eq!(price.get(), "12.5000");
        runtime.dispose();
    }
}
