# Metrics Aggregator Daemon - Rust Port Plan

## Executive Summary

This document outlines the plan to rewrite the Metrics Aggregator Daemon (MAD) in Rust, focusing on performance, resource efficiency, and seamless migration for existing users. The new implementation will maintain full configuration compatibility while delivering significant performance improvements.

## Motivation

### Performance Benefits
- **Memory Usage**: Reduced memory footprint by eliminating JVM overhead
- **Startup Time**: Faster startup without JVM initialization
- **CPU Efficiency**: Native performance for histogram computations
- **Latency**: Elimination of garbage collection pauses
- **Deterministic Performance**: More predictable resource usage

### Operational Benefits
- **Single Binary Deployment**: No JVM or dependency management required
- **Container Efficiency**: Smaller images, lower resource requirements
- **Infrastructure Costs**: Reduced memory and CPU requirements
- **Debugging**: Better profiling and debugging tools (perf, valgrind)

## Scope Decisions

### Included Features
- All current sources except file-based log parsing
- Full sink implementation parity
- Complete aggregation and statistics functionality
- Configuration compatibility layer
- HTTP/HTTPS endpoints with TLS support
- Dynamic configuration reloading

### Experimental Features (from mad-experimental)
- **OpenTelemetry gRPC Source**: Native OpenTelemetry protocol support
- **TransformingSource**: Metric name and dimension transformation with regex
- **TagDroppingSource**: Dimension dropping and metric merging
- **MetricSeriesLoggingSink**: Time series data export for analysis

### Excluded Features
- **FileSource and log parsing**: Eliminates complex log tailing and parsing logic
- **Legacy protocol versions**: Focus on modern client library support
- **Java-specific JVM metrics**: Replace with Rust-native system metrics

## Architecture Overview

### Technology Stack

#### Core Runtime
```rust
// Primary dependencies
tokio = "1.0"           // Async runtime (replaces Pekko)
axum = "0.7"            // HTTP framework
tower = "0.4"           // Middleware and service abstractions
serde = "1.0"           // Serialization
config = "0.14"         // Configuration management
tracing = "0.1"         // Structured logging
```

#### Specialized Libraries
```rust
// Protocol support
rdkafka = "0.36"        // Kafka client
tokio-udp = "0.1"       // UDP for StatsD
reqwest = "0.11"        // HTTP client for sinks
tonic = "0.10"          // gRPC client/server for OpenTelemetry
prost = "0.12"          // Protocol Buffers for OpenTelemetry

// Performance
ahash = "0.8"           // Fast hashing
dashmap = "5.5"         // Concurrent HashMap
parking_lot = "0.12"    // Fast synchronization primitives
regex = "1.9"           // High-performance regex for transformations
```

### High-Level Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   HTTP Sources  │    │   UDP Sources    │    │  Kafka Sources  │    │  gRPC Sources   │
│  (V1/V2/V3)     │    │   (StatsD)       │    │                 │    │ (OpenTelemetry) │
└─────────┬───────┘    └─────────┬────────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                       │                      │
          └──────────────────────┼───────────────────────┼──────────────────────┘
                                 │                       │
                    ┌────────────▼───────────────────────▼────────────┐
                    │          Transformation Layer                   │
                    │   (TagDroppingSource, TransformingSource)       │
                    └────────────┬────────────────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │   Metric Router         │
                    │  (Dimension-based       │
                    │   partitioning)         │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │  Aggregation Workers    │
                    │  (Async task per        │
                    │   partition)            │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     Sink Router         │
                    └────────────┬────────────┘
                                 │
          ┌──────────────────────┼───────────────────────┬─────────────────────┐
          │                      │                       │                     │
┌─────────▼───────┐    ┌─────────▼────────┐    ┌─────────▼───────┐   ┌─────────▼─────────┐
│ Telemetry Sink  │    │  HTTP Sinks      │    │  Kafka Sinks    │   │ Series Log Sink   │
│                 │    │                  │    │                 │   │                   │
└─────────────────┘    └──────────────────┘    └─────────────────┘   └───────────────────┘
```

## Core Components

### 1. Configuration System

#### Compatibility Layer
```rust
// Old format (HOCON/JSON) -> New format mapping
pub struct ConfigMigrator {
    // Maps old source types to new implementations
    source_mappings: HashMap<String, Box<dyn SourceFactory>>,
    // Maps old sink types to new implementations  
    sink_mappings: HashMap<String, Box<dyn SinkFactory>>,
}

// Example migration
impl ConfigMigrator {
    fn migrate_source(&self, old_config: Value) -> Result<SourceConfig> {
        match old_config["type"].as_str() {
            "com.arpnetworking.metrics.common.sources.ClientHttpSourceV3" => {
                Ok(SourceConfig::Http {
                    name: old_config["name"].as_str().unwrap().to_string(),
                    version: HttpVersion::V3,
                    bind_address: "0.0.0.0:7090".parse()?,
                })
            }
            // ... other mappings
        }
    }
}
```

#### New Configuration Model
```rust
#[derive(Deserialize, Debug, Clone)]
pub struct MadConfig {
    pub pipelines_directory: PathBuf,
    pub http_host: IpAddr,
    pub http_port: u16,
    pub https_config: Option<HttpsConfig>,
    pub monitoring: MonitoringConfig,
    pub pekko_configuration: Value, // Pass-through for compatibility
}

#[derive(Deserialize, Debug, Clone)]
pub struct PipelineConfig {
    pub name: String,
    pub sources: Vec<SourceConfig>,
    pub sinks: Vec<SinkConfig>,
    pub periods: Option<Vec<Duration>>,
    pub statistics: StatisticsConfig,
}
```

### 2. Source Implementations

#### HTTP Sources
```rust
#[async_trait]
pub trait MetricsSource: Send + Sync {
    async fn start(&self, sender: MetricSender) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

pub struct HttpSource {
    version: HttpVersion,
    bind_address: SocketAddr,
    router: Router,
}

impl HttpSource {
    async fn handle_metrics_v3(
        &self,
        body: Bytes,
    ) -> Result<Vec<Record>, HttpError> {
        // Parse metrics in native Rust
        // No JVM serialization overhead
    }
}
```

#### OpenTelemetry gRPC Source
```rust
use opentelemetry_proto::tonic::collector::metrics::v1::{
    metrics_service_server::{MetricsService, MetricsServiceServer},
    ExportMetricsServiceRequest, ExportMetricsServiceResponse,
};

pub struct OpenTelemetryGrpcSource {
    bind_address: SocketAddr,
    sender: MetricSender,
}

#[tonic::async_trait]
impl MetricsService for OpenTelemetryGrpcSource {
    async fn export(
        &self,
        request: tonic::Request<ExportMetricsServiceRequest>,
    ) -> Result<tonic::Response<ExportMetricsServiceResponse>, tonic::Status> {
        let metrics_data = request.into_inner();
        let records = self.parse_otel_metrics(metrics_data)?;
        
        for record in records {
            self.sender.send(record).await.map_err(|e| {
                tonic::Status::internal(format!("Failed to send record: {}", e))
            })?;
        }
        
        Ok(tonic::Response::new(ExportMetricsServiceResponse {
            partial_success: None,
        }))
    }
}
```

#### StatsD Source
```rust
pub struct StatsdSource {
    bind_address: SocketAddr,
    socket: UdpSocket,
    parser: StatsdParser,
}

// High-performance UDP processing
impl StatsdSource {
    async fn run(&self, sender: MetricSender) {
        let mut buffer = vec![0u8; 65536];
        loop {
            match self.socket.recv(&mut buffer).await {
                Ok(size) => {
                    let records = self.parser.parse(&buffer[..size])?;
                    for record in records {
                        sender.send(record).await?;
                    }
                }
                Err(e) => tracing::error!("UDP receive error: {}", e),
            }
        }
    }
}
```

#### Kafka Source
```rust
pub struct KafkaSource {
    consumer: StreamConsumer,
    parser: Box<dyn MetricsParser>,
    topics: Vec<String>,
}

impl KafkaSource {
    async fn consume_loop(&self, sender: MetricSender) {
        let mut stream = self.consumer.stream();
        while let Some(message) = stream.next().await {
            match message {
                Ok(borrowed_message) => {
                    if let Some(payload) = borrowed_message.payload() {
                        let records = self.parser.parse(payload)?;
                        for record in records {
                            sender.send(record).await?;
                        }
                    }
                }
                Err(e) => tracing::error!("Kafka error: {}", e),
            }
        }
    }
}
```

### 3. Aggregation Engine

#### Worker Architecture
```rust
pub struct AggregationWorker {
    partition_id: usize,
    buckets: DashMap<MetricKey, TimeBucket>,
    statistics_config: StatisticsConfig,
    sink_sender: SinkSender,
}

impl AggregationWorker {
    async fn process_record(&self, record: Record) -> Result<()> {
        let key = MetricKey::from_record(&record);
        
        // Get or create time bucket for this metric
        let bucket = self.buckets
            .entry(key)
            .or_insert_with(|| TimeBucket::new(&self.statistics_config));
            
        // Add samples to histogram
        bucket.add_samples(&record.metrics)?;
        
        // Check if bucket should be flushed
        if bucket.should_flush()? {
            let aggregated = bucket.flush()?;
            self.sink_sender.send(aggregated).await?;
        }
        
        Ok(())
    }
}

// Partition records by dimension hash (same as Java implementation)
pub struct MetricRouter {
    workers: Vec<mpsc::Sender<Record>>,
    hasher: RandomState, // ahash for performance
}

impl MetricRouter {
    pub async fn route_record(&self, record: Record) -> Result<()> {
        let hash = self.hasher.hash_one(&record.dimensions);
        let worker_idx = (hash as usize) % self.workers.len();
        self.workers[worker_idx].send(record).await?;
        Ok(())
    }
}
```

#### Histogram Implementation
```rust
// Direct port of current algorithm
#[derive(Debug, Clone)]
pub struct SparseHistogram {
    buckets: BTreeMap<OrderedFloat<f64>, u64>,
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
    precision_mask: u64, // Default: 0xffffe00000000000
}

impl SparseHistogram {
    pub fn add_sample(&mut self, value: f64) {
        // Same bucket calculation as Java version
        let bucket_key = self.calculate_bucket(value);
        *self.buckets.entry(bucket_key).or_insert(0) += 1;
        
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
    }
    
    fn calculate_bucket(&self, value: f64) -> OrderedFloat<f64> {
        let bits = value.to_bits();
        let bucket_bits = bits & self.precision_mask;
        OrderedFloat(f64::from_bits(bucket_bits))
    }
    
    pub fn percentile(&self, p: f64) -> f64 {
        // Implement same percentile calculation logic
        // Potential for SIMD optimization here
    }
}
```

### 4. Transformation Layer

#### Tag Dropping Source
```rust
pub struct TagDroppingSource {
    source: Box<dyn MetricsSource>,
    drop_sets: Vec<DropSet>,
}

#[derive(Clone)]
pub struct DropSet {
    metric_pattern: Regex,
    remove_dimensions: Vec<String>,
}

impl TagDroppingSource {
    async fn process_record(&self, mut record: Record) -> Vec<Record> {
        let mut merged_metrics: HashMap<MetricKey, MergingMetric> = HashMap::new();
        
        for (metric_name, metric) in record.metrics {
            let dimensions_to_drop: Vec<String> = self.drop_sets
                .iter()
                .filter(|ds| ds.metric_pattern.is_match(&metric_name))
                .flat_map(|ds| &ds.remove_dimensions)
                .cloned()
                .collect();
                
            let modified_dimensions = self.remove_dimensions(
                &record.dimensions, 
                &dimensions_to_drop
            );
            
            let key = MetricKey::new(modified_dimensions, metric_name.clone());
            
            match merged_metrics.entry(key) {
                Entry::Vacant(e) => {
                    e.insert(MergingMetric::new(metric));
                }
                Entry::Occupied(mut e) => {
                    e.get_mut().merge(metric)?;
                }
            }
        }
        
        // Convert merged metrics back to records
        self.build_records_from_merged(merged_metrics, &record)
    }
}
```

#### Transforming Source
```rust
pub struct TransformingSource {
    source: Box<dyn MetricsSource>,
    transformations: Vec<TransformationSet>,
}

#[derive(Clone)]
pub struct TransformationSet {
    transform_metrics: HashMap<Regex, Vec<String>>,
    inject_dimensions: HashMap<String, DimensionInjection>,
    remove_dimensions: Vec<String>,
}

impl TransformingSource {
    async fn process_record(&self, record: Record) -> Vec<Record> {
        // Variable context for replacement
        let mut variables = HashMap::new();
        variables.insert("dimension".to_string(), record.dimensions.clone());
        variables.insert("env".to_string(), std::env::vars().collect());
        
        for transformation in &self.transformations {
            // Apply metric name transformations with regex replacement
            // Support dimension injection and removal
            // Handle tag parsing from metric names (semicolon-separated)
        }
        
        // Similar merging logic as TagDroppingSource but with transformations
    }
}
```

### 5. Sink Implementations

#### HTTP Sink
```rust
pub struct HttpSink {
    client: reqwest::Client,
    uri: Url,
    batch_size: usize,
    flush_interval: Duration,
}

impl HttpSink {
    async fn send_batch(&self, batch: Vec<AggregatedData>) -> Result<()> {
        let body = serde_json::to_vec(&batch)?;
        let response = self.client
            .post(self.uri.clone())
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(HttpSinkError::BadResponse(response.status()));
        }
        
        Ok(())
    }
}
```

#### Series Logging Sink
```rust
pub struct SeriesLoggingSink {
    metrics_map: Arc<Mutex<HashMap<String, Vec<HashMap<String, String>>>>>,
    current_time: Arc<Mutex<DateTime<Utc>>>,
    output_path: PathBuf,
}

impl SeriesLoggingSink {
    async fn record_aggregated_data(&self, data: &AggregatedData) -> Result<()> {
        let dimensions = data.dimensions.clone();
        
        // Track metric series for analysis
        let mut metrics = self.metrics_map.lock().await;
        for metric_name in data.metrics.keys() {
            metrics
                .entry(metric_name.clone())
                .or_insert_with(Vec::new)
                .push(dimensions.clone());
        }
        
        // Periodic dump to JSON file
        self.maybe_dump_series().await?;
        Ok(())
    }
    
    async fn maybe_dump_series(&self) -> Result<()> {
        let mut current_time = self.current_time.lock().await;
        let now = Utc::now();
        
        if now > *current_time {
            *current_time = now;
            
            let metrics = self.metrics_map.lock().await;
            let json = serde_json::to_string_pretty(&*metrics)?;
            
            tokio::fs::write(&self.output_path, json).await?;
            
            // Clear for next period
            drop(metrics);
            self.metrics_map.lock().await.clear();
        }
        
        Ok(())
    }
}
```

## Migration Strategy

### Phase 1: Core Foundation (Month 1-2)
**Goal**: Prove performance benefits with minimal feature set

```rust
// Deliverables
- [x] Basic configuration loading (HOCON/JSON)
- [x] HTTP source (V3 protocol only)
- [x] Histogram implementation with statistics
- [x] Telemetry sink
- [x] Single pipeline support
- [x] Performance benchmarks vs Java version
```

**Success Criteria**: 
- Measurable performance improvement on basic workload
- Reduced memory usage compared to Java version
- Faster startup time

### Phase 2: Protocol Parity (Month 2-3)
**Goal**: Support all existing client integrations

```rust
// Deliverables
- [x] All HTTP source versions (V1, V2, V3)
- [x] StatsD UDP source  
- [x] Prometheus source
- [x] OpenTelemetry gRPC source
- [x] Kafka source and sink
- [x] HTTP sinks (AggregationServer, custom)
- [x] Configuration compatibility layer
- [x] Multi-pipeline support
```

### Phase 3: Advanced Features (Month 3-4)
**Goal**: Feature parity for production deployment

```rust
// Deliverables
- [x] Dynamic configuration reloading
- [x] TLS/HTTPS support
- [x] Transformation layer (TagDroppingSource, TransformingSource)
- [x] Period filtering sinks
- [x] Series logging sink for analysis
- [x] Comprehensive metrics and tracing
- [x] Docker containerization
```

### Phase 4: Optimization & Deployment (Month 4-5)
**Goal**: Production-ready performance optimization

```rust
// Deliverables
- [x] SIMD optimizations for statistics
- [x] Memory pool optimizations
- [x] Benchmark suite and regression testing
- [x] Migration documentation
- [x] Deployment automation
```

## Configuration Compatibility

### Source Mapping Examples

```toml
# Old Java config
sources = [
  {
    type = "com.arpnetworking.metrics.common.sources.ClientHttpSourceV3"
    name = "http_v3_source"
  },
  {
    type = "com.arpnetworking.metrics.mad.experimental.sources.OpenTelemetryGrpcSource"
    name = "otel_grpc_source"
  },
  {
    type = "com.arpnetworking.metrics.mad.experimental.sources.TagDroppingSource"
    name = "tag_dropping_source"
    source = { ... }
    dropSets = [...]
  }
]

# New Rust config (internal representation)
[[sources]]
type = "http"
name = "http_v3_source"
version = "v3"
bind_address = "0.0.0.0:7090"

[[sources]]
type = "otel_grpc"
name = "otel_grpc_source"
bind_address = "0.0.0.0:4317"

[[sources]]
type = "tag_dropping"
name = "tag_dropping_source"
source = { ... }
drop_sets = [...]
```

### Sink Mapping Examples

```toml
# Old Java config  
sinks = [
  {
    type = "com.arpnetworking.tsdcore.sinks.AggregationServerHttpSink"
    name = "cluster_http_sink"
    uri = "http://localhost:7066/metrics/v1/data/persist"
  }
]

# New Rust config (internal representation)
[[sinks]]
type = "http"
name = "cluster_http_sink"
uri = "http://localhost:7066/metrics/v1/data/persist"
method = "POST"
```

## Performance Expectations

### Expected Improvements
- **Memory Usage**: Lower baseline memory usage without JVM overhead
- **Startup Time**: Elimination of JVM startup and warmup time
- **Request Latency**: More consistent latency without GC pauses
- **Resource Predictability**: More deterministic resource usage patterns
- **Container Efficiency**: Smaller container images and faster cold starts

### Benchmark Strategy
- Establish baseline performance metrics for current Java implementation
- Focus on real-world workloads rather than synthetic benchmarks
- Measure memory usage, CPU utilization, and latency under load
- Compare resource efficiency in containerized environments
- Validate performance claims with production-like data volumes

## Risk Assessment

### Technical Risks
- **Learning Curve**: Team familiarity with Rust ecosystem
- **Library Maturity**: Some protocol libraries may be less mature
- **Complex Migration**: Ensuring exact compatibility with edge cases

### Mitigation Strategies
- **Parallel Development**: Keep Java version running during transition
- **Extensive Testing**: Comprehensive test suite including protocol compliance
- **Gradual Migration**: Support both versions during transition period
- **Expert Consultation**: Rust experts for performance optimization

## Success Criteria

### Performance Metrics
- [ ] Measurable reduction in memory usage vs Java version
- [ ] Faster startup time (target: sub-second)
- [ ] Consistent request latency without GC spikes
- [ ] Maintain or improve throughput of Java version

### Compatibility Metrics  
- [ ] 100% existing configuration files work unchanged
- [ ] All existing client integrations continue working
- [ ] Identical aggregation results vs Java version
- [ ] Zero-downtime migration path available

### Operational Metrics
- [ ] Single binary deployment working
- [ ] Smaller container images than Java version
- [ ] Production monitoring and alerting integrated
- [ ] Documentation and runbooks complete

This plan provides a clear roadmap for migrating MAD to Rust while maintaining full compatibility for existing users and delivering significant performance improvements.