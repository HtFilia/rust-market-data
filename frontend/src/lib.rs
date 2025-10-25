use leptos::*;
use wasm_bindgen::prelude::wasm_bindgen;

mod components;
mod logging;
pub mod ticks;

pub use components::dashboard::Dashboard;
pub use logging::init_logging;
pub use ticks::store::TickStore;
pub use ticks::types::{HistoryPoint, Region, Sector, Tick};
pub use ticks::websocket::{StreamStatus, connect_with_retry};

/// Root component bootstrapping the dashboard.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <main class="app-root">
            <Dashboard />
        </main>
    }
}

/// WASM entry point called automatically by `trunk`.
#[wasm_bindgen(start)]
pub fn main() -> Result<(), wasm_bindgen::JsValue> {
    init_logging();
    console_error_panic_hook::set_once();

    leptos::mount_to_body(|| view! { <App /> });
    Ok(())
}
