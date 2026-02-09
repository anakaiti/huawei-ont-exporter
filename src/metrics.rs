use crate::parser::OntMetrics;
use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_histogram, register_int_gauge_vec, Counter, Gauge,
    Histogram, IntGaugeVec, Opts,
};

lazy_static! {
    // ONT Optical Metrics
    pub static ref TX_POWER: Gauge = register_gauge!(
        "huawei_ont_optical_tx_power_dbm",
        "Transmit optical power in dBm"
    )
    .expect("metric registration failed");
    pub static ref RX_POWER: Gauge = register_gauge!(
        "huawei_ont_optical_rx_power_dbm",
        "Receive optical power in dBm"
    )
    .expect("metric registration failed");
    pub static ref VOLTAGE: Gauge =
        register_gauge!("huawei_ont_working_voltage_mv", "Working voltage in mV")
            .expect("metric registration failed");
    pub static ref BIAS_CURRENT: Gauge =
        register_gauge!("huawei_ont_bias_current_ma", "Bias current in mA")
            .expect("metric registration failed");
    pub static ref TEMPERATURE: Gauge = register_gauge!(
        "huawei_ont_working_temperature_celsius",
        "Working temperature in Celsius"
    )
    .expect("metric registration failed");

    // Device Info Metrics (using labels - always value 1)
    pub static ref DEVICE_INFO: IntGaugeVec = register_int_gauge_vec!(
        Opts::new("huawei_ont_device_info", "Device information (always 1)"),
        &["model", "serial", "version"]
    )
    .expect("metric registration failed");

    pub static ref UPTIME: Gauge = register_gauge!(
        "huawei_ont_uptime_seconds",
        "Device uptime in seconds"
    )
    .expect("metric registration failed");

    // WAN Metrics
    pub static ref WAN_STATUS: Gauge = register_gauge!(
        Opts::new("huawei_ont_wan_status", "WAN connection status (1=up, 0=down)")
            .const_label("ip", "unknown")
    )
    .expect("metric registration failed");

    pub static ref WAN_RX_BYTES: Gauge = register_gauge!(
        "huawei_ont_wan_rx_bytes",
        "Total WAN bytes received"
    )
    .expect("metric registration failed");

    pub static ref WAN_TX_BYTES: Gauge = register_gauge!(
        "huawei_ont_wan_tx_bytes",
        "Total WAN bytes transmitted"
    )
    .expect("metric registration failed");

    // Client Metrics
    pub static ref LAN_CLIENTS: Gauge = register_gauge!(
        "huawei_ont_lan_clients",
        "Number of connected LAN clients"
    )
    .expect("metric registration failed");

    pub static ref WIFI_CLIENTS: Gauge = register_gauge!(
        "huawei_ont_wifi_clients",
        "Number of connected WiFi clients"
    )
    .expect("metric registration failed");

    // Scrape Metrics
    pub static ref SCRAPE_DURATION: Histogram = register_histogram!(
        "huawei_ont_scrape_duration_seconds",
        "Duration of ONT scrape in seconds",
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
    )
    .expect("metric registration failed");
    pub static ref SCRAPE_ERRORS: Counter = register_counter!(
        "huawei_ont_scrape_errors_total",
        "Total number of scrape errors"
    )
    .expect("metric registration failed");
    pub static ref SCRAPES_TOTAL: Counter = register_counter!(
        "huawei_ont_scrapes_total",
        "Total number of scrapes attempted"
    )
    .expect("metric registration failed");

    // HTTP Server Metrics
    pub static ref HTTP_REQUESTS_TOTAL: Counter = register_counter!(
        "huawei_ont_http_requests_total",
        "Total number of HTTP requests"
    )
    .expect("metric registration failed");
    pub static ref HTTP_REQUESTS_ERRORS: Counter = register_counter!(
        "huawei_ont_http_requests_errors_total",
        "Total number of HTTP request errors"
    )
    .expect("metric registration failed");
}

pub fn update_metrics(data: &OntMetrics) {
    // Optical metrics (always present)
    TX_POWER.set(data.tx_power);
    RX_POWER.set(data.rx_power);
    VOLTAGE.set(data.voltage);
    BIAS_CURRENT.set(data.bias_current);
    TEMPERATURE.set(data.temperature);

    // Device info metrics with labels
    let model = data.device_model.as_deref().unwrap_or("unknown");
    let serial = data.serial_number.as_deref().unwrap_or("unknown");
    let version = data.software_version.as_deref().unwrap_or("unknown");
    DEVICE_INFO
        .with_label_values(&[model, serial, version])
        .set(1);

    // Uptime metric
    if let Some(uptime) = data.uptime_seconds {
        UPTIME.set(uptime as f64);
    }

    // WAN metrics (optional)
    if let Some(status) = &data.wan_status {
        let status_value = if status.eq_ignore_ascii_case("up")
            || status.eq_ignore_ascii_case("connected")
            || status.eq_ignore_ascii_case("online")
        {
            1.0
        } else {
            0.0
        };
        WAN_STATUS.set(status_value);
    }

    if let Some(rx_bytes) = data.wan_rx_bytes {
        WAN_RX_BYTES.set(rx_bytes as f64);
    }

    if let Some(tx_bytes) = data.wan_tx_bytes {
        WAN_TX_BYTES.set(tx_bytes as f64);
    }

    // Client metrics (optional)
    if let Some(lan_count) = data.lan_clients_count {
        LAN_CLIENTS.set(lan_count as f64);
    }

    if let Some(wifi_count) = data.wifi_clients_count {
        WIFI_CLIENTS.set(wifi_count as f64);
    }
}
