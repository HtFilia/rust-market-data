use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
};

use indexmap::IndexMap;

use super::types::{HistoryPoint, Tick};

pub type Movers = Vec<(String, f64)>;

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

    /// Return the top advancers and decliners by percentage change since their first recorded price.
    pub fn movers(&self, count: usize) -> (Movers, Movers) {
        if count == 0 || self.latest.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let mut changes: Vec<(String, f64)> = self
            .latest
            .iter()
            .map(|(symbol, _)| {
                let change = self
                    .history
                    .get(symbol)
                    .and_then(|history| {
                        let first = history.front()?;
                        let last = history.back()?;
                        if first.price > 0.0 {
                            Some(((last.price - first.price) / first.price) * 100.0)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);
                (symbol.clone(), change)
            })
            .collect();

        changes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        let advancers = changes
            .iter()
            .filter(|(_, change)| *change > 0.0)
            .take(count)
            .cloned()
            .collect::<Vec<_>>();

        let mut declines = changes.clone();
        declines.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        let decliners = declines
            .into_iter()
            .filter(|(_, change)| *change < 0.0)
            .take(count)
            .collect::<Vec<_>>();

        (advancers, decliners)
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

    #[test]
    fn movers_returns_sorted_advancers_decliners() {
        let mut store = TickStore::new(8);
        store.ingest(sample_tick("AAA", 10.0, 1));
        store.ingest(sample_tick("AAA", 11.0, 2));
        store.ingest(sample_tick("BBB", 20.0, 1));
        store.ingest(sample_tick("BBB", 18.0, 2));
        store.ingest(sample_tick("CCC", 30.0, 1));
        store.ingest(sample_tick("CCC", 39.0, 2));

        let (advancers, decliners) = store.movers(2);
        assert_eq!(advancers.len(), 2);
        assert_eq!(advancers.first().unwrap().0, "CCC");
        assert!(advancers.first().unwrap().1 > 20.0);
        assert_eq!(advancers.last().unwrap().0, "AAA");
        assert!(advancers.last().unwrap().1 > 5.0);

        assert_eq!(decliners.len(), 1);
        assert_eq!(decliners.first().unwrap().0, "BBB");
        assert!(decliners.first().unwrap().1 < 0.0);
    }
}
