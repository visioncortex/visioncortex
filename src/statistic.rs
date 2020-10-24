use std::collections::HashMap;

/// Statistics over a sample of objects
pub struct SampleStat {
    pub count: i32,
    pub mode: i32,
    pub median: i32,
    pub median_frequency: i32,
    pub histogram_bins: i32,
    pub mean: f64,
    /// standard deviation
    pub deviation: f64,
}

/// Used to compute `SampleStat`
#[derive(Default)]
pub struct SampleStatBuilder {
    sum: i32,
    sqsum: i32,
    sequence: Vec<i32>,
    histogram: HashMap<i32, i32>,
}

impl SampleStatBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, v: i32) {
        self.sum += v;
        self.sqsum += v * v;
        let counter = self.histogram.entry(v).or_insert(0);
        *counter += 1;
        self.sequence.push(v);
    }

    pub fn build(&mut self) -> SampleStat {
        self.sequence.sort();
        let mean = if !self.sequence.is_empty() {
            self.sum as f64 / self.sequence.len() as f64
        } else {
            0.0
        };
        let variance = if self.sequence.len() > 1 {
            (self.sqsum as f64 - self.sum as f64 * mean) / (self.sequence.len() as f64 - 1.0)
        } else {
            0.0
        };
        SampleStat {
            count: self.sequence.len() as i32,
            mean,
            mode: *self.histogram.iter().max_by_key(|x| x.0).unwrap_or((&0, &0)).0,
            histogram_bins: self.histogram.len() as i32,
            median: Self::median(&self.sequence),
            median_frequency: Self::median_frequency(&self.sequence, &self.histogram),
            deviation: variance.sqrt(),
        }
    }

    pub fn median(sorted_numbers: &[i32]) -> i32 {
        if sorted_numbers.is_empty() {
            return 0;
        }
        let mid = sorted_numbers.len() / 2;
        if sorted_numbers.len() % 2 == 0 {
            (sorted_numbers[mid - 1] + sorted_numbers[mid]) / 2
        } else {
            sorted_numbers[mid]
        }
    }

    pub fn median_frequency(sorted_numbers: &[i32], histogram: &HashMap<i32, i32>) -> i32 {
        if sorted_numbers.is_empty() {
            return 0;
        }
        let mid = sorted_numbers.len() / 2;
        *histogram.get(&sorted_numbers[mid]).unwrap_or(&0) + 
        if sorted_numbers.len() > 3 {
            if sorted_numbers.len() % 2 == 0 {
                *histogram.get(&sorted_numbers[mid - 1]).unwrap_or(&0)
            } else {
                *histogram.get(&sorted_numbers[mid + 1]).unwrap_or(&0)
            }
        } else {
            0
        }
    }
}
