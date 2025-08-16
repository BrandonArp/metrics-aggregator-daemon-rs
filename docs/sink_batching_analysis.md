# Sink Request Batching and Data Merging Analysis

## Executive Summary

This document analyzes approaches to implement intelligent request batching and data merging in MAD sinks to address the fundamental performance bottleneck where each aggregated data object becomes a separate request. The proposed solutions can provide 10-100x improvement in throughput while maintaining data integrity.

## Current Problem Analysis

### Java Implementation Issues

```java
// Current problematic pattern
for (AggregatedData data : aggregatedDataList) {
    HttpRequest request = serialize(data);  // 1 request per data object
    requestQueue.add(request);              // Queue fills with many small requests
}
// Result: High request volume, poor network utilization, queue pressure
```

### Performance Impact

- **Request Volume**: 1,000 metrics/second = 1,000 HTTP requests/second
- **Network Overhead**: HTTP headers (~500 bytes) + connection overhead per request  
- **Queue Memory**: Each request object has significant overhead
- **Sink Performance**: Many small requests vs fewer large requests
- **Error Handling**: Complex error tracking per individual request

## Design Goals

### Primary Goals
1. **Batch Multiple Data Points**: Combine multiple `AggregatedData` into single requests
2. **Smart Merging**: Merge compatible data when possible (same dimensions, time windows)
3. **Configurable Behavior**: Per-sink batching strategies
4. **Maintain Correctness**: No data loss, proper error handling
5. **Flexible Architecture**: Support both batching and non-batching sinks

### Performance Targets
- **10-100x** reduction in request volume
- **50%+** improvement in network utilization
- **Configurable** batch sizes and timeouts per sink
- **Sub-second** batch flush latencies

## Proposed Architectures

### Option 1: Trait-Based Batching with Capability Detection

```rust
pub trait MetricsSink: Send + Sync {
    /// Send single data point (legacy compatibility)
    async fn send(&self, data: &AggregatedData) -> Result<()>;
    
    /// Send batch of data points (optional, high-performance)
    async fn send_batch(&self, data: Vec<AggregatedData>) -> Result<()> {
        // Default: send individually
        for item in data {
            self.send(&item).await?;
        }
        Ok(())
    }
    
    /// Merge compatible data points (optional, advanced optimization)
    fn can_merge(&self, a: &AggregatedData, b: &AggregatedData) -> bool {
        false // Default: no merging
    }
    
    fn merge(&self, data: Vec<AggregatedData>) -> Result<Vec<AggregatedData>> {
        Ok(data) // Default: no-op
    }
    
    /// Batching configuration
    fn batch_config(&self) -> BatchConfig {
        BatchConfig::disabled()
    }
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait_time: Duration,
    pub enable_merging: bool,
}
```

**Pros:**
- Backward compatible (sinks can implement only `send`)
- Flexible per-sink batching behavior
- Gradual adoption path

**Cons:**  
- Complex trait with many optional methods
- Runtime capability detection needed

### Option 2: Separate Batching and Non-Batching Traits

```rust
pub trait MetricsSink: Send + Sync {
    async fn send(&self, data: &AggregatedData) -> Result<()>;
    async fn flush(&self) -> Result<()>;
    async fn close(&self) -> Result<()>;
    fn name(&self) -> &str;
}

pub trait BatchingMetricsSink: MetricsSink {
    async fn send_batch(&self, data: Vec<AggregatedData>) -> Result<()>;
    fn batch_config(&self) -> BatchConfig;
}

pub trait MergingMetricsSink: BatchingMetricsSink {
    fn can_merge(&self, a: &AggregatedData, b: &AggregatedData) -> bool;
    fn merge(&self, data: Vec<AggregatedData>) -> Result<Vec<AggregatedData>>;
}
```

**Pros:**
- Clear capability separation
- Type-safe compile-time checks
- Simpler individual traits

**Cons:**
- Multiple trait implementations required
- Dynamic dispatch complexity

### Option 3: Strategy Pattern with Sink Adapters

```rust
pub trait MetricsSink: Send + Sync {
    async fn send(&self, data: &AggregatedData) -> Result<()>;
    // ... standard methods
}

pub struct BatchingSinkAdapter<T: MetricsSink> {
    sink: T,
    config: BatchConfig,
    buffer: Vec<AggregatedData>,
    last_flush: Instant,
}

impl<T: MetricsSink> BatchingSinkAdapter<T> {
    pub fn new(sink: T, config: BatchConfig) -> Self { ... }
    
    async fn maybe_flush(&mut self) -> Result<()> {
        if self.should_flush() {
            self.flush_batch().await?;
        }
        Ok(())
    }
    
    fn should_flush(&self) -> bool {
        self.buffer.len() >= self.config.max_batch_size ||
        self.last_flush.elapsed() >= self.config.max_wait_time
    }
}
```

**Pros:**
- Keeps core sink trait simple
- Composable behavior (can wrap any sink)
- Clear separation of concerns

**Cons:**
- Extra wrapper layer
- Potential double-buffering

### Option 4: Queue-Level Batching with Smart Routing

```rust
pub struct SinkQueue {
    pending: Vec<AggregatedData>,
    sink_configs: HashMap<String, BatchConfig>,
    flush_timers: HashMap<String, Instant>,
}

impl SinkQueue {
    async fn enqueue(&mut self, data: AggregatedData, sink_name: &str) -> Result<()> {
        self.pending.push(data);
        
        if self.should_flush_for_sink(sink_name) {
            let batch = self.extract_batch_for_sink(sink_name);
            self.send_to_sink(sink_name, batch).await?;
        }
        
        Ok(())
    }
    
    fn extract_batch_for_sink(&mut self, sink_name: &str) -> Vec<AggregatedData> {
        // Smart extraction: group by sink, merge compatible items
        // Return batch ready for sink
    }
}
```

**Pros:**
- Centralized batching logic
- Cross-sink optimization opportunities
- Single buffer management

**Cons:**
- Complex queue logic
- Tight coupling between queue and sink behavior

## Detailed Design Recommendation

### Recommended Approach: Hybrid of Options 1 + 3

Combine trait-based capabilities with adapter pattern:

```rust
// Core trait - simple, focused
pub trait MetricsSink: Send + Sync {
    async fn send(&self, data: &AggregatedData) -> Result<()>;
    async fn flush(&self) -> Result<()>;
    async fn close(&self) -> Result<()>;
    fn name(&self) -> &str;
}

// Optional batching capability
pub trait BatchCapable {
    async fn send_batch(&self, data: Vec<AggregatedData>) -> Result<()>;
    fn batch_config(&self) -> BatchConfig;
}

// Optional merging capability  
pub trait MergeCapable {
    fn can_merge(&self, a: &AggregatedData, b: &AggregatedData) -> bool;
    fn merge(&self, data: Vec<AggregatedData>) -> Result<Vec<AggregatedData>>;
}

// Smart adapter that detects capabilities
pub struct SmartSinkAdapter {
    sink: Box<dyn MetricsSink>,
    batcher: Option<BatchProcessor>,
    merger: Option<MergeProcessor>,
}

impl SmartSinkAdapter {
    pub fn new(sink: Box<dyn MetricsSink>) -> Self {
        let batcher = if sink.as_any().downcast_ref::<dyn BatchCapable>().is_some() {
            Some(BatchProcessor::new(/* config */))
        } else {
            None
        };
        
        // Similar logic for merger...
        
        Self { sink, batcher, merger }
    }
}
```

### Batching Strategies

#### Time-Based Batching
```rust
pub struct TimeBatcher {
    max_wait_time: Duration,
    buffer: Vec<AggregatedData>,
    last_flush: Instant,
}

impl TimeBatcher {
    async fn add(&mut self, data: AggregatedData) -> Option<Vec<AggregatedData>> {
        self.buffer.push(data);
        
        if self.last_flush.elapsed() >= self.max_wait_time {
            Some(self.flush())
        } else {
            None
        }
    }
}
```

#### Size-Based Batching
```rust
pub struct SizeBatcher {
    max_batch_size: usize,
    buffer: Vec<AggregatedData>,
}

impl SizeBatcher {
    async fn add(&mut self, data: AggregatedData) -> Option<Vec<AggregatedData>> {
        self.buffer.push(data);
        
        if self.buffer.len() >= self.max_batch_size {
            Some(self.flush())
        } else {
            None
        }
    }
}
```

#### Hybrid Batching (Recommended)
```rust
pub struct HybridBatcher {
    max_batch_size: usize,
    max_wait_time: Duration,
    buffer: Vec<AggregatedData>,
    last_flush: Instant,
    flush_timer: Option<tokio::time::Sleep>,
}

impl HybridBatcher {
    async fn add(&mut self, data: AggregatedData) -> Option<Vec<AggregatedData>> {
        self.buffer.push(data);
        
        // Size-based flush
        if self.buffer.len() >= self.max_batch_size {
            return Some(self.flush());
        }
        
        // Time-based flush (set timer if not set)
        if self.flush_timer.is_none() {
            self.flush_timer = Some(tokio::time::sleep(self.max_wait_time));
        }
        
        None
    }
    
    async fn check_timer(&mut self) -> Option<Vec<AggregatedData>> {
        if let Some(timer) = &mut self.flush_timer {
            if timer.is_elapsed() {
                self.flush_timer = None;
                return Some(self.flush());
            }
        }
        None
    }
}
```

### Data Merging Strategies

#### Dimension-Based Merging
```rust
impl MergeCapable for HttpSink {
    fn can_merge(&self, a: &AggregatedData, b: &AggregatedData) -> bool {
        // Same dimensions and overlapping time windows
        a.dimensions == b.dimensions &&
        a.start_time <= b.end_time &&
        b.start_time <= a.end_time
    }
    
    fn merge(&self, mut data: Vec<AggregatedData>) -> Result<Vec<AggregatedData>> {
        // Group by dimensions
        let mut groups: HashMap<&HashMap<String, String>, Vec<AggregatedData>> = HashMap::new();
        
        for item in data {
            groups.entry(&item.dimensions).or_default().push(item);
        }
        
        let mut result = Vec::new();
        for (_, group) in groups {
            if group.len() == 1 {
                result.extend(group);
            } else {
                result.push(self.merge_group(group)?);
            }
        }
        
        Ok(result)
    }
    
    fn merge_group(&self, group: Vec<AggregatedData>) -> Result<AggregatedData> {
        // Merge metrics with same names, combine time ranges
        // Complex logic depending on metric types and statistics
    }
}
```

#### Time-Window Merging
```rust
fn merge_time_windows(data: Vec<AggregatedData>) -> Result<Vec<AggregatedData>> {
    // Sort by start time
    let mut sorted = data;
    sorted.sort_by_key(|d| d.start_time);
    
    let mut result = Vec::new();
    let mut current: Option<AggregatedData> = None;
    
    for item in sorted {
        match &current {
            None => current = Some(item),
            Some(curr) if can_merge_time_windows(curr, &item) => {
                current = Some(merge_adjacent_windows(curr.clone(), item)?);
            }
            Some(curr) => {
                result.push(curr.clone());
                current = Some(item);
            }
        }
    }
    
    if let Some(last) = current {
        result.push(last);
    }
    
    Ok(result)
}
```

## Configuration Design

### Per-Sink Configuration
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SinkConfig {
    pub name: String,
    pub sink_type: String,
    
    // Batching configuration
    pub batching: Option<BatchingConfig>,
    
    // Sink-specific config
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchingConfig {
    pub enabled: bool,
    pub max_batch_size: usize,
    pub max_wait_time_ms: u64,
    pub enable_merging: bool,
    pub merge_strategy: MergeStrategy,
}

#[derive(Debug, Clone, Deserialize)]
pub enum MergeStrategy {
    None,
    SameDimensions,
    TimeWindows,
    DimensionsAndTime,
    Custom(String), // Custom merge function name
}
```

### Example Configuration
```json
{
  "sinks": [
    {
      "name": "http_sink",
      "type": "http",
      "batching": {
        "enabled": true,
        "max_batch_size": 100,
        "max_wait_time_ms": 1000,
        "enable_merging": true,
        "merge_strategy": "DimensionsAndTime"
      },
      "config": {
        "uri": "http://localhost:8080/metrics",
        "timeout_ms": 5000
      }
    },
    {
      "name": "file_sink", 
      "type": "file",
      "batching": {
        "enabled": false
      },
      "config": {
        "path": "/var/log/metrics.log"
      }
    }
  ]
}
```

## Implementation Phases

### Phase 1: Basic Batching Framework
- [ ] Implement core batching traits
- [ ] Create hybrid batcher (size + time)
- [ ] Add basic batch configuration
- [ ] Simple HTTP sink with batching

### Phase 2: Advanced Merging
- [ ] Implement merging capabilities
- [ ] Dimension-based merging logic
- [ ] Time-window merging logic
- [ ] Configurable merge strategies

### Phase 3: Performance Optimization
- [ ] Memory pool for batch buffers
- [ ] Zero-copy serialization where possible
- [ ] SIMD optimizations for merging
- [ ] Comprehensive benchmarking

### Phase 4: Production Features
- [ ] Partial batch failure handling
- [ ] Metrics and monitoring for batching
- [ ] Dynamic configuration updates
- [ ] Advanced merge strategies

## Error Handling Design

### Batch Failure Scenarios
```rust
pub enum BatchError {
    PartialFailure {
        succeeded: Vec<AggregatedData>,
        failed: Vec<(AggregatedData, Error)>,
    },
    CompleteFailure(Error),
    MergeError {
        original_data: Vec<AggregatedData>,
        error: Error,
    },
}

impl BatchingSink {
    async fn handle_batch_error(&self, error: BatchError) -> Result<()> {
        match error {
            BatchError::PartialFailure { failed, .. } => {
                // Retry failed items individually or re-batch
                for (data, _) in failed {
                    self.retry_single(data).await?;
                }
            }
            BatchError::CompleteFailure(e) => {
                // Retry entire batch or fall back to individual sends
                return Err(e);
            }
            BatchError::MergeError { original_data, .. } => {
                // Fall back to sending original data without merging
                self.send_batch_without_merge(original_data).await?;
            }
        }
        Ok(())
    }
}
```

## Performance Projections

### Expected Improvements

| Metric | Current | With Batching | Improvement |
|--------|---------|---------------|-------------|
| Requests/sec | 1,000 | 10-50 | **20-100x** |
| Network Utilization | 30% | 80%+ | **2.5x+** |
| Queue Memory | High | Low | **5-10x** |
| CPU Overhead | High | Low | **3-5x** |
| Error Handling | Complex | Simplified | **Qualitative** |

### Batching Effectiveness by Sink Type

| Sink Type | Batch Size | Merge Potential | Expected Gain |
|-----------|------------|-----------------|---------------|
| HTTP JSON | 50-200 | High | **50-100x** |
| Kafka | 100-1000 | Medium | **20-50x** |
| File/Log | 10-50 | Low | **5-20x** |
| Database | 100-500 | High | **30-100x** |

## Risks and Mitigations

### Risk: Data Loss on Batch Failure
**Mitigation**: Implement robust error handling with individual fallback

### Risk: Increased Latency Due to Batching
**Mitigation**: Configurable time-based flush limits, monitoring

### Risk: Memory Usage from Buffering  
**Mitigation**: Configurable buffer limits, memory pools

### Risk: Complex Merge Logic Bugs
**Mitigation**: Extensive testing, optional merging, fallback to simple batching

## Conclusion

Implementing intelligent sink batching and merging addresses a critical performance bottleneck in the Java implementation. The hybrid approach provides:

1. **Backward Compatibility** via simple sink trait
2. **High Performance** via optional batching and merging
3. **Flexibility** via per-sink configuration
4. **Correctness** via robust error handling

This optimization can provide **10-100x improvement** in request throughput while maintaining data integrity, making it a foundational improvement for the Rust MAD implementation.