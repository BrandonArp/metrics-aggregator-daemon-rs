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

use anyhow::Result;
use mad::config::MadConfig;
use mad::Mad;
use std::env;
use std::path::PathBuf;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <config-file>", args[0]);
        std::process::exit(1);
    }

    let config_path = PathBuf::from(&args[1]);
    info!("Starting MAD with config: {}", config_path.display());

    // Load configuration
    let config = MadConfig::from_file(&config_path)?;
    info!("Configuration loaded successfully");

    // Create and start MAD instance
    let mad = Mad::new(config).await?;
    info!("MAD instance created, starting services...");

    // Start all services
    mad.start().await?;
    info!("MAD started successfully");

    // Wait for shutdown signal
    mad.wait_for_shutdown().await?;
    info!("Shutdown complete");

    Ok(())
}