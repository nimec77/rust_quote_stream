use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

use crossbeam::channel::{self, Receiver, Sender};
use rand::{Rng, rng};

use quote_common::{
    DEFAULT_INITIAL_PRICE, DEFAULT_QUOTE_RATE_MS, POPULAR_TICKERS, QuoteError, StockQuote,
};

/// Generates stock quotes on a fixed interval and broadcasts them over a channel.
pub struct QuoteGenerator {
    tickers: Vec<String>,
    prices: HashMap<String, f64>,
    popular: HashSet<String>,
    quote_interval: Duration,
}

impl QuoteGenerator {
    /// Create a new generator, seeding prices from configuration or defaults.
    pub fn new(
        tickers: Vec<String>,
        initial_prices: &HashMap<String, f64>,
        quote_rate_ms: Option<u64>,
    ) -> Self {
        let mut prices = HashMap::with_capacity(tickers.len());
        for ticker in &tickers {
            let price = initial_prices
                .get(ticker)
                .copied()
                .unwrap_or(DEFAULT_INITIAL_PRICE);
            prices.insert(ticker.into(), price);
        }

        let popular = POPULAR_TICKERS.iter().map(|s| s.to_string()).collect();

        Self {
            tickers,
            prices,
            popular,
            quote_interval: Duration::from_millis(quote_rate_ms.unwrap_or(DEFAULT_QUOTE_RATE_MS)),
        }
    }

    fn next_price(&mut self, ticker: &str, rng: &mut impl Rng) -> f64 {
        let current = self
            .prices
            .get(ticker)
            .copied()
            .unwrap_or(DEFAULT_INITIAL_PRICE);
        let delta = rng.random_range(-0.02..0.02);
        let updated = (current * (1.0 + delta)).max(0.01);
        let rounded = (updated * 100.0).round() / 100.0;
        self.prices.insert(ticker.to_string(), rounded);
        rounded
    }

    fn next_volume(&self, ticker: &str, rng: &mut impl Rng) -> u32 {
        let (low, high) = if self.popular.contains(ticker) {
            (1_000, 6_001)
        } else {
            (100, 1_101)
        };
        rng.random_range(low..high)
    }

    /// Start generating quotes, sending them via the provided channel sender.
    pub fn run(mut self, sender: Sender<StockQuote>) {
        let mut rng = rng();
        loop {
            for ticker in self.tickers.clone() {
                let price = self.next_price(&ticker, &mut rng);
                let volume = self.next_volume(&ticker, &mut rng);
                let quote = StockQuote::new(ticker.clone(), price, volume);
                if sender.send(quote).is_err() {
                    return;
                }
            }
            thread::sleep(self.quote_interval);
        }
    }
}

/// Spawn a generator thread and return the receiving side for consumers.
pub fn start_generator(
    tickers: Vec<String>,
    initial_prices: HashMap<String, f64>,
    quote_rate_ms: Option<u64>,
) -> Result<(Receiver<StockQuote>, thread::JoinHandle<()>), QuoteError> {
    let generator = QuoteGenerator::new(tickers, &initial_prices, quote_rate_ms);
    let (sender, receiver) = channel::unbounded();
    let handle = thread::Builder::new()
        .name("quote-generator".to_string())
        .spawn(move || generator.run(sender))
        .map_err(|err| {
            quote_common::quote_error!(IoError, err, "failed to spawn quote generator thread")
        })?;

    Ok((receiver, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn test_next_price_within_bounds() {
        let tickers = vec!["AAPL".to_string()];
        let mut generator = QuoteGenerator::new(tickers.clone(), &HashMap::new(), None);
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..50 {
            let price = generator.next_price("AAPL", &mut rng);
            assert!(price >= 0.01);
        }
    }

    #[test]
    fn test_volume_ranges() {
        let tickers = vec!["AAPL".to_string(), "XYZ".to_string()];
        let generator = QuoteGenerator::new(tickers.clone(), &HashMap::new(), None);
        let mut rng = StdRng::seed_from_u64(7);

        let popular_volume = generator.next_volume("AAPL", &mut rng);
        assert!((1_000..6_001).contains(&popular_volume));

        let regular_volume = generator.next_volume("XYZ", &mut rng);
        assert!((100..1_101).contains(&regular_volume));
    }

    #[test]
    fn test_start_generator_returns_receiver() {
        let tickers = vec!["AAPL".to_string(), "TSLA".to_string()];
        let (receiver, handle) =
            start_generator(tickers.clone(), HashMap::new(), Some(5)).expect("start generator");
        let received: Vec<StockQuote> = receiver.iter().take(4).collect();
        assert_eq!(received.len(), 4);
        for quote in &received {
            assert!(tickers.contains(&quote.ticker));
        }
        drop(receiver);
        handle.join().expect("generator thread should exit");
    }
}
