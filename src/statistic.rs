use std::collections::HashMap;

/// Simple statistics
#[derive(Debug, Default, std::cmp::PartialEq)]
pub struct SimpleStat {
    pub count: i32,
    pub mean: f64,
    /// standard deviation
    pub deviation: f64,
}

/// Sample statistics
#[derive(Debug, Default, std::cmp::PartialEq)]
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

#[derive(Default)]
pub struct SimpleStatBuilder {
    sum: i32,
    sqsum: u64,
    count: u32,
}

#[derive(Default)]
pub struct SampleStatBuilder {
    simple: SimpleStatBuilder,
    sequence: Vec<i32>,
    histogram: HashMap<i32, i32>,
}

impl SimpleStatBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, v: i32) {
        self.sum += v;
        self.sqsum += (v * v) as u64;
        self.count += 1;
    }

    pub fn build(&self) -> SimpleStat {
        let mean = if self.count != 0 {
            self.sum as f64 / self.count as f64
        } else {
            0.0
        };
        let variance = if self.count > 1 {
            (self.sqsum as f64 - self.sum as f64 * mean) / (self.count as f64 - 1.0)
        } else {
            0.0
        };

        SimpleStat {
            count: self.count as i32,
            mean,
            deviation: variance.sqrt(),
        }
    }

}

impl SampleStatBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, v: i32) {
        self.simple.add(v);
        let counter = self.histogram.entry(v).or_insert(0);
        *counter += 1;
        self.sequence.push(v);
    }

    pub fn build(&mut self) -> SampleStat {
        let SimpleStat {
            count,
            mean,
            deviation,
        } = self.simple.build();

        self.sequence.sort();
        let max = self.histogram.iter().max_by_key(|x| x.1).unwrap_or((&0, &0)).1;
        let mut maxes: Vec<(&i32, &i32)> = self.histogram.iter().filter(|x| x.1 == max).collect();
        maxes.sort_by_key(|x| x.0);
        let mode = *maxes.get(0).unwrap_or(&(&0, &0)).0;
        let median = Self::median(&self.sequence);

        SampleStat {
            count,
            mean,
            mode,
            histogram_bins: self.histogram.len() as i32,
            median,
            median_frequency: Self::median_frequency(&self.sequence, &self.histogram),
            deviation,
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
        if sorted_numbers.len() % 2 == 0 {
            (*histogram.get(&sorted_numbers[mid - 1]).unwrap_or(&0) +
            *histogram.get(&sorted_numbers[mid]).unwrap_or(&0)) / 2
        } else {
            *histogram.get(&sorted_numbers[mid]).unwrap_or(&0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_builder_1() {
        let mut builder = SampleStatBuilder::new();
        builder.add(1);
        assert_eq!(builder.build(), SampleStat {
            count: 1,
            mean: 1.0,
            mode: 1,
            histogram_bins: 1,
            median: 1,
            median_frequency: 1,
            deviation: 0.0,
        });
    }

    #[test]
    fn test_stat_builder_2() {
        let mut builder = SampleStatBuilder::new();
        builder.add(1);
        builder.add(2);
        builder.add(3);
        assert_eq!(builder.build(), SampleStat {
            count: 3,
            mean: 2.0,
            mode: 1,
            histogram_bins: 3,
            median: 2,
            median_frequency: 1,
            deviation: 1.0,
        });
    }

    #[test]
    fn test_stat_builder_3() {
        let mut builder = SampleStatBuilder::new();
        builder.add(1);
        builder.add(2);
        builder.add(2);
        builder.add(3);
        assert_eq!(builder.build(), SampleStat {
            count: 4,
            mean: 2.0,
            mode: 2,
            histogram_bins: 3,
            median: 2,
            median_frequency: 2,
            deviation: (2.0 / 3.0f64).sqrt(),
        });
    }
}