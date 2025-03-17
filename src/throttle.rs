use std::{
    ops::AddAssign,
    time::{Duration, Instant},
};

use num::Unsigned;
use tokio::time::sleep;

#[derive(Debug)]
pub struct Throttle<T: Unsigned + AddAssign + Copy + PartialOrd> {
    max_per_period: T,
    count_start: Instant,
    period: Duration,
    current_count: T,
}

impl<T: Unsigned + AddAssign + Copy + PartialOrd> Throttle<T> {
    pub fn new(max_per_period: T, period: Duration) -> Self {
        Self {
            max_per_period,
            period,
            count_start: Instant::now(),
            current_count: T::zero(),
        }
    }

    /// Equivalent to
    /// ```rust
    /// Throttle::new(max_per_period, Duration::from_secs(1))
    /// ```
    pub fn per_second(max_per_period: T) -> Self {
        Self::new(max_per_period, Duration::from_secs(1))
    }

    /// Equivalent to
    /// ```rust
    /// Throttle::new(max_per_period, Duration::from_secs(60))
    /// ```
    pub fn per_minute(max_per_period: T) -> Self {
        Self::new(max_per_period, Duration::from_secs(60))
    }

    /// Advances the throttle by 1, waiting if the throttle has been exhausted
    pub async fn advance(&mut self) -> T {
        self.advance_by(T::one()).await
    }

    /// Advances the throttle by the given amount, waiting if the throttle has been exhausted
    pub async fn advance_by(&mut self, by: T) -> T {
        if self.count_start.elapsed() >= self.period {
            self.current_count = T::zero();
            self.count_start = Instant::now();
        }

        if self.current_count >= self.max_per_period {
            sleep(self.period - self.count_start.elapsed()).await;
            self.current_count = T::zero();
            self.count_start = Instant::now();
        }

        self.current_count += by;

        return if self.current_count > self.max_per_period {
            T::zero()
        } else {
            self.max_per_period - self.current_count
        };
    }

    /// If throttle period has been exhausted, waits for the period to end <br>
    /// otherwise returns immediately
    pub async fn wait_if_exhausted(&self) {
        if self.count_start.elapsed() >= self.period {
            return;
        }

        if self.current_count >= self.max_per_period {
            sleep(self.period - self.count_start.elapsed()).await;
        }
    }

    /// Returns the remaining count for the current period
    pub fn remaining(&self) -> T {
        if self.count_start.elapsed() >= self.period {
            return self.max_per_period;
        }

        return if self.current_count > self.max_per_period {
            T::zero()
        } else {
            self.max_per_period - self.current_count
        };
    }
}

impl<T: Unsigned + AddAssign + Copy + PartialOrd> Clone for Throttle<T> {
    fn clone(&self) -> Self {
        Self {
            max_per_period: self.max_per_period,
            period: self.period.clone(),
            count_start: Instant::now(),
            current_count: T::zero(),
        }
    }
}
