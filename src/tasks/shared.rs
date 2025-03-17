use std::{
    fmt::Display,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use tokio::io::{AsyncRead, AsyncSeek};

use crate::util::{write_lock_arc::WriteLockArc, RollingTimeSeries, SizeUnit};

pub trait AsyncFileReader: AsyncRead + AsyncSeek + Unpin + Send + Sync {}
impl<T: AsyncRead + AsyncSeek + Unpin + Send + Sync> AsyncFileReader for T {}

#[derive(Debug, Clone)]
pub struct CurrentFileNetworkStats {
    /// Bytes per seconds
    pub bps: SizeUnit,
    /// Estimated finished time in seconds
    pub eta: Duration,
    /// Completion Percentage
    pub percentage: f64,
    /// Uploaded bytes so far
    pub done: SizeUnit,
    /// Total bytes to upload
    pub total: SizeUnit,
    /// Elapsed time
    pub elapsed: Duration,
}

impl CurrentFileNetworkStats {}

impl Display for CurrentFileNetworkStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match f.precision() {
            Some(precision) =>  f.write_fmt(format_args!(
                "Speed: {:.precision$}PS | ETA: {:.precision$?} | Progress: {:.precision$}/{:.precision$} ({:.precision$}%) | Elapsed: {:.precision$?}",
                self.bps, self.eta, self.done, self.total, self.percentage * 100.0, self.elapsed, precision = precision
            )),
            None =>  f.write_fmt(format_args!(
                "Speed: {}PS | ETA: {:?} | Progress: {}/{}({}) | Elapsed: {:?}",
                self.bps, self.eta, self.done, self.total, self.percentage, self.elapsed
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Pending,
    Working,
    Finished,
    Retrying,
    Aborted,
}

#[derive(Debug)]
pub struct FileNetworkStats {
    pub(super) done: Arc<AtomicU64>,
    pub(super) speed_buffer: WriteLockArc<RollingTimeSeries<u64, 5000>>,
    pub(super) total: f64,
    pub(super) start_time: WriteLockArc<Instant>,
}

impl FileNetworkStats {
    pub(super) fn new(total: f64) -> Self {
        Self {
            total,
            done: Arc::new(AtomicU64::new(0)),
            speed_buffer: WriteLockArc::new(RollingTimeSeries::new(Duration::from_secs(10))),
            start_time: WriteLockArc::new(Instant::now()),
        }
    }

    /// Returns estimated download/upload speed in bytes per second
    pub fn bytes_per_second(&self) -> f64 {
        self.inner_bytes_per_second()
    }

    /// Returns estimated finish time in seconds
    pub fn estimated_time(&self) -> f64 {
        let done = self.done.load(Ordering::Relaxed) as f64;

        self.inner_estimated_time(done)
    }

    /// Returns current percentage
    pub fn percentage(&self) -> f64 {
        let done = self.done.load(Ordering::Relaxed) as f64;

        done / self.total
    }

    /// Returns file stats at this point of time
    pub fn current_stats(&self) -> CurrentFileNetworkStats {
        let done = self.done.load(Ordering::Relaxed) as f64;

        CurrentFileNetworkStats {
            bps: self.inner_bytes_per_second().into(),
            eta: Duration::from_secs_f64(self.inner_estimated_time(done).max(0.0)),
            percentage: done / self.total,
            done: done.into(),
            total: self.total.into(),
            elapsed: self.start_time.elapsed(),
        }
    }

    pub(super) async fn add_done_bytes(&self, bytes: u64) {
        self.done.fetch_add(bytes, Ordering::Relaxed);
        let mut buffer = self.speed_buffer.lock_write().await;
        buffer.add_value(bytes);
    }

    fn inner_bytes_per_second(&self) -> f64 {
        let dps = self.speed_buffer.get_valid_points();
        let mut total = 0.0;
        let oldest_time = dps
            .iter()
            .map(|dp| {
                total += dp.data as f64;
                dp.time.elapsed()
            })
            .max();

        match oldest_time {
            Some(dur) => total / dur.as_secs_f64(),
            None => 0.0,
        }
    }

    fn inner_estimated_time(&self, done: f64) -> f64 {
        let mut bytes_per_sec = self.inner_bytes_per_second();

        if bytes_per_sec == 0.0 {
            bytes_per_sec = 1.0;
        }

        (self.total - done) / bytes_per_sec
    }
}
