use serde::{Deserialize, Serialize};

/// Geographical region of the issuer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Region {
    NorthAmerica,
    SouthAmerica,
    Europe,
    AsiaPacific,
    MiddleEastAfrica,
}

impl Region {
    pub const ALL: [Region; 5] = [
        Region::NorthAmerica,
        Region::SouthAmerica,
        Region::Europe,
        Region::AsiaPacific,
        Region::MiddleEastAfrica,
    ];
}

/// Activity sector classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    Technology,
    Financials,
    Industrials,
    Healthcare,
    ConsumerDiscretionary,
    ConsumerStaples,
    Energy,
    Utilities,
    Materials,
    RealEstate,
}

impl Sector {
    pub const ALL: [Sector; 10] = [
        Sector::Technology,
        Sector::Financials,
        Sector::Industrials,
        Sector::Healthcare,
        Sector::ConsumerDiscretionary,
        Sector::ConsumerStaples,
        Sector::Energy,
        Sector::Utilities,
        Sector::Materials,
        Sector::RealEstate,
    ];
}

/// Latest market data tick payload produced by the websocket feed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: String,
    pub price: f64,
    pub timestamp_ms: u64,
    pub region: Region,
    pub sector: Sector,
}

impl Tick {
    /// Convenience accessor for sorting keys.
    pub fn symbol_key(&self) -> &str {
        &self.symbol
    }
}

/// Lightweight historical point derived from ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistoryPoint {
    pub timestamp_ms: u64,
    pub price: f64,
}

impl From<&Tick> for HistoryPoint {
    fn from(source: &Tick) -> Self {
        HistoryPoint {
            timestamp_ms: source.timestamp_ms,
            price: source.price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_deserializes_from_sample() {
        let json = r#"{
            "symbol": "NA_TECH007",
            "price": 134.2875,
            "timestamp_ms": 1716400005123,
            "region": "north_america",
            "sector": "technology"
        }"#;

        let tick: Tick = serde_json::from_str(json).expect("valid tick");
        assert_eq!(tick.symbol, "NA_TECH007");
        assert_eq!(tick.region, Region::NorthAmerica);
        assert_eq!(tick.sector, Sector::Technology);
    }
}
