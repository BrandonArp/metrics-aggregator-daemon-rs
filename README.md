# Metrics Aggregator Daemon - Rust

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A high-performance, low-resource implementation of the Metrics Aggregator Daemon (MAD) written in Rust.

## Overview

This is a Rust rewrite of the [Java Metrics Aggregator Daemon](https://github.com/ArpNetworking/metrics-aggregator-daemon), designed to provide:

- **Reduced Memory Footprint**: Elimination of JVM overhead
- **Faster Startup**: Native binary with sub-second startup time  
- **Consistent Performance**: No garbage collection pauses
- **Single Binary Deployment**: No runtime dependencies
- **Full Compatibility**: Drop-in replacement with identical configuration format

## Quick Start

### Prerequisites

- Rust 1.70+ (`rustup` recommended)
- For development: `cargo`, `git`

### Installation

```bash
# Clone the repository
git clone https://github.com/BrandonArp/metrics-aggregator-daemon-rs.git
cd metrics-aggregator-daemon-rs

# Build release binary
cargo build --release

# Run with configuration
./target/release/mad config/config.conf
```

### Docker

```bash
# Build Docker image
docker build -t mad-rs .

# Run with configuration
docker run -v $(pwd)/config:/config mad-rs /config/config.conf
```

## Migration from Java Version

This Rust implementation maintains **100% configuration compatibility** with the Java version. Existing configuration files work unchanged.

See [RUST_PORT_PLAN.md](RUST_PORT_PLAN.md) for detailed migration guidance and feature roadmap.

### Configuration Compatibility

```hocon
# Your existing Java MAD configuration works as-is
pipelinesDirectory = "config/pipelines"
httpHost = "0.0.0.0"
httpPort = 7090

sources = [
  {
    type = "com.arpnetworking.metrics.common.sources.ClientHttpSourceV3"
    name = "http_v3_source"
  }
]

sinks = [
  {
    type = "com.arpnetworking.tsdcore.sinks.TelemetrySink"
    name = "telemetry_sink"
  }
]
```

## Features

### Supported Sources
- [x] HTTP Sources (V1, V2, V3)
- [x] StatsD UDP Source
- [ ] Prometheus Source  
- [ ] Kafka Source
- [ ] OpenTelemetry gRPC Source *(experimental)*

### Supported Sinks
- [x] Telemetry Sink
- [ ] HTTP Sinks
- [ ] Kafka Sink
- [ ] Series Logging Sink *(experimental)*

### Advanced Features
- [ ] Metric Transformations *(experimental)*
- [ ] Tag Dropping *(experimental)*
- [ ] Dynamic Configuration Reloading
- [ ] TLS/HTTPS Support

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Architecture

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library root
├── config/              # Configuration parsing
├── sources/             # Metric input sources
├── sinks/               # Metric output sinks  
├── aggregation/         # Time-based aggregation engine
├── model/              # Core data structures
└── transformations/    # Metric transformation layer
```

### Performance Testing

```bash
# Run aggregation benchmarks
cargo bench histogram_performance

# Compare with Java version
./scripts/benchmark_comparison.sh
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following Rust conventions
4. Add tests for new functionality
5. Run the test suite (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Code Style

This project uses standard Rust formatting:

```bash
# Format code
cargo fmt

# Check for common issues
cargo clippy
```

## Roadmap

See [RUST_PORT_PLAN.md](RUST_PORT_PLAN.md) for the complete development roadmap and migration strategy.

### Current Status: Phase 1 - Core Foundation
- [x] Project setup and configuration
- [x] Core data structures
- [ ] Basic HTTP source
- [ ] Histogram implementation
- [ ] Telemetry sink
- [ ] Single pipeline support

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- Original Java implementation: [ArpNetworking MAD](https://github.com/ArpNetworking/metrics-aggregator-daemon)
- Built with the excellent Rust ecosystem: Tokio, Axum, Serde, and many others