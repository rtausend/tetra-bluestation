# RX Gain Sweep Orchestrator Scripts

External orchestration scripts for RX gain sweep testing. Each script generates gain combinations and calls `bluestation-bs` multiple times with different parameters.

## Key Concept

Instead of internal per-combo device restarts (which can cause state issues), these scripts:
1. Generate all gain combinations
2. Call `bluestation-bs` once per combo with `--rx-gain-test-single-combo "lna=X pga=Y"`
3. Each run measures **one combo only**, then **auto-exits**
4. Wait (configurable, default 0.5s) between combos for driver recovery
5. Results accumulate in the output CSV

**Timing Control**: The orchestrator script fully controls the delay between combos via the `--delay` parameter. The Rust program does not add any additional wait time - it immediately reinitializes the device after shutdown.

## Usage: Python Script (Recommended)

```bash
# Quick test with a few combos
./scripts/rx-gain-sweep-orchestrator.py \
    ./example_config/config.toml \
    --lna-from 24 --lna-to 24 --lna-step 12 \
    --pga-from 0 --pga-to 4 --pga-step 2 \
    --delay 0.5

# Full sweep with merged output
./scripts/rx-gain-sweep-orchestrator.py \
    ./example_config/config.toml \
    --lna-from 24 --lna-to 48 --lna-step 12 \
    --pga-from 0 --pga-to 30 --pga-step 2 \
    --binary ./target/release/bluestation-bs \
    --delay 0.5 \
    --output results_full_sweep.csv
```

### Parameters

- `config` (required): Path to config.toml
- `--binary`: Path to bluestation-bs binary (default: `./target/release/bluestation-bs`)
- `--lna-from`: LNA start (dB) - default 24
- `--lna-to`: LNA end (dB) - default 48
- `--lna-step`: LNA step (dB) - default 12
- `--pga-from`: PGA start (dB) - default 0
- `--pga-to`: PGA end (dB) - default 30
- `--pga-step`: PGA step (dB) - default 2
- `--delay`: Wait between combos (seconds) - default 0.5
- `--output`: Optional: Merge all results into single CSV file

## Usage: Bash Script

Environment variables for bash version:

```bash
BINARY=./target/release/bluestation-bs \
CONFIG=./example_config/config.toml \
LNA_FROM=24 LNA_TO=48 LNA_STEP=12 \
PGA_FROM=0 PGA_TO=30 PGA_STEP=2 \
DELAY=1 \
./scripts/rx-gain-sweep-orchestrator.sh
```

## Config Setup

Your config.toml must have `rx_gain_sweep` section **enabled**:

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

## How It Works

### Single-Combo CLI Flag

The new `--rx-gain-test-single-combo "lna=X pga=Y"` flag:
- Overrides the sweep configuration to measure **only one combo**
- Sets `auto_exit=true` to terminate after measurement
- Results go into the timestamped CSV in the output directory

### Orchestrator Script Flow

```
1. Generate all combos from [LNA_from, LNA_to, LNA_step] × [PGA_from, PGA_to, PGA_step]
2. For each combo:
   - Run: bluestation-bs config.toml --rx-gain-test-single-combo "lna=X pga=Y"
   - Wait for exit (program auto-exits after one measurement)
   - Wait DELAY seconds for driver recovery
3. After all combos:
   - Print summary (total, succeeded, failed)
   - Exit with status 0 if all succeeded, 1 if any failed
```

## Output

By default, each program run creates its own timestamped CSV file in the output directory:
```
rx_gain_run_20250101_120000.csv
rx_gain_run_20250101_120023.csv
rx_gain_run_20250101_120045.csv
...
```

**To merge all results into a single CSV**, use the `--output` flag:
```bash
./scripts/rx-gain-sweep-orchestrator.py config.toml \
    --output sweep_results.csv
```

This creates a single `sweep_results.csv` with all measurements:
```
run_id,timestamp,combo_idx,lna_gain_db,pga_gain_db,detect_rate,%,gap_rate,%,crc_pass_rate,%,duration_ms,status
run_20250101_120000,2025-01-01T12:00:01Z,0,24.0,0.0,94.2,2.1,98.5,18500,STABLE
run_20250101_120000,2025-01-01T12:00:23Z,1,24.0,2.0,95.1,1.8,99.2,19200,STABLE
...
```

## Advantages Over Internal Restarts

✅ Each combo starts with completely fresh device initialization  
✅ No complex internal state management between runs  
✅ Easier debugging - each run is independent  
✅ Better isolation if device acts up on one combo  
✅ Can easily pause/resume sweep by manually running combos  
✅ Results accumulate automatically in single CSV  
✅ **Fully configurable delay via `--delay` parameter** (0.5s default, adjust as needed)  
✅ No need to recompile for different timing configurations  

## Example: Quick Test with 3 Combos

```bash
python3 scripts/rx-gain-sweep-orchestrator.py \
    ./example_config/config.toml \
    --lna-from 24 --lna-to 24 --lna-step 12 \
    --pga-from 0 --pga-to 4 --pga-step 2
```

This runs exactly 3 combos: (LNA=24, PGA=0), (24, 2), (24, 4)

---

**Author:** Built for tetra-bluestation RX gain sweep testing  
**Language:** Python 3 (primary), Bash (alternative)
