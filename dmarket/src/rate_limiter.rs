use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const FEE: usize = 110;
const LAST_SALES: usize = 6;
const MARKET_ITEMS: usize = 10;
const OTHER: usize = 20;

pub(crate) struct RateLimiter {
    times: AllocRingBuffer<Instant>,
    limit: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum RateLimiterType {
    Fee,
    LastSales,
    MarketItems,
    Other,
}

impl RateLimiter {
    pub(crate) fn limiters() -> [Mutex<RateLimiter>; 4] {
        [
            Mutex::new(RateLimiter::new(FEE)),
            Mutex::new(RateLimiter::new(LAST_SALES)),
            Mutex::new(RateLimiter::new(MARKET_ITEMS)),
            Mutex::new(RateLimiter::new(OTHER)),
        ]
    }

    pub(crate) fn new(limit: usize) -> Self {
        Self {
            times: AllocRingBuffer::new(limit),
            limit,
        }
    }

    pub(crate) fn check_and_update(&mut self, now: Instant) -> Option<Duration> {
        if self.times.len() < self.limit {
            self.times.push(now);
            return None;
        }

        let oldest = *self.times.get(0).unwrap();
        let next_slot = oldest + Duration::from_secs(1);

        if now >= next_slot {
            self.times.dequeue();
            self.times.push(now);
            None
        } else {
            Some(next_slot - now)
        }
    }
}
