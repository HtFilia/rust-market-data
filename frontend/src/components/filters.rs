use std::collections::HashSet;

use leptos::{ev, event_target_checked, *};

use crate::ticks::{
    format::{region_label, sector_label},
    types::{Region, Sector},
};

use super::dashboard::FilterState;

#[component]
pub fn FiltersPanel() -> impl IntoView {
    let filters = use_context::<FilterState>().expect("filter state context missing");
    let filters_for_regions = filters.clone();
    let filters_for_sectors = filters.clone();
    let filters_for_clear = filters.clone();

    let region_list: Vec<Region> = Region::ALL.into_iter().collect();
    let sector_list: Vec<Sector> = Sector::ALL.into_iter().collect();

    view! {
        <section class="filters-panel">
            <h2>"Filters"</h2>
            <div class="filters-panel__group">
                <h3>"Regions"</h3>
                <div class="filters-panel__options">
                    <For
                        each=move || region_list.clone().into_iter()
                        key=|region| *region
                        children=move |region| {
                            let filters = filters_for_regions.clone();
                            view! {
                                <label class="filters-panel__option">
                                    <input
                                        type="checkbox"
                                        on:input=move |ev: ev::Event| {
                                            let checked = event_target_checked(&ev);
                                            filters.regions.update(|set: &mut HashSet<Region>| {
                                                if checked {
                                                    set.insert(region);
                                                } else {
                                                    set.remove(&region);
                                                }
                                            });
                                        }
                                        prop:checked=move || filters.regions.with(|set| set.contains(&region))
                                    />
                                    <span>{region_label(region)}</span>
                                </label>
                            }
                        }
                    />
                </div>
            </div>
            <div class="filters-panel__group">
                <h3>"Sectors"</h3>
                <div class="filters-panel__options filters-panel__options--grid">
                    <For
                        each=move || sector_list.clone().into_iter()
                        key=|sector| *sector
                        children=move |sector| {
                            let filters = filters_for_sectors.clone();
                            view! {
                                <label class="filters-panel__option">
                                    <input
                                        type="checkbox"
                                        on:input=move |ev: ev::Event| {
                                            let checked = event_target_checked(&ev);
                                            filters.sectors.update(|set: &mut HashSet<Sector>| {
                                                if checked {
                                                    set.insert(sector);
                                                } else {
                                                    set.remove(&sector);
                                                }
                                            });
                                        }
                                        prop:checked=move || filters.sectors.with(|set| set.contains(&sector))
                                    />
                                    <span>{sector_label(sector)}</span>
                                </label>
                            }
                        }
                    />
                </div>
            </div>
            <button class="filters-panel__clear"
                on:click=move |_| {
                    filters_for_clear.regions.set(HashSet::new());
                    filters_for_clear.sectors.set(HashSet::new());
                }
            >
                "Clear filters"
            </button>
        </section>
    }
}
