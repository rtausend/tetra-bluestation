# Setting Up RX Gain Sweep on Device

## Prerequisites

You need a newer version of `bluestation-bs-rx-test` that includes the `--rx-gain-test-single-combo` CLI flag (version 0.5.9-c3c760e or later).

## Getting the Latest Binary

### Option 1: Build on Dev PC and Copy

On your **development PC**:

```bash
cd /Users/robert/Entwicklung/tetra-bluestation
git checkout feat/rx-gain-sweep
cargo build --release --bin bluestation-bs

# Copy to device (adjust IP/path as needed)
scp target/release/bluestation-bs dk5rta@mobile-tetra:/opt/tetra/bluestation-bs-rx-test
```

### Option 2: Cross-Compile for Device (aarch64)

If the device is ARM-based, use cross-compilation:

```bash
cd /Users/robert/Entwicklung/tetra-bluestation
git checkout feat/rx-gain-sweep

# Build for ARM64
cross build --release --bin bluestation-bs --target aarch64-unknown-linux-gnu

# Copy to device
scp target/aarch64-unknown-linux-gnu/release/bluestation-bs dk5rta@mobile-tetra:/opt/tetra/bluestation-bs-rx-test
```

## On the Device

### 1. Verify You Have the New Binary

```bash
cd /opt/tetra

# Check version (should show 0.5.9-c3c760e or later)
./bluestation-bs-rx-test --help 2>&1 | grep -A1 "Version:"

# Should see the new flag
./bluestation-bs-rx-test --help 2>&1 | grep "rx-gain-test-single-combo"
```

### 2. Clean Up Config

Remove the obsolete `restart_process_per_combo` field from `sweep_config.toml`:

**Option A: Manual Edit**
```bash
# Edit sweep_config.toml and remove this line:
#   restart_process_per_combo = true
```

**Option B: Use Script** (handles automatically)
The orchestrator script will clean this up automatically, so you don't need to do it manually.

### 3. Run the Sweep

```bash
cd /opt/tetra

# Test with a few combos first (quick)
python3 sweep.py sweep_config.toml \
  --lna-from 24 --lna-to 24 --lna-step 12 \
  --pga-from 0 --pga-to 4 --pga-step 2 \
  --binary ./bluestation-bs-rx-test \
  --output test_results.csv
```

### 4. Full Sweep (all 48 combos in one CSV)

```bash
# Run all combos: LNA [24/36/48] × PGA [0/2/4/.../30] = 48 combos
python3 sweep.py sweep_config.toml \
  --lna-from 24 --lna-to 48 --lna-step 12 \
  --pga-from 0 --pga-to 30 --pga-step 2 \
  --binary ./bluestation-bs-rx-test \
  --delay 0.5 \
  --output sweep_full.csv
```

The results all go into **one CSV file** (`sweep_full.csv`) instead of separate files.

## What the Script Does

1. **Generates all gain combinations** from your parameters
2. **Cleans config** - removes obsolete fields automatically
3. **For each combo**:
   - Runs `bluestation-bs-rx-test` with `--rx-gain-test-single-combo "lna=X pga=Y"`
   - Waits for program to auto-exit after one measurement
   - Waits 1s for driver recovery
4. **Appends results** to timestamped CSV file
5. **Reports summary** (total, succeeded, failed)

## Output

Look for CSV files in the output directory specified by the program:

```
run_20250505_120000.csv

Columns:
  run_id                  - Sweep run identifier
  timestamp               - When measurement occurred
  combo_idx               - Combination index (0-based)
  lna_gain_db            - LNA setting in dB
  pga_gain_db            - PGA setting in dB
  detect_rate            - Percentage of bursts detected
  gap_rate               - Percentage of gaps in signal
  crc_pass_rate          - Percentage of passing CRCs
  duration_ms            - Measurement duration in milliseconds
  status                 - STABLE or UNSTABLE
```

## Troubleshooting

### Script Says "No such file or directory"

The script resolves paths, but make sure the binary actually exists:

```bash
ls -la /opt/tetra/bluestation-bs-rx-test
```

### "Unrecognized fields" Error

The config has obsolete fields. Either:
- Let the script clean it automatically (it does), or
- Manually edit `sweep_config.toml` and remove `restart_process_per_combo`

### "Invalid rx-gain-test setup"

The config is missing `rx_gain_sweep` section. Add this to sweep_config.toml:

```toml
[phy_io.soapysdr.rx_gain_sweep]
enabled = true
strategy = "grid"
window_bursts = 300
settling_slots = 8
auto_exit = true
test_signal_profile = "t1_all_ul_slots"
test_device_type = "stabilock4032"
required_ul_slots = [0, 1, 2, 3]
min_slot_crc_pass_rate = 0.95
test_tx_power_dbm = -85.0
test_level_dbm = -95.0

[phy_io.soapysdr.rx_gain_sweep.gains.pga]
from = 0.0
to = 30.0
step = 2.0

[phy_io.soapysdr.rx_gain_sweep.gains.lna]
from = 24.0
to = 48.0
step = 12.0
```

### Program Times Out

Each combo has a 120s timeout. If signal is weak or test signal missing, it might timeout. Check:
- Test signal is transmitting
- Device is receiving properly
- LNA/PGA values are in valid range

### Device Seems Stuck

Press Ctrl+C to stop the sweep. It will exit cleanly and report progress so far.

---

**Questions?** Check the [RX Gain Sweep Orchestrator documentation](./RX_GAIN_SWEEP_ORCHESTRATOR.md)
