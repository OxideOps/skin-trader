use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub(crate) struct RateLimiter {
    request_times: AllocRingBuffer<Instant>,
    request_limit: usize,
    time_frame: Duration,
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
            Mutex::new(RateLimiter::new(110, Duration::from_secs(1))), // Fee
            Mutex::new(RateLimiter::new(6, Duration::from_secs(1))),   // LastSales
            Mutex::new(RateLimiter::new(10, Duration::from_secs(1))),  // MarketItems
            Mutex::new(RateLimiter::new(20, Duration::from_secs(1))),  // Other
        ]
    }

    pub(crate) fn new(request_limit: usize, time_frame: Duration) -> Self {
        Self {
            request_times: AllocRingBuffer::new(request_limit),
            request_limit,
            time_frame,
        }
    }

    pub(crate) fn check_and_update(&mut self, now: Instant) -> Option<Duration> {
        if self.request_times.len() < self.request_limit {
            self.request_times.push(now);
            return None;
        }

        let oldest = *self.request_times.get(0).unwrap();
        let next_slot = oldest + self.time_frame;

        if now >= next_slot {
            self.request_times.dequeue();
            self.request_times.push(now);
            None
        } else {
            Some(next_slot - now)
        }
    }
}
