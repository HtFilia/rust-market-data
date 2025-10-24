use leptos::*;

use crate::ticks::types::Tick;

use super::dashboard::{SelectedSymbolSignal, TickStoreSignal};

#[component]
pub fn TickTable() -> impl IntoView {
    let tick_store = use_context::<TickStoreSignal>().expect("tick store context missing");
    let selected_symbol =
        use_context::<SelectedSymbolSignal>().expect("selected symbol context missing");

    let rows = create_memo(move |_| {
        tick_store
            .0
            .with(|store| store.latest().values().cloned().collect::<Vec<Tick>>())
    });

    view! {
        <section class="tick-table">
            <h2>"Live Quotes"</h2>
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
                            let selected = selected_symbol;
                            let symbol_display = tick.symbol.clone();
                            let symbol_for_click = symbol_display.clone();
                            let symbol_for_selected = symbol_display.clone();
                            let price = tick.price;
                            let region = tick.region;
                            let sector = tick.sector;

                            view! {
                                <tr
                                    on:click=move |_| {
                                        selected.0.set(Some(symbol_for_click.clone()));
                                    }
                                    class:selected=move || selected.0.get().as_deref() == Some(symbol_for_selected.as_str())
                                >
                                    <td>{symbol_display}</td>
                                    <td>{format!("{:.4}", price)}</td>
                                    <td>{format_region(region)}</td>
                                    <td>{format_sector(sector)}</td>
                                </tr>
                            }
                        }
                    />
                </tbody>
            </table>
        </section>
    }
}

fn format_region(region: crate::ticks::types::Region) -> &'static str {
    use crate::ticks::types::Region::*;
    match region {
        NorthAmerica => "North America",
        SouthAmerica => "South America",
        Europe => "Europe",
        AsiaPacific => "Asia-Pacific",
        MiddleEastAfrica => "Middle East & Africa",
    }
}

fn format_sector(sector: crate::ticks::types::Sector) -> &'static str {
    use crate::ticks::types::Sector::*;
    match sector {
        Technology => "Technology",
        Financials => "Financials",
        Industrials => "Industrials",
        Healthcare => "Healthcare",
        ConsumerDiscretionary => "Consumer Discretionary",
        ConsumerStaples => "Consumer Staples",
        Energy => "Energy",
        Utilities => "Utilities",
        Materials => "Materials",
        RealEstate => "Real Estate",
    }
}
