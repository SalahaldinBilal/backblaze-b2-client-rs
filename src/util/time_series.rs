use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TimeSeriesDataPoint<T> {
    pub data: T,
    pub time: Instant,
}

impl<T> TimeSeriesDataPoint<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            time: Instant::now(),
        }
    }
}

impl<T> AsRef<T> for TimeSeriesDataPoint<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

#[derive(Debug)]
pub struct RollingTimeSeries<T, const N: usize> {
    data_points: [Option<TimeSeriesDataPoint<T>>; N],
    max_age: Duration,
}

impl<T, const N: usize> RollingTimeSeries<T, N> {
    pub fn new(max_age: Duration) -> Self {
        Self {
            data_points: [const { None }; N],
            max_age,
        }
    }

    pub fn get_valid_points(&self) -> Vec<&TimeSeriesDataPoint<T>> {
        self.data_points
            .iter()
            .filter_map(|curr| match curr {
                Some(point) if point.time.elapsed() < self.max_age => Some(point),
                Some(_) | None => None,
            })
            .collect()
    }

    pub fn add_value(&mut self, value: T) {
        let mut oldest_datapoint: &mut Option<TimeSeriesDataPoint<T>> = &mut None;

        for data_point in self.data_points.iter_mut() {
            match data_point {
                Some(dp) if dp.time.elapsed() >= self.max_age => {
                    *data_point = Some(TimeSeriesDataPoint::new(value));
                    return;
                }
                Some(dp) => match oldest_datapoint {
                    Some(dp2) if dp.time.elapsed() > dp2.time.elapsed() => {
                        oldest_datapoint = data_point
                    }
                    Some(_) | None => oldest_datapoint = data_point,
                },
                None => {
                    *data_point = Some(TimeSeriesDataPoint::new(value));
                    return;
                }
            }
        }

        if let Some(_) = oldest_datapoint {
            *oldest_datapoint = Some(TimeSeriesDataPoint::new(value));
        }
    }
}
