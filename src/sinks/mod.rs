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

//! Metric sinks for outputting aggregated data to various destinations
//!
//! Sinks receive aggregated metrics data and forward them to external systems
//! for storage, alerting, or further processing.

// use crate::model::Record;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// TODO: Phase 1 - implement telemetry sink
// pub mod telemetry;

/// Trait for all metric sinks
#[async_trait]
pub trait MetricsSink: Send + Sync {
    /// Send aggregated data to this sink
    async fn send(&self, data: &AggregatedData) -> Result<()>;

    /// Flush any buffered data
    async fn flush(&self) -> Result<()>;

    /// Close the sink gracefully
    async fn close(&self) -> Result<()>;

    /// Get the name of this sink
    fn name(&self) -> &str;
}

/// Aggregated metrics data for a specific time period
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregatedData {
    /// Start time of the aggregation period
    pub start_time: DateTime<Utc>,

    /// End time of the aggregation period
    pub end_time: DateTime<Utc>,

    /// Duration of the aggregation period
    pub period: Duration,

    /// Dimensions/tags for this data
    pub dimensions: HashMap<String, String>,

    /// Aggregated metrics by name
    pub metrics: HashMap<String, AggregatedMetric>,
}

/// A single aggregated metric with computed statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregatedMetric {
    /// Statistics computed for this metric
    pub statistics: HashMap<String, f64>,

    /// Sample count
    pub count: u64,

    /// Raw values (if configured to retain them)
    pub values: Option<Vec<f64>>,
}