use serde::Deserialize;
use std::collections::HashMap;
use toml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CfgGainSweepStrategy {
    #[default]
    Grid,
    CoordinateDescent,
}

#[derive(Debug, Clone)]
pub struct CfgGainRange {
    pub from: f64,
    pub to: f64,
    pub step: f64,
}

#[derive(Debug, Clone)]
pub struct CfgRxGainSweep {
    pub enabled: bool,
    pub strategy: CfgGainSweepStrategy,
    pub window_bursts: u32,
    pub settling_slots: u32,
    pub auto_exit: bool,
    pub restart_process_per_combo: bool,
    pub test_signal_profile: Option<String>,
    pub required_ul_slots: Vec<u8>,
    pub min_slot_crc_pass_rate: Option<f64>,
    pub test_device_type: Option<String>,
    pub test_tx_power_dbm: Option<f64>,
    pub test_level_dbm: Option<f64>,
    pub gains: HashMap<String, CfgGainRange>,
}

/// SoapySDR configuration
#[derive(Debug, Clone)]
pub struct CfgSoapySdr {
    /// Uplink frequency in Hz
    pub ul_freq: f64,
    /// Downlink frequency in Hz
    pub dl_freq: f64,
    /// PPM frequency error correction
    pub ppm_err: f64,
    /// Argument string to select a specific SDR device.
    /// If None, devices will be enumerated until the first supported device is found.
    pub device: Option<String>,
    /// RX antenna. Device specific default will be used if None.
    pub rx_ant: Option<String>,
    /// TX antenna. Device specific default will be used if None.
    pub tx_ant: Option<String>,
    /// RX gain values.
    /// Device specific defaults will be used for gains that are not set.
    pub rx_gains: HashMap<String, f64>,
    /// TX gain values.
    /// Device specific defaults will be used for gains that are not set.
    pub tx_gains: HashMap<String, f64>,
    /// RX and TX sample rate. Device specific default will be used if None.
    pub fs: Option<f64>,
    /// RX channel number
    pub rx_ch: Option<usize>,
    /// TX channel number
    pub tx_ch: Option<usize>,
    /// Optional autonomous RX gain sweep test configuration.
    pub rx_gain_sweep: Option<CfgRxGainSweep>,
}

impl CfgSoapySdr {
    /// Get corrected UL frequency with PPM error applied
    pub fn ul_freq_corrected(&self) -> (f64, f64) {
        let ppm = self.ppm_err;
        let err = (self.ul_freq / 1_000_000.0) * ppm;
        (self.ul_freq + err, err)
    }

    /// Get corrected DL frequency with PPM error applied
    pub fn dl_freq_corrected(&self) -> (f64, f64) {
        let ppm = self.ppm_err;
        let err = (self.dl_freq / 1_000_000.0) * ppm;
        (self.dl_freq + err, err)
    }
}

#[derive(Deserialize)]
pub struct SoapySdrDto {
    pub rx_freq: f64,
    pub tx_freq: f64,
    pub ppm_err: Option<f64>,

    pub device: Option<String>,

    pub rx_antenna: Option<String>,
    pub tx_antenna: Option<String>,

    pub sample_rate: Option<f64>,
    pub rx_channel: Option<usize>,
    pub tx_channel: Option<usize>,

    pub rx_gain_sweep: Option<RxGainSweepDto>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Deserialize)]
pub struct GainRangeDto {
    pub from: f64,
    pub to: f64,
    pub step: f64,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Deserialize)]
pub struct RxGainSweepDto {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub strategy: CfgGainSweepStrategy,
    pub window_bursts: Option<u32>,
    pub settling_slots: Option<u32>,
    #[serde(default)]
    pub auto_exit: bool,
    #[serde(default)]
    pub restart_process_per_combo: bool,

    pub test_signal_profile: Option<String>,
    #[serde(default)]
    pub required_ul_slots: Vec<u8>,
    pub min_slot_crc_pass_rate: Option<f64>,
    pub test_device_type: Option<String>,
    pub test_tx_power_dbm: Option<f64>,
    pub test_level_dbm: Option<f64>,

    #[serde(default)]
    pub gains: HashMap<String, GainRangeDto>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
