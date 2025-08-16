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

//! # Metrics Aggregator Daemon (MAD) - Rust Implementation
//!
//! A high-performance, low-resource metrics aggregation service that provides
//! time-based aggregation of metrics from multiple sources and forwards them
//! to configurable sinks.
//!
//! ## Features
//!
//! - **Multiple Input Sources**: HTTP, StatsD, Kafka, OpenTelemetry
//! - **Configurable Aggregation**: Time-bucketed statistics computation
//! - **Multiple Output Sinks**: HTTP, Kafka, Telemetry
//! - **High Performance**: Native Rust performance with minimal memory footprint
//! - **Configuration Compatibility**: Drop-in replacement for Java MAD
//!
//! ## Example
//!
//! ```rust,no_run
//! use mad::{Mad, config::MadConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = MadConfig::from_file("config.conf")?;
//!     let mad = Mad::new(config).await?;
//!     mad.start().await?;
//!     mad.wait_for_shutdown().await?;
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::Notify;
use tracing::{info, warn};

pub mod aggregation;
pub mod config;
pub mod model;
pub mod sinks;
pub mod sources;

// #[cfg(feature = "transformations")]
// pub mod transformations;

use config::MadConfig;

/// Main MAD application instance
pub struct Mad {
    config: MadConfig,
    shutdown_notify: Arc<Notify>,
}

impl Mad {
    /// Create a new MAD instance with the given configuration
    pub async fn new(config: MadConfig) -> Result<Self> {
        Ok(Self {
            config,
            shutdown_notify: Arc::new(Notify::new()),
        })
    }

    /// Start all MAD services (sources, aggregation, sinks)
    pub async fn start(&self) -> Result<()> {
        info!("Starting MAD services");

        // TODO: Phase 1 implementation
        // 1. Start HTTP server for health checks
        // 2. Start metric sources
        // 3. Start aggregation workers
        // 4. Start sinks
        // 5. Set up signal handlers

        self.setup_signal_handlers().await;

        info!("All services started successfully");
        Ok(())
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) -> Result<()> {
        self.shutdown_notify.notified().await;
        info!("Shutdown signal received, stopping services...");

        // TODO: Graceful shutdown implementation
        // 1. Stop accepting new metrics
        // 2. Flush aggregation buffers
        // 3. Stop sinks
        // 4. Stop sources

        Ok(())
    }

    async fn setup_signal_handlers(&self) {
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        tokio::spawn(async move {
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received SIGINT, initiating shutdown");
                    shutdown_notify.notify_one();
                }
                Err(err) => {
                    warn!("Failed to listen for SIGINT: {}", err);
                }
            }
        });

        #[cfg(unix)]
        {
            let shutdown_notify = Arc::clone(&self.shutdown_notify);
            tokio::spawn(async move {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");

                sigterm.recv().await;
                info!("Received SIGTERM, initiating shutdown");
                shutdown_notify.notify_one();
            });
        }
    }
}