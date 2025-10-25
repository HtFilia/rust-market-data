use std::collections::{HashMap, VecDeque};

use indexmap::IndexMap;

use super::types::{HistoryPoint, Tick};

/// In-memory structure keeping the latest tick per symbol and recent history.
#[derive(Clone)]
pub struct TickStore {
    max_history: usize,
    latest: IndexMap<String, Tick>,
    history: HashMap<String, VecDeque<HistoryPoint>>,
}

impl TickStore {
    pub fn new(max_history: usize) -> Self {
        Self {
            max_history,
            latest: IndexMap::new(),
            history: HashMap::new(),
        }
    }

    /// Ingest a single tick, updating the latest price and history buffer.
    pub fn ingest(&mut self, tick: Tick) {
        let symbol = tick.symbol.clone();
        self.latest.insert(symbol.clone(), tick.clone());
        let entry = self.history.entry(symbol).or_default();
        entry.push_back((&tick).into());
        if entry.len() > self.max_history {
            entry.pop_front();
        }
    }

    /// Ingest a batch of ticks, updating the latest snapshot in one pass.
    pub fn ingest_batch<I>(&mut self, ticks: I)
    where
        I: IntoIterator<Item = Tick>,
    {
        for tick in ticks {
            self.ingest(tick);
        }
    }

    pub fn latest(&self) -> &IndexMap<String, Tick> {
        &self.latest
    }

    pub fn latest_mut(&mut self) -> &mut IndexMap<String, Tick> {
        &mut self.latest
    }

    pub fn history_for(&self, symbol: &str) -> Option<&VecDeque<HistoryPoint>> {
        self.history.get(symbol)
    }

    /// Reset the store to an empty state, removing all cached ticks and history.
    pub fn clear(&mut self) {
        self.latest.clear();
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tick(symbol: &str, price: f64, timestamp_ms: u64) -> Tick {
        Tick {
            symbol: symbol.to_string(),
            price,
            timestamp_ms,
            region: crate::ticks::types::Region::NorthAmerica,
            sector: crate::ticks::types::Sector::Technology,
        }
    }

    #[test]
    fn maintains_latest_per_symbol() {
        let mut store = TickStore::new(4);
        store.ingest(sample_tick("AAA", 10.0, 1));
        store.ingest(sample_tick("AAA", 11.0, 2));
        store.ingest(sample_tick("BBB", 9.5, 3));

        assert_eq!(store.latest().len(), 2);
        assert_eq!(store.latest().get("AAA").unwrap().price, 11.0);
    }

    #[test]
    fn trims_history_bound() {
        let mut store = TickStore::new(2);
        store.ingest(sample_tick("AAA", 10.0, 1));
        store.ingest(sample_tick("AAA", 11.0, 2));
        store.ingest(sample_tick("AAA", 12.0, 3));

        let history = store.history_for("AAA").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history.front().unwrap().price, 11.0);
        assert_eq!(history.back().unwrap().price, 12.0);
    }

    #[test]
    fn batch_ingest_updates_multiple_symbols() {
        let mut store = TickStore::new(4);
        store.ingest_batch(vec![
            sample_tick("AAA", 10.0, 1),
            sample_tick("BBB", 20.0, 1),
            sample_tick("AAA", 11.0, 2),
        ]);

        assert_eq!(store.latest().len(), 2);
        assert_eq!(store.latest().get("AAA").unwrap().price, 11.0);
        assert_eq!(store.latest().get("BBB").unwrap().price, 20.0);
    }
}
