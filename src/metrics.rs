use crate::parser::OntMetrics;
use lazy_static::lazy_static;
use prometheus::{register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram};

lazy_static! {
    // ONT Metrics
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
    TX_POWER.set(data.tx_power);
    RX_POWER.set(data.rx_power);
    VOLTAGE.set(data.voltage);
    BIAS_CURRENT.set(data.bias_current);
    TEMPERATURE.set(data.temperature);
}
