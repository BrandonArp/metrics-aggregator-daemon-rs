/*
 * Copyright 2024 ArpNetworking
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Core data model for metrics aggregation
//!
//! Defines the fundamental data structures used throughout MAD for representing
//! metrics, records, and aggregated data.

use ahash::{HashMap, HashMapExt};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A complete metrics record containing multiple metrics with shared dimensions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// Unique identifier for this record
    pub id: String,

    /// Timestamp when the record was created
    pub time: DateTime<Utc>,

    /// Key-value pairs of dimensions/tags associated with all metrics in this record
    pub dimensions: HashMap<String, String>,

    /// Annotations - additional metadata
    pub annotations: HashMap<String, String>,

    /// Map of metric name to metric data
    pub metrics: HashMap<String, Metric>,
}

/// A single metric with its type, values, and computed statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    /// The type of metric (counter, gauge, timer)
    #[serde(rename = "type")]
    pub metric_type: MetricType,

    /// Raw sample values
    pub values: Vec<Quantity>,

    /// Pre-computed statistics for this metric
    #[serde(default)]
    pub statistics: HashMap<String, Vec<CalculatedValue>>,
}

/// Type of metric
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    /// Monotonically increasing counter
    Counter,
    /// Point-in-time value
    Gauge,
    /// Duration measurement
    Timer,
}

/// A single measurement value with optional unit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// The numeric value
    pub value: f64,

    /// Optional unit information
    pub unit: Option<Unit>,
}

/// Unit information for a quantity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    /// The type of unit (e.g., time, data, etc.)
    pub unit_type: UnitType,

    /// The specific unit (e.g., second, byte, etc.)
    pub name: String,
}

/// Category of unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitType {
    /// Time-based units (seconds, milliseconds, etc.)
    Time,
    /// Data size units (bytes, kilobytes, etc.)
    Data,
    /// Count units (items, requests, etc.)
    Count,
    /// Other units
    Other,
}

/// A calculated statistic value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalculatedValue {
    /// The computed value
    pub value: StatisticValue,
}

/// Value of a computed statistic
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatisticValue {
    /// Numeric value
    Number(f64),
    /// Histogram value
    Histogram(SparseHistogram),
}

/// Sparse histogram implementation matching Java MAD
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparseHistogram {
    /// Sparse buckets: bucket lower bound -> count
    pub buckets: BTreeMap<OrderedFloat, u64>,

    /// Minimum value observed
    pub min: f64,

    /// Maximum value observed
    pub max: f64,

    /// Sum of all values
    pub sum: f64,

    /// Total count of samples
    pub count: u64,

    /// Precision mask for bucket calculation
    #[serde(default = "default_precision_mask")]
    pub precision_mask: u64,
}

/// Wrapper for f64 that implements Ord for use in BTreeMap
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct OrderedFloat(pub f64);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl SparseHistogram {
    /// Create a new sparse histogram with default precision (7 bits)
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: 0.0,
            count: 0,
            precision_mask: default_precision_mask(),
        }
    }

    /// Add a sample to the histogram
    pub fn add_sample(&mut self, value: f64) {
        if !value.is_finite() {
            return; // Skip infinite or NaN values
        }

        // Calculate bucket using same algorithm as Java implementation
        let bucket_key = self.calculate_bucket(value);

        // Update bucket count
        *self.buckets.entry(bucket_key).or_insert(0) += 1;

        // Update summary statistics
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
    }

    /// Calculate bucket for a value using IEEE Double mantissa truncation
    fn calculate_bucket(&self, value: f64) -> OrderedFloat {
        let bits = value.to_bits();
        let bucket_bits = bits & self.precision_mask;
        OrderedFloat(f64::from_bits(bucket_bits))
    }

    /// Calculate percentile from histogram
    pub fn percentile(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }

        let target_count = (p / 100.0 * self.count as f64).ceil() as u64;
        let mut running_count = 0;

        for (bucket, count) in &self.buckets {
            running_count += count;
            if running_count >= target_count {
                return bucket.0;
            }
        }

        self.max
    }

    /// Get the mean value
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

impl Default for SparseHistogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Default precision mask (7 bits of precision)
fn default_precision_mask() -> u64 {
    0xffffe00000000000u64
}

/// Key for identifying unique metric series (dimensions + metric name)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricKey {
    /// Dimensions for this metric series
    pub dimensions: HashMap<String, String>,

    /// Name of the metric
    pub metric_name: String,
}

impl std::hash::Hash for MetricKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.metric_name.hash(state);
        // Hash dimensions in a deterministic order
        let mut dims: Vec<_> = self.dimensions.iter().collect();
        dims.sort_by_key(|(k, _)| *k);
        for (k, v) in dims {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl MetricKey {
    /// Create a new metric key
    pub fn new(dimensions: HashMap<String, String>, metric_name: String) -> Self {
        Self {
            dimensions,
            metric_name,
        }
    }

    /// Create a metric key from a record and metric name
    pub fn from_record(record: &Record, metric_name: &str) -> Self {
        Self {
            dimensions: record.dimensions.clone(),
            metric_name: metric_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_histogram() {
        let mut histogram = SparseHistogram::new();

        // Add some samples
        histogram.add_sample(1.0);
        histogram.add_sample(2.0);
        histogram.add_sample(3.0);
        histogram.add_sample(4.0);
        histogram.add_sample(5.0);

        assert_eq!(histogram.count, 5);
        assert_eq!(histogram.min, 1.0);
        assert_eq!(histogram.max, 5.0);
        assert_eq!(histogram.sum, 15.0);
        assert_eq!(histogram.mean(), 3.0);

        // Test percentiles
        assert!(histogram.percentile(50.0) >= 2.0);
        assert!(histogram.percentile(100.0) >= 5.0);
    }

    #[test]
    fn test_metric_serialization() {
        let metric = Metric {
            metric_type: MetricType::Timer,
            values: vec![Quantity {
                value: 1.5,
                unit: Some(Unit {
                    unit_type: UnitType::Time,
                    name: "second".to_string(),
                }),
            }],
            statistics: HashMap::new(),
        };

        let json = serde_json::to_string(&metric).unwrap();
        let deserialized: Metric = serde_json::from_str(&json).unwrap();

        assert_eq!(metric, deserialized);
    }
}