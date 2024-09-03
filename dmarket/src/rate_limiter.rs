use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::time::{Duration, Instant};

pub(crate) struct RateLimiter {
    request_times: AllocRingBuffer<Instant>,
    rate_limit: usize,
    time_frame: Duration,
}

impl RateLimiter {
    pub(crate) fn new(rate_limit: usize, time_frame: Duration) -> Self {
        Self {
            request_times: AllocRingBuffer::new(rate_limit),
            rate_limit,
            time_frame,
        }
    }

    pub(crate) fn check_and_update(&mut self, now: Instant) -> bool {
        if self.request_times.len() < self.rate_limit {
            self.request_times.push(now);
            return true;
        }

        if let Some(oldest) = self.request_times.get(0) {
            if now.duration_since(*oldest) >= self.time_frame {
                self.request_times.dequeue();
                self.request_times.push(now);
                return true;
            }
        }

        false
    }
}
