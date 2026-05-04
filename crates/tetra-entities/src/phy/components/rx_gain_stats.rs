use serde::Serialize;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct RxGainWindowExport {
    pub timestamp: String,
    pub gain_combo: String,
    pub test_level_dbm: f64,
    pub test_tx_power_dbm: f64,
    pub test_device_type: String,
    pub test_signal_mode: String,
    pub expected_slots: u32,
    pub detected: u32,
    pub decode_attempted: u64,
    pub decode_success: u64,
    pub crc_ok: u64,
    pub false_positive: u64,
    pub crc_pass_rate: f64,
    pub false_positive_rate: f64,
    pub slot0_detected: u32,
    pub slot1_detected: u32,
    pub slot2_detected: u32,
    pub slot3_detected: u32,
    pub slot0_crc_pass_rate: f64,
    pub slot1_crc_pass_rate: f64,
    pub slot2_crc_pass_rate: f64,
    pub slot3_crc_pass_rate: f64,
    pub gaps_detected: u32,
    pub gap_rate: f64,
    pub stability_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RxGainSummaryExport {
    pub timestamp: String,
    pub rank: u32,
    pub gain_combo: String,
    pub detected: u32,
    pub expected_slots: u32,
    pub detect_rate: f64,
    pub decode_attempted: u64,
    pub decode_success: u64,
    pub crc_ok: u64,
    pub false_positive: u64,
    pub crc_pass_rate: f64,
    pub false_positive_rate: f64,
    pub passes_required_slots: bool,
    pub passes_slot_crc_threshold: bool,
    pub slot0_crc_pass_rate: f64,
    pub slot1_crc_pass_rate: f64,
    pub slot2_crc_pass_rate: f64,
    pub slot3_crc_pass_rate: f64,
    pub gaps_detected: u32,
    pub gap_rate: f64,
    pub stability_status: String,
    pub test_level_dbm: f64,
    pub test_tx_power_dbm: f64,
    pub test_device_type: String,
    pub test_signal_mode: String,
}

#[derive(Debug, Clone)]
pub struct RxGainExportWriter {
    base_dir: PathBuf,
    run_id: String,
}

impl RxGainExportWriter {
    pub fn new(base_dir: impl AsRef<Path>, run_id: String) -> Result<Self, String> {
        let base_dir = base_dir.as_ref().to_path_buf();
        create_dir_all(&base_dir).map_err(|e| format!("Failed to create export directory '{}': {}", base_dir.display(), e))?;
        Ok(Self { base_dir, run_id })
    }

    fn window_json_path(&self) -> PathBuf {
        self.base_dir.join(format!("rx_gain_window_{}.jsonl", self.run_id))
    }

    fn window_csv_path(&self) -> PathBuf {
        self.base_dir.join(format!("rx_gain_window_{}.csv", self.run_id))
    }

    fn summary_json_path(&self) -> PathBuf {
        self.base_dir.join(format!("rx_gain_summary_{}.json", self.run_id))
    }

    fn summary_csv_path(&self) -> PathBuf {
        self.base_dir.join(format!("rx_gain_summary_{}.csv", self.run_id))
    }

    fn append_line(path: &Path, line: &str) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("Failed to open '{}': {}", path.display(), e))?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|e| format!("Failed to write '{}': {}", path.display(), e))
    }

    pub fn append_window(&self, row: &RxGainWindowExport) -> Result<(), String> {
        let json_line = serde_json::to_string(row).map_err(|e| format!("Failed to serialize window JSON: {}", e))?;
        Self::append_line(&self.window_json_path(), &json_line)?;

        let csv_path = self.window_csv_path();
        if !csv_path.exists() {
            Self::append_line(
                &csv_path,
                "timestamp,gain_combo,test_level_dbm,test_tx_power_dbm,test_device_type,test_signal_mode,expected_slots,detected,decode_attempted,decode_success,crc_ok,false_positive,crc_pass_rate,false_positive_rate,slot0_detected,slot1_detected,slot2_detected,slot3_detected,slot0_crc_pass_rate,slot1_crc_pass_rate,slot2_crc_pass_rate,slot3_crc_pass_rate,gaps_detected,gap_rate,stability_status",
            )?;
        }

        let line = format!(
            "{},\"{}\",{:.3},{:.3},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{},{:.4},\"{}\"",
            row.timestamp,
            row.gain_combo,
            row.test_level_dbm,
            row.test_tx_power_dbm,
            row.test_device_type,
            row.test_signal_mode,
            row.expected_slots,
            row.detected,
            row.decode_attempted,
            row.decode_success,
            row.crc_ok,
            row.false_positive,
            row.crc_pass_rate,
            row.false_positive_rate,
            row.slot0_detected,
            row.slot1_detected,
            row.slot2_detected,
            row.slot3_detected,
            row.slot0_crc_pass_rate,
            row.slot1_crc_pass_rate,
            row.slot2_crc_pass_rate,
            row.slot3_crc_pass_rate,
            row.gaps_detected,
            row.gap_rate,
            row.stability_status
        );
        Self::append_line(&csv_path, &line)
    }

    pub fn write_summary(&self, rows: &[RxGainSummaryExport]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(rows).map_err(|e| format!("Failed to serialize summary JSON: {}", e))?;
        let mut jf = File::create(self.summary_json_path()).map_err(|e| format!("Failed to create summary JSON file: {}", e))?;
        jf.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write summary JSON: {}", e))?;

        let mut cf = File::create(self.summary_csv_path()).map_err(|e| format!("Failed to create summary CSV file: {}", e))?;
        cf.write_all(
            b"timestamp,rank,gain_combo,detected,expected_slots,detect_rate,decode_attempted,decode_success,crc_ok,false_positive,crc_pass_rate,false_positive_rate,passes_required_slots,passes_slot_crc_threshold,slot0_crc_pass_rate,slot1_crc_pass_rate,slot2_crc_pass_rate,slot3_crc_pass_rate,gaps_detected,gap_rate,stability_status,test_level_dbm,test_tx_power_dbm,test_device_type,test_signal_mode\n",
        )
        .map_err(|e| format!("Failed to write summary CSV header: {}", e))?;

        for row in rows {
            let line = format!(
                "{},\"{}\",{},{},{},{:.6},{},{},{},{},{:.6},{:.6},{},{},{:.6},{:.6},{:.6},{:.6},{},{:.4},\"{}\",{:.3},{:.3},{},{}\n",
                row.timestamp,
                row.rank,
                row.gain_combo,
                row.detected,
                row.expected_slots,
                row.detect_rate,
                row.decode_attempted,
                row.decode_success,
                row.crc_ok,
                row.false_positive,
                row.crc_pass_rate,
                row.false_positive_rate,
                row.passes_required_slots,
                row.passes_slot_crc_threshold,
                row.slot0_crc_pass_rate,
                row.slot1_crc_pass_rate,
                row.slot2_crc_pass_rate,
                row.slot3_crc_pass_rate,
                row.gaps_detected,
                row.gap_rate,
                row.stability_status,
                row.test_level_dbm,
                row.test_tx_power_dbm,
                row.test_device_type,
                row.test_signal_mode
            );
            cf.write_all(line.as_bytes())
                .map_err(|e| format!("Failed to write summary CSV row: {}", e))?;
        }

        Ok(())
    }
}
