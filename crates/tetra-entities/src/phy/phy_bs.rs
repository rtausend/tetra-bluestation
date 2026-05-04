use crossbeam_channel::Sender;
use std::cmp::Ordering;
use std::panic;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use tetra_config::bluestation::{RxGainDecodeCounters, SharedConfig};
use tetra_core::tetra_entities::TetraEntity;
use tetra_core::{BitBuffer, BurstType, PhyBlockNum, PhyBlockType, Sap, TdmaTime, TrainingSequence};
use tetra_pdus::phy::traits::rxtx_dev::RxBurstBits;
use tetra_pdus::phy::traits::rxtx_dev::{RxTxDev, TxSlotBits};
use tetra_saps::tp::TpUnitdataInd;
use tetra_saps::{SapMsg, SapMsgInner};

use crate::phy::components::phy_io_file::{FileWriteMsg, PhyIoFileMode};
use crate::phy::components::{burst_consts::*, slotter, train_consts::*};
use crate::umac::subcomp::bs_sched::MACSCHED_TX_AHEAD;
use crate::{MessageQueue, TetraEntityTrait};

use super::components::phy_io_file::PhyIoFile;
use super::components::rx_gain_stats::{RxGainExportWriter, RxGainSummaryExport, RxGainWindowExport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RxGainSweepPhase {
    ApplyGains,
    Settling,
    MeasureWindow,
    FinalizeAndNext,
    Completed,
}

#[derive(Debug, Clone)]
struct RxGainSweepResult {
    gain_combo: HashMap<String, f64>,
    detected_bursts: u32,
    /// Note: Named 'expected_slots' for legacy compatibility, but actually counts rxtx_timeslot ticks
    /// (i.e., physical RX/TX operations), not TDMA slots. Detection rate = detected_bursts / expected_slots.
    expected_slots: u32,
    slot_detected: [u32; 4],
    measured_slots: u32,
    decode_attempted: u64,
    decode_success: u64,
    crc_ok: u64,
    false_positive: u64,
    slot_attempted: [u64; 4],
    slot_crc_ok: [u64; 4],
    /// Number of gaps (missing slots) detected during measurement window
    gaps_detected: u32,
    /// Gap rate as percentage (gaps / expected_slots) * 100
    gap_rate: f64,
    /// Status: STABLE (<3% gaps), UNSTABLE (>=3% gaps)
    stability_status: String,
    /// Duration of measurement window in milliseconds
    duration_ms: u64,
}

#[derive(Debug, Clone)]
struct RxGainSweepRuntime {
    phase: RxGainSweepPhase,
    combos: Vec<HashMap<String, f64>>,
    current_idx: usize,
    settling_slots: u32,
    settling_remaining: u32,
    window_bursts: u32,
    progress_log_step_bursts: u32,
    next_progress_log_bursts: u32,
    measured_bursts: u32,
    /// Note: counts rxtx_timeslot ticks, not TDMA slots (see RxGainSweepResult.expected_slots)
    expected_slots: u32,
    slot_detected: [u32; 4],
    measured_slots: u32,
    window_decode_baseline: RxGainDecodeCounters,
    required_ul_slots: Vec<u8>,
    min_slot_crc_pass_rate: Option<f64>,
    auto_exit: bool,
    test_device_type: String,
    test_signal_mode: String,
    test_level_dbm: f64,
    test_tx_power_dbm: f64,
    export_writer: Option<RxGainExportWriter>,
    run_started_unix: u64,
    results: Vec<RxGainSweepResult>,
    /// Track detected slot times for gap analysis
    detected_slot_times: Vec<TdmaTime>,
    /// Timestamp when measurement window started (Unix seconds)
    window_start_unix: u64,
}

pub struct PhyBs<D: RxTxDev> {
    config: SharedConfig,
    dltime: TdmaTime,

    /// Channel for asynchronous downlink TX data logging
    dl_tx_sender: Option<Sender<FileWriteMsg>>,
    /// Channel for asynchronous uplink RX data logging
    ul_rx_sender: Option<Sender<FileWriteMsg>>,

    /// Testing mode: Transmit input data from file instead of from stack
    dl_input_file: Option<PhyIoFile>,
    /// Testing mode: Parse input data from file instead of from SDR
    ul_input_file: Option<PhyIoFile>,

    /// RX/TX device
    rxtxdev: D,

    rx_gain_sweep: Option<RxGainSweepRuntime>,

    tick: u64,
}

impl<D: RxTxDev> PhyBs<D> {
    fn expand_gain_values(from: f64, to: f64, step: f64) -> Vec<f64> {
        if step <= 0.0 || from > to {
            return Vec::new();
        }

        let mut values = Vec::new();
        let mut i: usize = 0;
        let eps = step.abs() * 1e-6 + 1e-9;

        loop {
            let value = from + step * i as f64;
            if value > to + eps {
                break;
            }
            values.push(value.min(to));
            i += 1;
        }

        if values.is_empty() {
            values.push(from);
        }

        let last = values[values.len() - 1];
        if last < to - eps {
            values.push(to);
        }

        values
    }

    fn generate_gain_combos(gains: &HashMap<String, tetra_config::bluestation::CfgGainRange>) -> Vec<HashMap<String, f64>> {
        let mut keys: Vec<String> = gains.keys().cloned().collect();
        keys.sort();

        let mut combos: Vec<HashMap<String, f64>> = vec![HashMap::new()];

        for key in keys {
            let range = &gains[&key];
            let values = Self::expand_gain_values(range.from, range.to, range.step);

            let mut next = Vec::with_capacity(combos.len() * values.len().max(1));
            for combo in &combos {
                for value in &values {
                    let mut new_combo = combo.clone();
                    new_combo.insert(key.clone(), *value);
                    next.push(new_combo);
                }
            }
            combos = next;
        }

        combos
    }

    fn format_gain_combo(combo: &HashMap<String, f64>) -> String {
        let mut entries: Vec<(&String, &f64)> = combo.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        entries
            .into_iter()
            .map(|(k, v)| format!("{}={:.2}", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn safe_ratio_u64(num: u64, den: u64) -> f64 {
        if den == 0 { 0.0 } else { num as f64 / den as f64 }
    }

    fn safe_ratio_u32(num: u32, den: u32) -> f64 {
        if den == 0 { 0.0 } else { num as f64 / den as f64 }
    }

    fn slot_crc_rates(result: &RxGainSweepResult) -> [f64; 4] {
        [
            Self::safe_ratio_u64(result.slot_crc_ok[0], result.slot_attempted[0]),
            Self::safe_ratio_u64(result.slot_crc_ok[1], result.slot_attempted[1]),
            Self::safe_ratio_u64(result.slot_crc_ok[2], result.slot_attempted[2]),
            Self::safe_ratio_u64(result.slot_crc_ok[3], result.slot_attempted[3]),
        ]
    }

    fn passes_required_slots(runtime: &RxGainSweepRuntime, result: &RxGainSweepResult) -> bool {
        runtime.required_ul_slots.iter().all(|slot| {
            let idx = slot.saturating_sub(1).min(3) as usize;
            result.slot_attempted[idx] > 0
        })
    }

    fn passes_slot_crc_threshold(runtime: &RxGainSweepRuntime, result: &RxGainSweepResult) -> bool {
        let Some(min_rate) = runtime.min_slot_crc_pass_rate else {
            return true;
        };

        runtime.required_ul_slots.iter().all(|slot| {
            let idx = slot.saturating_sub(1).min(3) as usize;
            let attempts = result.slot_attempted[idx];
            if attempts == 0 {
                return false;
            }
            let rate = Self::safe_ratio_u64(result.slot_crc_ok[idx], attempts);
            rate >= min_rate
        })
    }

    fn crc_pass_rate(result: &RxGainSweepResult) -> f64 {
        Self::safe_ratio_u64(result.crc_ok, result.decode_attempted)
    }

    fn false_positive_rate(result: &RxGainSweepResult) -> f64 {
        Self::safe_ratio_u64(result.false_positive, result.decode_attempted)
    }

    fn rank_rx_gain_result(runtime: &RxGainSweepRuntime, result: &RxGainSweepResult) -> (u8, f64, f64, u32) {
        let passes_required = Self::passes_required_slots(runtime, result);
        let passes_threshold = Self::passes_slot_crc_threshold(runtime, result);
        let eligible = if passes_required && passes_threshold { 1 } else { 0 };

        (
            eligible,
            Self::crc_pass_rate(result),
            -Self::false_positive_rate(result),
            result.detected_bursts,
        )
    }

    fn compare_ranked_results(runtime: &RxGainSweepRuntime, a: &RxGainSweepResult, b: &RxGainSweepResult) -> Ordering {
        let (a_eligible, a_crc, a_fp_neg, a_detected) = Self::rank_rx_gain_result(runtime, a);
        let (b_eligible, b_crc, b_fp_neg, b_detected) = Self::rank_rx_gain_result(runtime, b);

        b_eligible
            .cmp(&a_eligible)
            .then_with(|| b_crc.total_cmp(&a_crc))
            .then_with(|| b_fp_neg.total_cmp(&a_fp_neg))
            .then_with(|| b_detected.cmp(&a_detected))
    }

    /// Calculate gap-detection metrics from detected timeslots
    /// Returns: (gaps_detected, gap_rate, stability_status)
    /// gaps_detected = expected_slots - detected_bursts
    fn calculate_gap_metrics(detected_slot_times: &[TdmaTime], expected_slots: u32) -> (u32, f64, String) {
        if expected_slots == 0 {
            return (0, 0.0, "STABLE".to_string());
        }

        // Count unique detected bursts (each entry is one detected burst)
        let detected_bursts = detected_slot_times.len() as u32;
        
        // Calculate gaps: missing slots = expected - detected
        let gaps_detected = expected_slots.saturating_sub(detected_bursts);
        
        // Gap rate: fraction of missing slots relative to expected
        let gap_rate = if expected_slots > 0 {
            gaps_detected as f64 / expected_slots as f64
        } else {
            0.0
        };

        // Classify as STABLE (<3% gaps) or UNSTABLE (>=3% gaps)
        let stability_status = if gap_rate >= 0.03 { "UNSTABLE" } else { "STABLE" }.to_string();

        (gaps_detected, gap_rate, stability_status)
    }

    fn finalize_current_combo(runtime: &mut RxGainSweepRuntime, latest_decode_counters: &RxGainDecodeCounters) {
        let gain_combo = runtime.combos[runtime.current_idx].clone();
        let decode_delta = latest_decode_counters.diff_from(&runtime.window_decode_baseline);
        runtime.window_decode_baseline = latest_decode_counters.clone();

        // Calculate gap-detection metrics
        let (gaps_detected, gap_rate, stability_status) = Self::calculate_gap_metrics(&runtime.detected_slot_times, runtime.expected_slots);

        // Calculate measurement window duration in milliseconds
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let duration_ms = ((now_unix.saturating_sub(runtime.window_start_unix)) * 1000) as u64;

        let result = RxGainSweepResult {
            gain_combo: gain_combo.clone(),
            detected_bursts: runtime.measured_bursts,
            expected_slots: runtime.expected_slots,
            slot_detected: runtime.slot_detected,
            measured_slots: runtime.measured_slots,
            decode_attempted: decode_delta.decode_attempted,
            decode_success: decode_delta.decode_success,
            crc_ok: decode_delta.crc_ok,
            false_positive: decode_delta.false_positive,
            slot_attempted: decode_delta.slot_attempted,
            slot_crc_ok: decode_delta.slot_crc_ok,
            gaps_detected,
            gap_rate,
            stability_status: stability_status.clone(),
            duration_ms,
        };

        let slot_rates = Self::slot_crc_rates(&result);
        tracing::info!(
            combo_index = runtime.current_idx,
            detected_bursts = result.detected_bursts,
            expected_slots = result.expected_slots,
            decode_attempted = result.decode_attempted,
            decode_success = result.decode_success,
            crc_ok = result.crc_ok,
            false_positive = result.false_positive,
            crc_pass_rate = Self::crc_pass_rate(&result),
            false_positive_rate = Self::false_positive_rate(&result),
            slot0_crc_pass_rate = slot_rates[0],
            slot1_crc_pass_rate = slot_rates[1],
            slot2_crc_pass_rate = slot_rates[2],
            slot3_crc_pass_rate = slot_rates[3],
            gain_combo = %Self::format_gain_combo(&gain_combo),
            gaps_detected = gaps_detected,
            gap_rate = format!("{:.2}%", gap_rate * 100.0),
            stability = %stability_status,
            "rx_gain_sweep_measure_done"
        );

        if let Some(writer) = &runtime.export_writer {
            let row = RxGainWindowExport {
                timestamp: format!("{}", runtime.run_started_unix),
                gain_combo: Self::format_gain_combo(&gain_combo),
                test_level_dbm: runtime.test_level_dbm,
                test_tx_power_dbm: runtime.test_tx_power_dbm,
                test_device_type: runtime.test_device_type.clone(),
                test_signal_mode: runtime.test_signal_mode.clone(),
                expected_slots: result.expected_slots,
                detected: result.detected_bursts,
                decode_attempted: result.decode_attempted,
                decode_success: result.decode_success,
                crc_ok: result.crc_ok,
                false_positive: result.false_positive,
                crc_pass_rate: Self::crc_pass_rate(&result),
                false_positive_rate: Self::false_positive_rate(&result),
                slot0_detected: result.slot_detected[0],
                slot1_detected: result.slot_detected[1],
                slot2_detected: result.slot_detected[2],
                slot3_detected: result.slot_detected[3],
                slot0_crc_pass_rate: slot_rates[0],
                slot1_crc_pass_rate: slot_rates[1],
                slot2_crc_pass_rate: slot_rates[2],
                slot3_crc_pass_rate: slot_rates[3],
                gaps_detected,
                gap_rate,
                stability_status: stability_status.clone(),
                duration_ms: result.duration_ms,
            };

            if let Err(err) = writer.append_window(&row) {
                tracing::error!("Failed to append RX gain window export: {}", err);
            }
        }

        runtime.results.push(result);
    }

    fn report_rx_gain_sweep_results(runtime: &RxGainSweepRuntime) {
        let mut ranked = runtime.results.clone();
        ranked.sort_by(|a, b| Self::compare_ranked_results(runtime, a, b));

        let is_valid = |row: &RxGainSweepResult| {
            Self::passes_required_slots(runtime, row) && Self::passes_slot_crc_threshold(runtime, row)
        };
        let valid_count = ranked.iter().filter(|row| is_valid(row)).count();

        tracing::info!(
            total_combos = runtime.combos.len(),
            measured_combos = runtime.results.len(),
            valid_combos = valid_count,
            test_device_type = %runtime.test_device_type,
            test_signal_mode = %runtime.test_signal_mode,
            "rx_gain_sweep completed"
        );

        if valid_count == 0 {
            tracing::warn!(
                required_ul_slots = ?runtime.required_ul_slots,
                min_slot_crc_pass_rate = ?runtime.min_slot_crc_pass_rate,
                "rx_gain_sweep_no_valid_winner"
            );
        } else if let Some(best_valid) = ranked.iter().find(|row| is_valid(row)) {
            tracing::info!(
                gain_combo = %Self::format_gain_combo(&best_valid.gain_combo),
                crc_pass_rate = Self::crc_pass_rate(best_valid),
                false_positive_rate = Self::false_positive_rate(best_valid),
                "rx_gain_sweep_best_valid"
            );
        }

        for (i, result) in ranked.iter().take(3).enumerate() {
            let slot_rates = Self::slot_crc_rates(result);
            tracing::info!(
                rank = i + 1,
                detected_bursts = result.detected_bursts,
                expected_slots = result.expected_slots,
                measured_slots = result.measured_slots,
                decode_attempted = result.decode_attempted,
                decode_success = result.decode_success,
                crc_ok = result.crc_ok,
                false_positive = result.false_positive,
                crc_pass_rate = Self::crc_pass_rate(result),
                false_positive_rate = Self::false_positive_rate(result),
                passes_required_slots = Self::passes_required_slots(runtime, result),
                passes_slot_crc_threshold = Self::passes_slot_crc_threshold(runtime, result),
                slot0_crc_pass_rate = slot_rates[0],
                slot1_crc_pass_rate = slot_rates[1],
                slot2_crc_pass_rate = slot_rates[2],
                slot3_crc_pass_rate = slot_rates[3],
                gain_combo = %Self::format_gain_combo(&result.gain_combo),
                "rx_gain_sweep_top"
            );
        }

        if let Some(writer) = &runtime.export_writer {
            let timestamp = format!("{}", runtime.run_started_unix);
            let summary: Vec<RxGainSummaryExport> = ranked
                .iter()
                .enumerate()
                .map(|(idx, row)| {
                    let detect_rate = Self::safe_ratio_u32(row.detected_bursts, row.expected_slots);
                    let slot_rates = Self::slot_crc_rates(row);

                    RxGainSummaryExport {
                        timestamp: timestamp.clone(),
                        rank: (idx + 1) as u32,
                        gain_combo: Self::format_gain_combo(&row.gain_combo),
                        detected: row.detected_bursts,
                        expected_slots: row.expected_slots,
                        detect_rate,
                        decode_attempted: row.decode_attempted,
                        decode_success: row.decode_success,
                        crc_ok: row.crc_ok,
                        false_positive: row.false_positive,
                        crc_pass_rate: Self::crc_pass_rate(row),
                        false_positive_rate: Self::false_positive_rate(row),
                        passes_required_slots: Self::passes_required_slots(runtime, row),
                        passes_slot_crc_threshold: Self::passes_slot_crc_threshold(runtime, row),
                        slot0_crc_pass_rate: slot_rates[0],
                        slot1_crc_pass_rate: slot_rates[1],
                        slot2_crc_pass_rate: slot_rates[2],
                        slot3_crc_pass_rate: slot_rates[3],
                        gaps_detected: row.gaps_detected,
                        gap_rate: row.gap_rate,
                        stability_status: row.stability_status.clone(),
                        duration_ms: row.duration_ms,
                        test_level_dbm: runtime.test_level_dbm,
                        test_tx_power_dbm: runtime.test_tx_power_dbm,
                        test_device_type: runtime.test_device_type.clone(),
                        test_signal_mode: runtime.test_signal_mode.clone(),
                    }
                })
                .collect();

            if let Err(err) = writer.write_summary(&summary) {
                tracing::error!("Failed to write RX gain summary export: {}", err);
            }
        }
    }

    fn rx_gain_sweep_tick_pre(&mut self) {
        let config = self.config.clone();
        let Some(runtime) = self.rx_gain_sweep.as_mut() else {
            return;
        };

        match runtime.phase {
            RxGainSweepPhase::ApplyGains => {
                let combo = &runtime.combos[runtime.current_idx];
                if let Err(err) = self.rxtxdev.apply_rx_gain_combo(combo) {
                    tracing::error!("Failed to apply runtime RX gain combo: {:?}", err);
                    runtime.phase = RxGainSweepPhase::Completed;
                    return;
                }

                tracing::info!(
                    combo_index = runtime.current_idx,
                    gain_combo = %Self::format_gain_combo(combo),
                    "rx_gain_sweep_apply"
                );
                runtime.settling_remaining = runtime.settling_slots;
                runtime.measured_bursts = 0;
                runtime.next_progress_log_bursts = runtime.progress_log_step_bursts;
                runtime.expected_slots = 0;
                runtime.slot_detected = [0; 4];
                runtime.measured_slots = 0;
                runtime.detected_slot_times.clear();
                runtime.window_start_unix = 0;
                runtime.window_decode_baseline = config.state_read().rx_gain_decode_counters.clone();
                runtime.phase = RxGainSweepPhase::Settling;
            }
            RxGainSweepPhase::Settling => {
                if runtime.settling_remaining > 0 {
                    runtime.settling_remaining -= 1;
                } else {
                    tracing::info!(combo_index = runtime.current_idx, "rx_gain_sweep_measure_start");
                    // Record start time for measurement window duration calculation
                    runtime.window_start_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    runtime.phase = RxGainSweepPhase::MeasureWindow;
                }
            }
            RxGainSweepPhase::MeasureWindow => {}
            RxGainSweepPhase::FinalizeAndNext => {
                let latest_decode_counters = config.state_read().rx_gain_decode_counters.clone();
                Self::finalize_current_combo(runtime, &latest_decode_counters);

                runtime.current_idx += 1;
                if runtime.current_idx < runtime.combos.len() {
                    runtime.phase = RxGainSweepPhase::ApplyGains;
                } else {
                    runtime.phase = RxGainSweepPhase::Completed;
                    Self::report_rx_gain_sweep_results(runtime);
                    if runtime.auto_exit {
                        tracing::info!("rx_gain_sweep auto_exit enabled, terminating process");
                        std::process::exit(0);
                    }
                }
            }
            RxGainSweepPhase::Completed => {}
        }
    }

    fn rx_gain_sweep_tick_post(&mut self, detected_bursts: u32, slot_detected: [u32; 4]) {
        let Some(runtime) = self.rx_gain_sweep.as_mut() else {
            return;
        };

        if runtime.phase != RxGainSweepPhase::MeasureWindow {
            return;
        }

        runtime.measured_bursts = runtime.measured_bursts.saturating_add(detected_bursts);
        runtime.expected_slots = runtime.expected_slots.saturating_add(1);
        for (i, val) in slot_detected.iter().enumerate() {
            runtime.slot_detected[i] = runtime.slot_detected[i].saturating_add(*val);
        }
        runtime.measured_slots = runtime.measured_slots.saturating_add(1);

        if detected_bursts > 0 {
            let progress_pct = Self::safe_ratio_u32(runtime.measured_bursts, runtime.window_bursts) * 100.0;
            tracing::info!(
                combo_index = runtime.current_idx,
                bursts_in_tick = detected_bursts,
                counted_bursts = runtime.measured_bursts,
                target_bursts = runtime.window_bursts,
                progress_pct,
                measured_slots = runtime.measured_slots,
                "rx_gain_sweep_burst_count"
            );
        }

        if runtime.measured_bursts < runtime.window_bursts
            && runtime.measured_bursts >= runtime.next_progress_log_bursts
        {
            let progress_pct = Self::safe_ratio_u32(runtime.measured_bursts, runtime.window_bursts) * 100.0;
            tracing::info!(
                combo_index = runtime.current_idx,
                counted_bursts = runtime.measured_bursts,
                target_bursts = runtime.window_bursts,
                progress_pct,
                measured_slots = runtime.measured_slots,
                "rx_gain_sweep_progress"
            );

            while runtime.next_progress_log_bursts <= runtime.measured_bursts {
                runtime.next_progress_log_bursts = runtime
                    .next_progress_log_bursts
                    .saturating_add(runtime.progress_log_step_bursts.max(1));
            }
        }

        if runtime.measured_bursts >= runtime.window_bursts {
            tracing::debug!(combo_index = runtime.current_idx, "rx_gain_sweep_window_ready");
            runtime.phase = RxGainSweepPhase::FinalizeAndNext;
        }
    }

    pub fn new(config: SharedConfig, rxtxdev: D) -> Self {
        let c = &config.config().phy_io;

        // Create async writers for file logging of generated DL and received UL signals
        let dl_tx_logger = c
            .dl_tx_file
            .as_ref()
            .and_then(|f| PhyIoFile::create_async_writer(f, "dl_tx_logger".to_string()).ok());
        let ul_rx_logger = c
            .ul_rx_file
            .as_ref()
            .and_then(|f| PhyIoFile::create_async_writer(f, "ul_rx_logger".to_string()).ok());

        // Open input files overriding either generated DL or received UL data
        let dl_input_file = if let Some(ref f) = c.dl_input_file {
            Some(PhyIoFile::new(f, PhyIoFileMode::ReadRepeat).expect("Failed to open dl_input_file"))
        } else {
            None
        };
        let ul_input_file = if let Some(ref f) = c.ul_input_file {
            Some(PhyIoFile::new(f, PhyIoFileMode::Read).expect("Failed to open ul_input_file"))
        } else {
            None
        };

        let rx_gain_sweep = {
            let rx_gain_test_mode = config.state_read().rx_gain_test_mode;
            if rx_gain_test_mode {
                config
                    .config()
                    .phy_io
                    .soapysdr
                    .as_ref()
                    .and_then(|soapy| soapy.rx_gain_sweep.as_ref())
                    .and_then(|sweep| {
                        if !sweep.enabled {
                            return None;
                        }

                        let combos = Self::generate_gain_combos(&sweep.gains);
                        if combos.is_empty() {
                            return None;
                        }

                        let run_started_unix = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let run_id = format!("{}", run_started_unix);
                        let export_writer = match RxGainExportWriter::new("rx_gain_sweep_results", run_id) {
                            Ok(w) => Some(w),
                            Err(err) => {
                                tracing::warn!("RX gain export disabled: {}", err);
                                None
                            }
                        };

                        Some(RxGainSweepRuntime {
                            phase: RxGainSweepPhase::ApplyGains,
                            combos,
                            current_idx: 0,
                            settling_slots: sweep.settling_slots,
                            settling_remaining: 0,
                            window_bursts: sweep.window_bursts,
                            progress_log_step_bursts: (sweep.window_bursts / 10).max(1),
                            next_progress_log_bursts: (sweep.window_bursts / 10).max(1),
                            measured_bursts: 0,
                            expected_slots: 0,
                            slot_detected: [0; 4],
                            measured_slots: 0,
                            window_decode_baseline: RxGainDecodeCounters::default(),
                            required_ul_slots: if sweep.required_ul_slots.is_empty() {
                                vec![1, 2, 3, 4]
                            } else {
                                sweep.required_ul_slots.clone()
                            },
                            min_slot_crc_pass_rate: sweep.min_slot_crc_pass_rate,
                            auto_exit: sweep.auto_exit,
                            test_device_type: sweep
                                .test_device_type
                                .clone()
                                .unwrap_or_else(|| "stabilock4032".to_string()),
                            test_signal_mode: sweep
                                .test_signal_profile
                                .clone()
                                .unwrap_or_else(|| "t1_all_ul_slots".to_string()),
                            test_level_dbm: sweep.test_level_dbm.unwrap_or(0.0),
                            test_tx_power_dbm: sweep.test_tx_power_dbm.unwrap_or(0.0),
                            export_writer,
                            run_started_unix,
                            results: Vec::new(),
                            detected_slot_times: Vec::new(),
                            window_start_unix: 0,
                        })
                    })
            } else {
                None
            }
        };

        Self {
            config,
            dltime: TdmaTime::default(), // updated in tick_start
            dl_tx_sender: dl_tx_logger,
            ul_rx_sender: ul_rx_logger,
            dl_input_file,
            ul_input_file,
            rxtxdev,
            rx_gain_sweep,
            tick: 0,
        }
    }

    fn send_rxblock_to_lmac(
        queue: &mut MessageQueue,
        train_type: TrainingSequence,
        burst_type: BurstType,
        block_type: PhyBlockType,
        block_num: PhyBlockNum,
        bits: BitBuffer,
    ) {
        // Uplink timeslot is two after downlink. Thus was transmitted at dltime - 2
        let sapmsg = SapMsg {
            sap: Sap::TpSap,
            src: TetraEntity::Phy,
            dest: TetraEntity::Lmac,
            msg: SapMsgInner::TpUnitdataInd(TpUnitdataInd {
                train_type,
                burst_type,
                block_type,
                block_num,
                block: bits,
            }),
        };
        queue.push_back(sapmsg);
    }

    fn split_rxslot_and_send_to_lmac(queue: &mut MessageQueue, burst: &RxBurstBits<'_>) {
        let train_seq = burst.train_type;
        match train_seq {
            TrainingSequence::NormalTrainSeq1 => {
                assert!(burst.bits.len() == NUB_BITS);

                let mut blk = BitBuffer::new(NUB_BLK_BITS * 2);
                blk.copy_bits_from_bitarr(&burst.bits[NUB_BLK1_OFFSET..NUB_BLK1_OFFSET + NUB_BLK_BITS]);
                blk.copy_bits_from_bitarr(&burst.bits[NUB_BLK2_OFFSET..NUB_BLK2_OFFSET + NUB_BLK_BITS]);
                blk.seek(0);

                Self::send_rxblock_to_lmac(queue, train_seq, BurstType::NUB, PhyBlockType::NUB, PhyBlockNum::Both, blk);
            }

            TrainingSequence::NormalTrainSeq2 => {
                assert!(burst.bits.len() == NUB_BITS);

                let blk1 = BitBuffer::from_bitarr(&burst.bits[NUB_BLK1_OFFSET..NUB_BLK1_OFFSET + NUB_BLK_BITS]);
                let blk2 = BitBuffer::from_bitarr(&burst.bits[NUB_BLK2_OFFSET..NUB_BLK2_OFFSET + NUB_BLK_BITS]);

                Self::send_rxblock_to_lmac(queue, train_seq, BurstType::NUB, PhyBlockType::NUB, PhyBlockNum::Block1, blk1);
                Self::send_rxblock_to_lmac(queue, train_seq, BurstType::NUB, PhyBlockType::NUB, PhyBlockNum::Block2, blk2);
            }
            TrainingSequence::ExtendedTrainSeq => {
                assert!(burst.bits.len() == CUB_BITS);

                let mut blk = BitBuffer::new(CUB_BLK_BITS * 2);
                blk.copy_bits_from_bitarr(&burst.bits[CUB_BLK1_OFFSET..CUB_BLK1_OFFSET + CUB_BLK_BITS]);
                blk.copy_bits_from_bitarr(&burst.bits[CUB_BLK2_OFFSET..CUB_BLK2_OFFSET + CUB_BLK_BITS]);
                blk.seek(0);

                Self::send_rxblock_to_lmac(queue, train_seq, BurstType::CUB, PhyBlockType::SSN1, PhyBlockNum::Block1, blk);
            }

            _ => panic!(),
        }
    }

    fn log_rx_burst(ts: TdmaTime, slot_kind: &'static str, burst: &RxBurstBits<'_>) {
        tracing::info!(
            ts = %ts,
            slot_kind,
            train_type = ?burst.train_type,
            train_errs = burst.train_errs,
            burst_pos = burst.burst_pos,
            burst_len = burst.burst_len,
            "phy_rx_burst"
        );
    }

    fn rx_tpsap_prim(&mut self, queue: &mut MessageQueue, message: SapMsg) {
        // Handle TpUnitdataReq with a TX slot
        // Prepare TxSlotBits for transmission
        // TODO FIXME: optimize

        self.tick += 1;
    self.rx_gain_sweep_tick_pre();

        let SapMsgInner::TpUnitdataReq(prim) = message.msg else { panic!() };

        // Generate block (from file or from LMAC data)
        let mut dl_burst = [0u8; TIMESLOT_TYPE4_BITS];
        if let Some(dl_input_file) = &mut self.dl_input_file {
            // Code for testing mode, when replaying from DL input file
            dl_input_file.read_block(&mut dl_burst).expect("Failed to read dl_input_file data");
        } else {
            // We received data from LMAC, convert BBK block to bitarr
            assert!(prim.bbk.is_some());
            let mut bbk = [0u8; 30];
            prim.bbk.unwrap().to_bitarr(&mut bbk);

            // Build NDB or SDB burst
            dl_burst = match prim.burst_type {
                BurstType::SDB => {
                    // SDB burst
                    assert!(prim.train_type == TrainingSequence::SyncTrainSeq);
                    assert!(prim.blk1.is_some() && prim.blk2.is_some());

                    let mut blk1 = [0u8; 120];
                    let mut blk2 = [0u8; 216];
                    prim.blk1.unwrap().to_bitarr(&mut blk1); // Guaranteed for SDB
                    prim.blk2.unwrap().to_bitarr(&mut blk2); // Guaranteed for SDB

                    slotter::build_sdb(&blk1, &bbk, &blk2)
                }
                BurstType::NDB => {
                    let mut blk1 = [0u8; 216];
                    let mut blk2 = [0u8; 216];

                    match prim.train_type {
                        TrainingSequence::NormalTrainSeq1 => {
                            // Single large block
                            assert!(prim.blk1.is_some() && prim.blk2.is_none());
                            let mut blk1_src = prim.blk1.unwrap(); // Guaranteed for NDB
                            blk1_src.to_bitarr(&mut blk1);
                            blk1_src.to_bitarr(&mut blk2);
                        }
                        TrainingSequence::NormalTrainSeq2 => {
                            // Two half slots
                            assert!(prim.blk1.is_some() && prim.blk2.is_some());
                            prim.blk1.unwrap().to_bitarr(&mut blk1); // Guaranteed for NDB
                            prim.blk2.unwrap().to_bitarr(&mut blk2); // Guaranteed for NDB trainseq 2
                        }
                        _ => panic!("Unsupported training sequence for NDB burst"),
                    }

                    slotter::build_ndb(prim.train_type, &blk1, &bbk, &blk2)
                }
                _ => panic!(),
            };
        }

        // Prepare the TX slot for the tx device
        let tx_slot: [TxSlotBits; 1] = [TxSlotBits {
            time: self.dltime.add_timeslots(MACSCHED_TX_AHEAD as i32),
            slot: Some(&dl_burst),
            ..Default::default()
        }];

        // Code for testing mode, when capturing all DL output to file
        if let Some(dl_tx_sender) = &self.dl_tx_sender {
            let _ = dl_tx_sender.try_send(FileWriteMsg::WriteBlock(dl_burst.to_vec()));
        }

        // Transmit slot and receive rx data (if any trainseq was found)
        // This function is blocking and the source of timing sync in the whole stack
        // let tick_done = std::time::Instant::now();
        let rx = self.rxtxdev.rxtx_timeslot(&tx_slot).expect("Got error from rxtx_timeslot");
        // let new_tick_start = std::time::Instant::now();
        // let elapsed = new_tick_start.duration_since(tick_done);
        // tracing::debug!("rxtx_timeslot: tick_done {:?}, new_tick_start {:?}, elapsed {:?}", tick_done, new_tick_start, elapsed);

        // Process received slot (either full, subslot1 or subslot2)
        // In exceptional cases, we might receive multiple slots (multiple possible detected bursts in one timeslot)
        // This may be due to two subslots, or due to false psoitives in training seq detection
        // The Lmac error correction will eliminate the false positives
        let mut detected_bursts: u32 = 0;
        let mut detected_per_slot: [u32; 4] = [0; 4];
        for rx_slot in rx {
            if let Some(rx_slot) = rx_slot {
                let ts_idx = (rx_slot.time.t as usize).saturating_sub(1).min(3);
                let mut slot_sent = false;
                if rx_slot.slot.train_type != TrainingSequence::NotFound {
                    detected_bursts = detected_bursts.saturating_add(1);
                    detected_per_slot[ts_idx] = detected_per_slot[ts_idx].saturating_add(1);
                    // Track detected timeslot for gap analysis
                    if let Some(runtime) = &mut self.rx_gain_sweep {
                        runtime.detected_slot_times.push(rx_slot.time);
                    }
                    Self::log_rx_burst(self.dltime, "fullslot", &rx_slot.slot);

                    if let Some(ul_rx_sender) = &self.ul_rx_sender {
                        // Log received data to file (non-blocking)
                        let _ = ul_rx_sender.try_send(FileWriteMsg::WriteHeaderAndBlock(3, self.tick, rx_slot.slot.bits.to_vec()));
                    }

                    Self::split_rxslot_and_send_to_lmac(queue, &rx_slot.slot);
                    slot_sent = true;
                }
                if rx_slot.subslot1.train_type != TrainingSequence::NotFound {
                    detected_bursts = detected_bursts.saturating_add(1);
                    detected_per_slot[ts_idx] = detected_per_slot[ts_idx].saturating_add(1);
                    // Track detected timeslot for gap analysis
                    if let Some(runtime) = &mut self.rx_gain_sweep {
                        runtime.detected_slot_times.push(rx_slot.time);
                    }
                    Self::log_rx_burst(self.dltime, "subslot1", &rx_slot.subslot1);
                    if slot_sent {
                        tracing::warn!("Sending same burst twice to LMAC");
                    }
                    if let Some(ul_rx_sender) = &self.ul_rx_sender {
                        // Log received data to file (non-blocking)
                        let _ = ul_rx_sender.try_send(FileWriteMsg::WriteHeaderAndBlock(1, self.tick, rx_slot.subslot1.bits.to_vec()));
                    }

                    Self::split_rxslot_and_send_to_lmac(queue, &rx_slot.subslot1);
                    slot_sent = true;
                }
                if rx_slot.subslot2.train_type != TrainingSequence::NotFound {
                    detected_bursts = detected_bursts.saturating_add(1);
                    detected_per_slot[ts_idx] = detected_per_slot[ts_idx].saturating_add(1);
                    // Track detected timeslot for gap analysis
                    if let Some(runtime) = &mut self.rx_gain_sweep {
                        runtime.detected_slot_times.push(rx_slot.time);
                    }
                    Self::log_rx_burst(self.dltime, "subslot2", &rx_slot.subslot2);
                    if slot_sent {
                        tracing::warn!("Sending same burst twice to LMAC");
                    }
                    if let Some(ul_rx_sender) = &self.ul_rx_sender {
                        // Log received data to file (non-blocking)
                        let _ = ul_rx_sender.try_send(FileWriteMsg::WriteHeaderAndBlock(2, self.tick, rx_slot.subslot2.bits.to_vec()));
                    }

                    Self::split_rxslot_and_send_to_lmac(queue, &rx_slot.subslot2);
                }
            }
        }

        self.rx_gain_sweep_tick_post(detected_bursts, detected_per_slot);
    }

    fn rx_tpc_prim(&mut self, _queue: &mut MessageQueue, _message: SapMsg) {
        unimplemented!();
    }
}

impl<D: RxTxDev + Send + 'static> TetraEntityTrait for PhyBs<D> {
    fn entity(&self) -> TetraEntity {
        TetraEntity::Phy
    }

    fn rx_prim(&mut self, queue: &mut MessageQueue, message: SapMsg) {
        tracing::debug!("rx_prim: {:?}", message);
        // tracing::debug!(ts=%message.dltime, "rx_prim: {:?}", message);

        match message.sap {
            Sap::TpSap => {
                self.rx_tpsap_prim(queue, message);
            }
            Sap::TpcSap => {
                self.rx_tpc_prim(queue, message);
            }
            _ => {
                panic!();
            }
        }
    }

    fn tick_start(&mut self, _queue: &mut MessageQueue, ts: TdmaTime) {
        self.dltime = ts;
    }
}
