# Huawei ONT Exporter

A Rust-based Prometheus exporter for Huawei ONT devices (specifically modeled for HG8145V5 and similar).

## Features

- Scrapes optical metrics:
  - TX Optical Power (dBm)
  - RX Optical Power (dBm)
  - Working Voltage (mV)
  - Bias Current (mA)
  - Working Temperature (°C)
- Handles authentication flow:
  - Fetches login token from `/asp/GetRandCount.asp`
  - Logs in via `/login.cgi`
  - Scrapes `/html/amp/opticinfo/opticinfo.asp`
  - Logs out immediately
- Exposes Prometheus metrics at `/metrics`
- Health check at `/health`

## Usage

### Build

```bash
cd huawei-ont-exporter
cargo build --release
```

### Run

Required environment variables:
- `ONT_URL` - URL of your ONT device (e.g., `http://192.168.100.1`)
- `ONT_USER` - Username for authentication
- `ONT_PASS` - Password for authentication

Optional environment variables:
- `SCRAPE_INTERVAL` - Scrape interval in seconds (default: 30)
- `RUST_LOG` - Log level (default: info, use `debug` for more verbose output)

Example:

```bash
export ONT_URL="http://your-ont-ip"
export ONT_USER="your-username"
export ONT_PASS="your-password"
export SCRAPE_INTERVAL="30"

./target/release/huawei_ont_exporter
```

### Metrics

Access metrics at `http://localhost:8000/metrics`.

Example output:
```
# HELP huawei_ont_bias_current_ma Bias current in mA
# TYPE huawei_ont_bias_current_ma gauge
huawei_ont_bias_current_ma 10
# HELP huawei_ont_optical_rx_power_dbm Receive optical power in dBm
# TYPE huawei_ont_optical_rx_power_dbm gauge
huawei_ont_optical_rx_power_dbm -24.09
# HELP huawei_ont_optical_tx_power_dbm Transmit optical power in dBm
# TYPE huawei_ont_optical_tx_power_dbm gauge
huawei_ont_optical_tx_power_dbm 2.33
# HELP huawei_ont_working_temperature_celsius Working temperature in Celsius
# TYPE huawei_ont_working_temperature_celsius gauge
huawei_ont_working_temperature_celsius 47
# HELP huawei_ont_working_voltage_mv Working voltage in mV
# TYPE huawei_ont_working_voltage_mv gauge
huawei_ont_working_voltage_mv 3364
```

## License

Apache-2.0

