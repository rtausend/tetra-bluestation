# Implementation Plan: RX Gain Tuning with Stabilock Test Bursts for tetra-bluestation

## 1. Goal

The goal is a reproducible measurement and evaluation path that can determine, for defined Stabilock test transmissions:

- which RX gain settings (for example LNA/TIA/PGA on Lime) provide the best decoding quality;
- up to which injected transmit power level (or received level range) bursts remain meaningfully processable;
- in which range overdrive, false positives, or CRC errors become dominant.

The plan focuses on the existing BS uplink path and builds on the current logging/decoding behavior.

## 2. Standards Reference (work-relevant)

For the measurement logic, the following TETRA air interface fundamentals are used (EN 300 392-2, already indirectly referenced in the code base):

- TDMA with 4 slots per frame, 18 frames per multiframe, 255 symbols per slot.
- pi/4-DQPSK, 18 ksps symbol rate.
- Training sequences as the primary burst detection feature.
- Uplink burst forms:
  - Normal Uplink Burst (NUB, full slot, NormalTrainSeq1/2),
  - Control Uplink Burst (CUB, subslot, ExtendedTrainSeq).
- Quality decisions should not stop at L1 detection; they must continue to L2/LMAC checks (CRC/FEC).

Important for the measurement setup: finding a training sequence in PHY alone is not enough to count as good. For gain tuning, the end-to-end rate up to CRC is decisive.

## 2.1 Stabilock-4032 Relevant Findings (from main manual + TETRA BS test document)

From your provided file `M_4032_GER_0306_622_A.pdf` (main operating manual), the following points could be clearly confirmed:

- The device explicitly lists TETRA software options:
  - `Tetra MS Test`
  - `Tetra BS Test`
- The operating concept for receiver tests is based on:
  - `RX-Maske` (base mask for receiver measurements),
  - `RX-SPECIALS` (automated typical receiver measurements),
  - `GEN_A` switching for RX/TX signal path,
  - parameter/configuration masks via `DEF.PAR`.
- The main manual therefore describes the operational framework, but not the full TETRA burst details of the BS test option package.

For TETRA BS specific burst modes, the TETRA BS test supplement was additionally used:

From the available TETRA BS test supplement for 4031/4032 (option 897 942, "TETRA BS Test"), the following are especially relevant for our RX test mode:

- RX tests on BS are explicitly supported ("feed a test signal to the BS, allowing to test the receiver").
- The generated RX test signal type is selectable:
  - TCH/7.2 (bursted),
  - SCH/F (bursted),
  - Bit Pattern (continuous: 0000, 1111, 0101, 1010, 1100, PN9, unmod).
- The datasheet states for TETRA:
  - symbol rate 18 ksym/s,
  - pi/4 DQPSK,
  - burst patterns T1:TCH/7.2 and T1:SCH/F,
  - continuous patterns including PN9.

Consequence for BlueStation:

- The sweep workflow should explicitly distinguish bursted and continuous test modes.
- For actual sensitivity/gain optimization, bursted modes (TCH/7.2 or SCH/F) are primary, because they are closer to real uplink behavior.
- For T1 test mode with load on all 4 UL slots, evaluation must be per-slot, not only aggregated.

Mapping for evaluation (important for consistency):

- Stabilock `TCH/7.2` (bursted) -> in BlueStation primarily NUB/full-slot path (traffic).
- Stabilock `SCH/F` (bursted) -> in BlueStation primarily NUB/full-slot path (control).
- Stabilock `Bit Pattern` (continuous, for example PN9) -> only limited suitability for end-to-end CRC; primarily for RF/modulation observation.

Practical implementation rule from source comparison:

- Main manual (`M_4032_GER_0306_622_A.pdf`) for operation and workflow concept (RX mask, specials, signal path).
- TETRA BS supplement for signal/burst specific parameters (TCH/7.2, SCH/F, PN9, autosync behavior).

## 2.2 External Technical Note from `sxxcvr` Example Script

From the linked script `plot_rxtx_response.py` (tejeez/sxxcvr), two technical patterns are interesting for our implementation:

- Explicit wait phase until PLL lock after frequency changes (time-based via hardware time).
- Fast RF amplitude estimation via correlation against a known tone (windowed matched correlation).

Classification for BlueStation:

- This is useful as an optional RF pre-check/health-check per gain combination.
- For actual TETRA RX evaluation, end-to-end (detect -> decode -> CRC) remains the primary criterion.
- In BlueStation, the correlation value may only be a secondary metric, not a replacement for 4/4 or CRC criteria.

## 3. Current State in the Project (Code Mapping)

### 3.1 PHY / Burst Detection

- File: `crates/tetra-entities/src/phy/components/demodulator.rs`
  - `SlotBurstFinder` detects training sequences via Hamming distance.
  - Hard thresholds are currently very strict (`SEQ_*_MAX_ERRS = 1`).
  - Internally available: `train_errs`, `burst_pos`, `train_type`.

- File: `crates/tetra-entities/src/phy/phy_bs.rs`
  - `rx_tpsap_prim()` currently logs only events such as:
    - `got NormalTrainSeq1 in fullslot`
  - Bursts are forwarded to LMAC (`split_rxslot_and_send_to_lmac`).

### 3.2 SDR/Gain Configuration

- File: `crates/tetra-config/src/bluestation/sec_phy.rs`
  - Reads `rx_gain_*` and `tx_gain_*` from `config.toml`.

- File: `crates/tetra-entities/src/phy/components/soapy_settings.rs`
  - Device-specific gain elements and defaults (for example Lime: LNA/TIA/PGA).

- File: `crates/tetra-entities/src/phy/components/soapyio.rs`
  - Sets gains via `set_gain_element(...)` at startup.
  - Runtime changes of RX gains are currently not implemented.

### 3.3 LMAC Decoding as Quality Criterion

- File: `crates/tetra-entities/src/lmac/lmac_bs.rs`
  - `rx_blk_control()` uses `decode_cp(...)` and checks `crc_pass`.
  - `rx_blk_traffic()` uses `decode_tp(...)` and evaluates `crc_ok`.

- File: `crates/tetra-entities/src/lmac/components/errorcontrol.rs`
  - `decode_cp(...)` and `decode_tp(...)` return CRC success.

## 4. Core Evaluation Idea

We evaluate each gain/level combination through a metric cascade:

1. **L1 Detect Rate**
  - Fraction of detected bursts relative to `expected_slots` in the measurement window.
2. **L1 Train Quality**
   - Distribution of `train_errs` and position offset `burst_pos`.
3. **L2 Decode Rate (Control/Traffic)**
   - Fraction of bursts that reach `decode_cp/decode_tp`.
4. **CRC Pass Rate**
   - Fraction with `crc_pass == true` or `crc_ok == true`.
5. **False Positive Rate**
   - Bursts with training detection but without valid downstream processing.

Note on metric definition:

- In the test run, `expected_slots` is maintained as the number of observed slot opportunities in the measurement window (not as externally guaranteed burst rate from the test device). This keeps the metric consistent even with variable burst patterns.

T1-specific extension (4/4 criterion):

- For T1 tests, `slot_crc_pass_rate[0..3]` is additionally evaluated per timeslot.
- A level is considered "4/4 stable" only if all 4 UL slots reach the configured minimum value (for example `slot_crc_pass_rate >= 95%`).
- The "lower sensitivity limit" for a gain set is the lowest level where "4/4 stable" still holds.

For operation, the best gain is in the range where CRC pass is maximal, while false positive rate remains low and there are no clipping indicators.

## 5. Implementation Plan

## Phase A - Extend Telemetry (without behavior change)

### A1. Make PHY Debug Data Visible per Burst

Files:

- `crates/tetra-entities/src/phy/components/demodulator.rs`
- `crates/tetra-entities/src/phy/phy_bs.rs`

Steps:

- Extend `SlotBurstFinder` with readable burst metadata (at least: `train_errs`, `burst_pos`, `burst_len`).
- Propagate this metadata in `RxBurstBits`/new metadata struct up to `phy_bs.rs`.
- Expand logging in `rx_tpsap_prim()` from only "train_type found" to structured fields:
  - timestamp/tdma time,
  - fullslot/subslot1/subslot2,
  - training sequence,
  - train_errs,
  - burst_pos.

Result:

- It becomes visible whether gain degrades correlation margin long before CRC starts to fail.

### A2. Log LMAC Success Metrics Consistently

Files:

- `crates/tetra-entities/src/lmac/lmac_bs.rs`
- optionally `crates/tetra-entities/src/lmac/components/errorcontrol.rs`

Steps:

- In `rx_blk_control()` and `rx_blk_traffic()`, emit standardized measurement logs:
  - logical channel,
  - block_num,
  - decode attempted (yes/no),
  - CRC result.
- Ensure clear correlation to the PHY event (for example via tdma time + slot type).

Result:

- End-to-end measurement from detection to CRC.

### A3. Optional: RF Pre-Qualification per Gain Combination

Files:

- `crates/tetra-entities/src/phy/components/soapyio.rs`
- optional new component under `crates/tetra-entities/src/phy/components/` (for example `rx_rf_probe.rs`)

Steps:

- After gain or frequency changes, add a short time-based settling phase using hardware time (in addition to slot-based settling).
- Offer an optional RF probe mode before the actual measurement window:
  - capture a short I/Q window,
  - correlate against a reference tone,
  - log normalized correlation level as `rf_probe_db`.
- Use `rf_probe_db` only for diagnostics (for example detecting obvious clipping/underdrive), not as primary ranking criterion.

Result:

- Faster plausibility checks and more robust measurement-window starts after switching events.

## Phase B - Measurement Mode for Gain Sweeps

### B0. Explicit RX Test Start Mode (switch)

Files:

- `bins/bluestation-bs/src/main.rs`
- `crates/tetra-config/src/bluestation/config.rs`

Steps:

- Introduce an explicit program switch for RX test mode, for example `--rx-gain-test`.
- Without switch, BlueStation runs unchanged in normal operating mode.
- With switch, a clearly separate test workflow is activated:
  - start sweep,
  - process measurement windows,
  - print result report,
  - cleanly terminate process with exit code 0.
- Handle error cases (invalid configuration, no valid gain elements, no samples) with non-zero exit code.

Result:

- Confusion with normal operation is avoided; the test run is reproducible and unambiguous.

### B1. Range-Based Gain Sweep Configuration

Files:

- `crates/tetra-config/src/bluestation/sec_phy_soapy.rs`
- `crates/tetra-config/src/bluestation/sec_phy.rs`
- `example_config/config.toml`

Steps:

- Introduce an optional measurement mode, for example `rx_gain_sweep`, with range definitions per gain element.
- For each available gain element (device-dependent), allow an optional range:
  - `from`,
  - `to`,
  - `step`.
- Example idea (conceptual):
  - if only `pga` is present: sweep only over `pga`.
  - if `lna` and `pga` are present: sweep over both ranges (cartesian product).
- Offer optional search strategy mode:
  - `grid` (all combinations),
  - later optionally `coordinate_descent` (faster for large search spaces).
- Keep backward compatibility: if `rx_gain_sweep` is not set, preserve current static behavior with `rx_gain_*`.

Proposed configuration schema:

```toml
[phy_io.soapysdr.rx_gain_sweep]
enabled = true
strategy = "grid"         # optional, default: grid
window_bursts = 500        # bursts per measurement window
settling_slots = 8         # slots to discard after gain change
auto_exit = true           # test mode exits process after final report

# test signal characteristics for evaluation
test_signal_profile = "t1_all_ul_slots"  # for example t1_all_ul_slots, t1_single_slot
required_ul_slots = [0, 1, 2, 3]
min_slot_crc_pass_rate = 0.95

# optional: assumed test device transmit power for this run
# (included in result report)
test_tx_power_dbm = -85.0

# test-level label for this run (level at DUT input)
# one run == one level point; multiple level points are executed as multiple runs.
test_level_dbm = -95.0

# specify only gains that the device really has
[phy_io.soapysdr.rx_gain_sweep.gains.pga]
from = 0.0
to = 30.0
step = 1.0

[phy_io.soapysdr.rx_gain_sweep.gains.lna]
from = 0.0
to = 30.0
step = 3.0
```

Result:

- Automatic generation of gain combinations to test from range definitions.

### B1.1 Capture Test Device Power (config or input)

Files:

- `bins/bluestation-bs/src/main.rs`
- `crates/tetra-config/src/bluestation/sec_phy_soapy.rs`

Steps:

- Make test device transmit power mandatory metadata for RX test runs.
- Additionally require `test_level_dbm` (effective level at DUT input) as mandatory metadata.
- Source priority:
  1. CLI argument (for example `--test-tx-power-db`, `--test-level-db`),
  2. configuration value (`test_tx_power_dbm`, `test_level_dbm`).
- If no source is available, abort test-run startup with a clear error message.
- No interactive input is allowed once the test run has started (autonomous operation).

Result:

- Each run is traceable to a documented transmit power value.
- Each run is clearly assigned to one level point.

### B2. Runtime Switching of RX Gains

Files:

- `crates/tetra-entities/src/phy/components/soapyio.rs`
- `crates/tetra-entities/src/phy/components/soapy_dev.rs`

Steps:

- Build an internal runtime interface for controlled switching of RX gain combinations between measurement windows (no external user API).
- Switch only at safe boundaries (for example slot boundaries) and with settling guard (discard x slots).
- Introduce a sweep state machine:
  - `ApplyGains`,
  - `Settling`,
  - `MeasureWindow`,
  - `FinalizeAndNext`.
- Evaluate a fixed burst count per combination (`window_bursts`).
- After test-mode start, the state machine runs fully automatically until final report and process exit.
- After sweep end, automatic best-value determination:
  - primarily by `crc_pass_rate`,
  - for T1 with `required_ul_slots=[0,1,2,3]` additionally allow only candidates that are 4/4 stable,
  - on ties by lower `false_positive_rate`,
  - optionally by lower mean `train_errs`,
  - use `rf_probe_db` only as diagnostic/plausibility filter (optional), not as primary ranking criterion.
- Output best gain set in result logs; optionally also write it to a result file for next startup.

Result:

- Fully automated sweep without process restart, including automatic best gain selection.

### B3. Test Device Specific Architecture (extensible)

Files:

- new component, for example `crates/tetra-entities/src/phy/components/rx_test_devices.rs`
- `bins/bluestation-bs/src/main.rs`

Steps:

- Define an abstract interface for test devices, for example:
  - `device_name()`,
  - `burst_mode()` (bursted/continuous),
  - `metadata()` (for example tx_power_dbm, pattern, optional slot info),
  - `validate_for_run()`.
- First implementation: `Stabilock4032Adapter`.
- `Stabilock4032Adapter` should separate two layers:
  - `operation_profile` (from main manual: RX mask/special logic),
  - `signal_profile` (from TETRA BS supplement: TCH/7.2, SCH/F, bit pattern/PN9).
- Default path should allow future adapters (for example R&S, Aeroflex, Keysight) without touching sweep core logic.
- Result report also stores `test_device_type` and `test_signal_mode`.

Result:

- Other test devices can be cleanly added through new adapter classes.

## Phase C - Aggregation and Result File

### C1. In-Memory Statistics

Files:

- `crates/tetra-entities/src/phy/phy_bs.rs`
- optionally new component under `crates/tetra-entities/src/phy/components/`

Steps:

- Build counters per (gain_combo, test_level_dbm, burst_type, train_type):
  - expected_slots,
  - detected,
  - decoded,
  - crc_ok,
  - false_positive.
- Rolling window for fast visibility + cumulative values for final report.
- Additionally maintain per-slot counters for UL slots 0..3 so 4/4 stability can be evaluated.

### C2. Export for Offline Evaluation

Files:

- new file, for example `crates/tetra-entities/src/phy/components/rx_gain_stats.rs`

Steps:

- Periodic CSV/JSON export (machine-readable), for example:
  - `timestamp, gain_combo, test_level_dbm, burst, detected, crc_ok_rate, slot0_crc_rate, slot1_crc_rate, slot2_crc_rate, slot3_crc_rate, train_err_mean, fp_rate, rf_probe_db, test_device_type, test_signal_mode, test_tx_power_dbm`.
- Additionally export final ranking, for example:
  - `rank, gain_combo, crc_ok_rate, fp_rate, train_err_mean, samples`.

Result:

- Plottable in external tools and reproducible for regressions.

### C3. Final Report and Process End

Files:

- `bins/bluestation-bs/src/main.rs`
- optional new utility file for report formatting

Steps:

- After all combinations are complete, print a compact final report:
  - best gain set,
  - top-3 ranking,
  - measured test device transmit power,
  - burst mode,
  - sample count and quality metrics,
  - 4/4 evaluation per level point (pass/fail) and lowest 4/4-stable level.
- Then terminate the process in RX test mode in a controlled way.

Result:

- Run is completed, results are immediately visible, no manual post-processing required.

## Phase D - Operator Workflow (Stabilock)

### D1. Defined Test Procedure

1. Configure Stabilock for fixed burst type and fixed rate (for example TCH/7.2 or SCH/F).
2. Start BlueStation in explicit RX test mode (CLI switch enabled).
3. Capture test device power (CLI, config, or input).
4. Set level point (`test_level_dbm`) for this run.
5. Within this level point:
  - run all automatically generated gain combinations,
  - collect at least N bursts per combination (recommended >= 500).
6. In T1 mode, reduce level step-by-step (for example from -40 dBm down toward around -137 dBm) until 4/4 stability is no longer reached.
7. Read final report after run end; process exits automatically.
8. For additional level points (for example 1 or 2 dB steps), reconfigure test device and repeat run.
9. Repeat measurement for increasing and decreasing levels (detect hysteresis/saturation).

### D2. Explicitly Test Near-Far Scenario (practical case)

Goal:

- Measure robustness against simultaneous strong and weak UL signals on different timeslots.

Proposed test:

1. Inject high level on timeslot A (near-MS equivalent).
2. Inject significantly lower level on timeslot B (far-MS equivalent).
3. Increase level difference in steps (for example 10, 20, 30 dB) while keeping average total level constant.
4. Per step, capture per-slot decode/CRC rates and check for failure of the weak slot.

Result:

- Dynamic range limit (near-far margin) available as additional operational metric.

### D3. Decision Rules

Recommended primary goals:

- maximize `crc_pass_rate`,
- minimize `false_positive_rate`,
- keep stable range with low variance of `train_errs`.

Secondary:

- largest usable level range (lower sensitivity limit to upper overload limit).

## 6. Definition of "meaningfully processable"

Pragmatic technical definition for the report:

- **Control bursts**: meaningfully processable if `crc_pass_rate >= 95%` over the measurement window.
- **Traffic bursts**: meaningfully processable if `decode_tp` is stable and `crc_ok` is in target range (define project-specific, for example >= 90%).
- **Not meaningful**: if only training is detected but CRC rate drops significantly or false positives increase.
- **T1 all-UL-slots**: meaningfully processable only if all required UL slots keep the minimum CRC value (4/4 criterion).

Note: intentionally implement thresholds as configurable parameters.

## 7. Test and Acceptance Plan

- Unit tests for new statistics and export logic.
- Integration test with `ul_input_file` (deterministic replay path) for reproducible comparisons.
- Field test with Stabilock:
  - at least 2 complete sweep runs,
  - compare top-2 profiles for repeatability.
- Near-far test:
  - at least one run with two simultaneously active UL slots and >= 20 dB level difference,
  - prove at which difference the weak slot starts failing.

Acceptance when:

- automatic sweep runs without crash,
- result file is created per sweep,
- clear "best profile" and reliable level limits are evaluable.

## 8. Risks and Mitigations

- **Detection thresholds too strict (`SEQ_*_MAX_ERRS = 1`)**:
  - mitigation: make thresholds configurable in measurement mode and evaluate separately.
- **Gain switching transients**:
  - mitigation: discard settling slots.
- **Nonlinear frontend effects at high levels**:
  - mitigation: observe train_errs/burst_pos drift in addition to CRC.
- **Near-far masking (strong slot suppresses weak slot)**:
  - mitigation: require per-slot metrics and dedicated near-far acceptance test.
- **Difference between lab setup and real world**:
  - mitigation: cross-check top profiles with real OTA signal.
- **Search space too large (many gain combinations)**:
  - mitigation: make maximum combination count configurable and optionally switch to faster search strategy (for example coordinate descent).

## 9. Recommended Implementation Order

1. Phase A (telemetry) - immediate value, low risk.
2. Phase B (test mode + sweep infrastructure) - provide run control and gain switching.
3. Phase C (aggregation/export) - build metrics and ranking on stable workflow.
4. Phase D (operator documentation + polishing).

## 10. Expected Result

After implementation, there is a reproducible RX tuning workflow that can objectively answer for Stabilock test bursts:

- Which RX gain combination is optimal?
- Where is the lower sensitivity limit?
- At which level do overdrive or decision errors begin?

This turns the current single log event

- `rx_tpsap_prim got NormalTrainSeq1 in fullslot`

into a reliable, quantitative decision basis for operation.

## 11. Gap-Detection System for Measurement Stability

### 11.1 Purpose

The gap-detection system validates measurement completeness by identifying missing timeslots during a measurement window. It provides an objective metric to assess RF stability and signal reception quality for each gain combination.

**Problem it solves:**

- High gap rates indicate RF instability, reception dropout, or excessive burst collisions.
- Without gap-detection, a low CRC rate could be incorrectly attributed to gain settings instead of measurement quality issues.
- Gap-rate provides an independent stability assessment orthogonal to CRC-based ranking.

### 11.2 Terminology

- **Timeslot**: A TDMA timeslot in the TETRA frame structure. TETRA uses timeslots numbered 1-4 per frame, 1-18 frames per multiframe.
- **Expected slots**: The target number of bursts expected during a measurement window (configured as `window_bursts`).
- **Detected bursts**: Actual bursts received and processed by the PHY layer during the measurement window.
- **Gap**: A missing burst in the continuous TDMA sequence. Consecutive detected timeslots should form an unbroken sequence; any deviation indicates a gap.
- **Gap rate**: `(gaps_detected / expected_slots) * 100` percentage, indicating measurement quality.
- **Stability status**: 
  - `STABLE`: gap_rate < 3% (acceptable measurement quality)
  - `UNSTABLE`: gap_rate ≥ 3% (potential RF issues or receiver problems)

### 11.3 Gap Detection Algorithm

1. **Collect detected timeslots**: During the measurement window, every received burst (full slot or subslot) records its TDMA timestamp (`multiframe.frame.timeslot.subslot`).
2. **Sort timeslots**: Sort all detected timestamps by multiframe, then frame, then timeslot.
3. **Detect gaps**: Iterate through consecutive timeslots. A gap exists if:
   - Same frame: timeslot advances by more than 1 (e.g., slot 2 → slot 4).
   - Frame boundary: current is not slot 4 or next is not slot 1.
   - Frame discontinuity: frames differ by more than 1.
4. **Calculate metrics**:
   - `gaps_detected = count of identified gaps`
   - `gap_rate = gaps_detected / expected_slots`
   - `stability_status = STABLE if gap_rate < 0.03, else UNSTABLE`

### 11.4 CSV Export Format

The window and summary CSV exports include three new columns:

- `gaps_detected` (integer): Number of gaps found in the measurement window.
- `gap_rate` (float, 0.0-1.0): Gap rate as a fraction of expected slots.
- `stability_status` (string): "STABLE" or "UNSTABLE" classification.

Example window export row:
```
timestamp,...,gaps_detected,gap_rate,stability_status
1234567890,...,2,0.0400,"UNSTABLE"
```

### 11.5 Interpretation Guide

**When analyzing RX gain sweep results:**

- **All combos STABLE (gap_rate < 3%)**:
  - Measurement is reliable; ranking by CRC alone is valid.
  - Choose top-ranked combo by CRC performance.

- **Some combos UNSTABLE (gap_rate ≥ 3%)**:
  - These combos experienced RF dropout or collision issues.
  - Avoid selecting unstable combos, even if CRC rates appear good (likely due to incomplete measurement).
  - Prefer a stable combo with slightly lower CRC if necessary.

- **All combos UNSTABLE**:
  - Indicates systematic issue: antenna problem, TX/RX interference, or test signal too weak.
  - Recommendations:
    - Verify test signal level (`test_level_dbm`).
    - Check antenna connections and cable quality.
    - Verify no interference on test frequency.
    - Increase settling time (`settling_slots` config) to allow PLL stabilization.
    - Consider signal path checks (RF probe measurements).

### 11.6 Logging

During measurement finalization, gap-detection results are logged:
```
rx_gain_sweep_measure_done gaps_detected=2 gap_rate=4.00% stability=UNSTABLE
```

This allows operators to quickly identify problematic gain combinations in real-time logs.

### 11.7 Future Extensions (Tier 2+)

- **Temporal clustering**: Identify patterns in gap distribution (burst-starvation at specific times).
- **Adaptive burst increase**: Automatically increase window bursts if gaps exceed threshold.
- **Per-slot gap analysis**: Track gaps per timeslot (1-4) to identify slot-specific issues.
- **Ranking penalty**: Apply gap_rate penalty to score to prefer stable combos in borderline CRC cases.
