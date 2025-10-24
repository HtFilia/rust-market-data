use serde::{Deserialize, Serialize};

use crate::model::{Region, Sector};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: String,
    pub price: f64,
    pub timestamp_ms: u128,
    pub region: Region,
    pub sector: Sector,
}
