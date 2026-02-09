# AGENTS.md - Development Guidelines for Huawei ONT Exporter

## Project Overview

A Rust-based Prometheus exporter for Huawei ONT (Optical Network Terminal) devices. Scrapes optical metrics and exposes them via HTTP.

## Technical Stack

- **Language**: Rust (Edition 2024)
- **Web Framework**: Actix-web
- **Metrics**: Prometheus client library
- **HTTP Client**: Reqwest
- **Logging**: Tracing (JSON format)
- **CI/CD**: GitHub Actions
- **Dependency Management**: Renovate Bot

## Development Workflow

### 1. Build and Test Locally

```bash
# Development build
cargo build

# Run tests
cargo test

# Run clippy (must pass with no warnings)
cargo clippy -- -D warnings

# Release build
cargo build --release

# Run locally (requires env vars)
export ONT_URL="http://your-ont-ip"
export ONT_USER="your-username"
export ONT_PASS="your-password"
./target/release/huawei_ont_exporter
```

### 2. CI/CD Pipeline

The CI runs on every push/PR to `main`:

1. **Build Job**:
   - `cargo build --verbose`
   - `cargo test --verbose`
   - `cargo clippy -- -D warnings`
   - Uses cargo caching for speed

2. **Smoke Test Job**:
   - Builds release binary
   - Starts server with dummy env vars
   - Tests `/health` endpoint (should return 200)
   - Tests `/metrics` endpoint (should return 200 with `huawei_ont_` metrics)
   - Depends on build job

### 3. Caching Strategy

Cargo caching is split into 3 separate caches:
- **Registry**: `~/.cargo/registry` (crate downloads)
- **Git Index**: `~/.cargo/git` (git dependencies)
- **Build**: `target/` (compiled artifacts)

Cache keys include `Cargo.lock` and `Cargo.toml` hashes for stability.

### 4. Dependency Updates

**Renovate** handles dependency updates automatically:
- Runs on all PRs
- Automerges minor/patch updates if CI passes
- Requires manual review for major version bumps
- Creates Dependency Dashboard issue to track updates

**To run Renovate locally** (for testing):
```bash
RENOVATE_TOKEN="$(gh auth token)" bunx renovate --platform=github anakaiti/huawei-ont-exporter
```

### 5. Security

- **No hardcoded secrets** - All credentials via environment variables
- **Cargo.lock committed** - For reproducible builds
- **Dependabot alerts** - Monitor for security vulnerabilities
- **gitleaks** - Run before commits to catch secrets

**Environment Variables Required:**
- `ONT_URL` - ONT device URL
- `ONT_USER` - Username
- `ONT_PASS` - Password
- `SCRAPE_INTERVAL` - Optional, default 30s
- `RUST_LOG` - Optional, log level (default: info)

## Code Style Guidelines

### Logging
- Use `tracing` crate with JSON output
- Log levels:
  - `error!` - Failures and errors
  - `info!` - Startup info, configuration
  - `debug!` - Operational details (scraping, login, etc.)
- Hide successful operations under `debug!` level
- Always log failures

### Metrics
All metrics prefixed with `huawei_ont_`:

**ONT Metrics:**
- `huawei_ont_optical_tx_power_dbm` - TX power
- `huawei_ont_optical_rx_power_dbm` - RX power
- `huawei_ont_working_voltage_mv` - Voltage
- `huawei_ont_bias_current_ma` - Bias current
- `huawei_ont_working_temperature_celsius` - Temperature

**Operational Metrics:**
- `huawei_ont_scrapes_total` - Total scrape attempts
- `huawei_ont_scrape_errors_total` - Failed scrapes
- `huawei_ont_scrape_duration_seconds` - Histogram of scrape times
- `huawei_ont_http_requests_total` - HTTP requests served
- `huawei_ont_http_requests_errors_total` - HTTP request errors

### Error Handling
- Use `anyhow` for error propagation
- Use `context()` for adding context to errors
- Fail fast on startup if required env vars missing

## Common Tasks

### Adding a New Metric

1. Add gauge/counter/histogram to `src/metrics.rs` using `lazy_static!`
2. Update `update_metrics()` function to set the value
3. Scrape the value in `src/client.rs` and return in `OntMetrics`
4. Update README.md with new metric documentation

### Updating Dependencies

1. Check Renovate PRs - review and merge if CI passes
2. For major version bumps:
   - Check breaking changes in changelog
   - Test locally before merging
   - Update code if API changes
3. Run `cargo update` to refresh lockfile
4. Commit `Cargo.lock` changes

### Debugging CI Issues

1. Check GitHub Actions logs for failure reason
2. Reproduce locally if possible
3. Common issues:
   - Cache miss - First build after dependency update
   - clippy warnings - Must fix all warnings
   - Smoke test failure - Server didn't start or /metrics broken

## Deployment

### Docker (Future)
```dockerfile
FROM rust:1-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/huawei_ont_exporter /usr/local/bin/
EXPOSE 8000
ENTRYPOINT ["huawei_ont_exporter"]
```

### Systemd Service (Example)
```ini
[Unit]
Description=Huawei ONT Exporter
After=network.target

[Service]
Type=simple
Environment="ONT_URL=http://192.168.100.1"
Environment="ONT_USER=root"
Environment="ONT_PASS=CHANGEME"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/huawei_ont_exporter
Restart=always

[Install]
WantedBy=multi-user.target
```

## Release Checklist

- [ ] All tests passing
- [ ] Clippy clean (no warnings)
- [ ] Smoke tests pass
- [ ] Version bumped in `Cargo.toml`
- [ ] `Cargo.lock` committed
- [ ] README.md updated
- [ ] CHANGELOG.md updated (if exists)
- [ ] Tag created: `git tag -a v0.x.0 -m "Release v0.x.0"`
- [ ] Secrets scanner passed (gitleaks)

## Repository URLs

- **GitHub**: https://github.com/anakaiti/huawei-ont-exporter
- **Metrics Endpoint**: `http://localhost:8000/metrics`
- **Health Endpoint**: `http://localhost:8000/health`

## License

Apache-2.0
