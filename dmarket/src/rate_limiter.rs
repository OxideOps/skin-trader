use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

const FEE: usize = 110;
const LAST_SALES: usize = 6;
const MARKET_ITEMS: usize = 5;
const OTHER: usize = 20;

const ONE_SECOND: Duration = Duration::from_secs(1);

pub type RateLimiters = [Mutex<RateLimiter>; 4];

pub struct RateLimiter {
    times: AllocRingBuffer<Instant>,
}

#[derive(Clone, Copy)]
pub enum RateLimiterType {
    Fee,
    LastSales,
    MarketItems,
    Other,
}

impl RateLimiter {
    pub fn request_limiters() -> RateLimiters {
        [
            Mutex::new(Self::new(FEE)),
            Mutex::new(Self::new(LAST_SALES)),
            Mutex::new(Self::new(MARKET_ITEMS)),
            Mutex::new(Self::new(OTHER)),
        ]
    }

    pub fn new(limit: usize) -> Self {
        Self {
            times: AllocRingBuffer::new(limit),
        }
    }

    pub async fn wait(&mut self) {
        if self.times.is_full() {
            sleep(*self.times.front().unwrap() + ONE_SECOND - Instant::now()).await;
        }
        self.times.push(Instant::now());
    }
}
