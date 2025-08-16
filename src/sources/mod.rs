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

//! Metric sources for ingesting data from various protocols
//!
//! Sources are responsible for receiving metrics data from external systems
//! and converting them into the internal Record format for aggregation.

use crate::model::Record;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

// TODO: Phase 1 - implement HTTP source
// pub mod http;

/// Trait for all metric sources
#[async_trait]
pub trait MetricsSource: Send + Sync {
    /// Start the source, sending received metrics to the provided channel
    async fn start(&self, sender: MetricSender) -> Result<()>;

    /// Stop the source gracefully
    async fn stop(&self) -> Result<()>;

    /// Get the name of this source
    fn name(&self) -> &str;
}

/// Channel for sending metrics from sources to aggregation
pub type MetricSender = mpsc::UnboundedSender<Record>;

/// Channel for receiving metrics in aggregation
pub type MetricReceiver = mpsc::UnboundedReceiver<Record>;