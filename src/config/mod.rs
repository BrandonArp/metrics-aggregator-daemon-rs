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

//! Configuration parsing and compatibility layer
//!
//! Provides configuration loading that maintains compatibility with the Java
//! implementation while supporting both JSON and HOCON formats.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Main MAD configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MadConfig {
    /// Directory containing pipeline configuration files
    pub pipelines_directory: PathBuf,

    /// HTTP server bind address
    #[serde(default = "default_http_host")]
    pub http_host: IpAddr,

    /// HTTP server port
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// HTTPS configuration (optional)
    pub https_config: Option<HttpsConfig>,

    /// Monitoring configuration
    #[serde(default)]
    pub monitoring: MonitoringConfig,

    /// Log directory
    #[serde(default = "default_log_directory")]
    pub log_directory: PathBuf,

    /// JVM metrics collection interval
    #[serde(
        default = "default_jvm_metrics_interval",
        with = "duration_serde"
    )]
    pub jvm_metrics_collection_interval: Duration,

    /// Legacy Pekko configuration (pass-through for compatibility)
    #[serde(default)]
    pub pekko_configuration: HashMap<String, serde_json::Value>,
}

/// HTTPS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpsConfig {
    /// HTTPS server bind address
    pub https_host: IpAddr,

    /// HTTPS server port
    pub https_port: u16,

    /// Path to TLS certificate file
    pub https_certificate_path: PathBuf,

    /// Path to TLS private key file
    pub https_key_path: PathBuf,
}

/// Monitoring configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringConfig {
    /// Monitoring cluster name
    #[serde(default = "default_monitoring_cluster")]
    pub cluster: String,

    /// Monitoring service name
    #[serde(default = "default_monitoring_service")]
    pub service: String,

    /// Monitoring host (optional)
    pub host: Option<String>,

    /// Monitoring sinks configuration
    #[serde(default)]
    pub sinks: Vec<serde_json::Value>,
}

impl MadConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        // Determine format by extension
        if path.extension().map_or(false, |ext| ext == "conf") {
            // HOCON format - for now, treat as JSON (Phase 2: add proper HOCON support)
            Self::from_json(&contents)
        } else {
            // JSON format
            Self::from_json(&contents)
        }
    }

    /// Parse configuration from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse configuration")
    }
}

// Default value functions
fn default_http_host() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}

fn default_http_port() -> u16 {
    7090
}

fn default_log_directory() -> PathBuf {
    PathBuf::from("logs")
}

fn default_jvm_metrics_interval() -> Duration {
    Duration::from_secs(1)
}

fn default_monitoring_cluster() -> String {
    "mad".to_string()
}

fn default_monitoring_service() -> String {
    "mad".to_string()
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            cluster: default_monitoring_cluster(),
            service: default_monitoring_service(),
            host: None,
            sinks: Vec::new(),
        }
    }
}

// Duration serialization helpers
mod duration_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to ISO-8601 duration format (PT1.0S)
        let seconds = duration.as_secs_f64();
        let iso_duration = format!("PT{}S", seconds);
        serializer.serialize_str(&iso_duration)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Parse ISO-8601 duration format
        if s.starts_with("PT") && s.ends_with('S') {
            let seconds_str = &s[2..s.len() - 1];
            let seconds: f64 = seconds_str.parse().map_err(serde::de::Error::custom)?;
            Ok(Duration::from_secs_f64(seconds))
        } else {
            Err(serde::de::Error::custom(format!(
                "Invalid duration format: {}",
                s
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let json_config = r#"
        {
            "pipelinesDirectory": "config/pipelines",
            "httpHost": "0.0.0.0",
            "httpPort": 7090,
            "logDirectory": "logs",
            "jvmMetricsCollectionInterval": "PT1.0S",
            "monitoring": {
                "cluster": "test_cluster",
                "service": "test_service"
            }
        }
        "#;

        let config = MadConfig::from_json(json_config).unwrap();
        assert_eq!(config.http_port, 7090);
        assert_eq!(config.monitoring.cluster, "test_cluster");
        assert_eq!(config.jvm_metrics_collection_interval, Duration::from_secs(1));
    }

    #[test]
    fn test_duration_parsing() {
        let json_config = r#"
        {
            "pipelinesDirectory": "config/pipelines",
            "jvmMetricsCollectionInterval": "PT5.5S"
        }
        "#;

        let config = MadConfig::from_json(json_config).unwrap();
        assert_eq!(
            config.jvm_metrics_collection_interval,
            Duration::from_secs_f64(5.5)
        );
    }
}