#!/usr/bin/env python3
"""
RX Gain Sweep Orchestrator
Runs bluestation-bs with different gain combinations in sequence.
Each run tests a single gain combo and exits automatically.
"""

import subprocess
import sys
import time
import argparse
from pathlib import Path

def expand_gain_values(from_val, to_val, step):
    """Generate list of gain values from range."""
    if step <= 0 or from_val > to_val:
        return []
    
    values = []
    eps = step * 1e-6 + 1e-9
    i = 0
    
    while True:
        value = from_val + step * i
        if value > to_val + eps:
            break
        values.append(min(value, to_val))
        i += 1
    
    if not values:
        values.append(from_val)
    
    if values[-1] < to_val - eps:
        values.append(to_val)
    
    return values

def format_float(f):
    """Format float value."""
    return f"{f:.2f}"

def generate_gain_combos(lna_from, lna_to, lna_step, pga_from, pga_to, pga_step):
    """Generate all gain combinations."""
    lna_values = expand_gain_values(lna_from, lna_to, lna_step)
    pga_values = expand_gain_values(pga_from, pga_to, pga_step)
    
    combos = []
    for lna in lna_values:
        for pga in pga_values:
            combos.append({
                'lna': lna,
                'pga': pga
            })
    
    return combos

def run_single_combo(binary_path, config_path, combo, combo_idx, total_combos):
    """Run bluestation-bs with a single gain combo."""
    combo_str = f"lna={format_float(combo['lna'])} pga={format_float(combo['pga'])}"
    
    print(f"\n{'='*70}")
    print(f"[{combo_idx+1}/{total_combos}] Running: {combo_str}")
    print(f"{'='*70}")
    
    cmd = [
        str(binary_path),
        str(config_path),
        "--rx_gain_test_single_combo",
        combo_str
    ]
    
    try:
        result = subprocess.run(cmd, timeout=120)  # 2 min timeout per combo
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        print(f"ERROR: Combo {combo_idx+1} timed out!")
        return False
    except Exception as e:
        print(f"ERROR: Failed to run combo {combo_idx+1}: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(
        description="Orchestrate RX gain sweep tests"
    )
    parser.add_argument(
        "config",
        help="Path to config.toml"
    )
    parser.add_argument(
        "--binary",
        default="./target/release/bluestation-bs",
        help="Path to bluestation-bs binary"
    )
    parser.add_argument(
        "--lna-from",
        type=float,
        default=24.0,
        help="LNA start value (dB)"
    )
    parser.add_argument(
        "--lna-to",
        type=float,
        default=48.0,
        help="LNA end value (dB)"
    )
    parser.add_argument(
        "--lna-step",
        type=float,
        default=12.0,
        help="LNA step size (dB)"
    )
    parser.add_argument(
        "--pga-from",
        type=float,
        default=0.0,
        help="PGA start value (dB)"
    )
    parser.add_argument(
        "--pga-to",
        type=float,
        default=30.0,
        help="PGA end value (dB)"
    )
    parser.add_argument(
        "--pga-step",
        type=float,
        default=2.0,
        help="PGA step size (dB)"
    )
    parser.add_argument(
        "--delay",
        type=float,
        default=1.0,
        help="Delay (seconds) between combos for driver recovery"
    )
    
    args = parser.parse_args()
    
    # Validate paths
    config_path = Path(args.config)
    binary_path = Path(args.binary)
    
    if not config_path.exists():
        print(f"ERROR: Config file not found: {config_path}")
        sys.exit(1)
    
    if not binary_path.exists():
        print(f"ERROR: Binary not found: {binary_path}")
        sys.exit(1)
    
    # Generate combos
    combos = generate_gain_combos(
        args.lna_from, args.lna_to, args.lna_step,
        args.pga_from, args.pga_to, args.pga_step
    )
    
    print(f"\n{'='*70}")
    print(f"RX Gain Sweep Orchestrator")
    print(f"{'='*70}")
    print(f"Config: {config_path}")
    print(f"Binary: {binary_path}")
    print(f"Total combos: {len(combos)}")
    print(f"LNA: {args.lna_from} to {args.lna_to} dB (step {args.lna_step})")
    print(f"PGA: {args.pga_from} to {args.pga_to} dB (step {args.pga_step})")
    print(f"Delay between combos: {args.delay}s")
    print(f"{'='*70}\n")
    
    # Run all combos
    succeeded = 0
    failed = 0
    
    for idx, combo in enumerate(combos):
        success = run_single_combo(binary_path, config_path, combo, idx, len(combos))
        
        if success:
            succeeded += 1
        else:
            failed += 1
        
        # Wait before next combo (allow driver to recover)
        if idx < len(combos) - 1:
            print(f"Waiting {args.delay}s for driver recovery...")
            time.sleep(args.delay)
    
    # Summary
    print(f"\n{'='*70}")
    print(f"Sweep Complete!")
    print(f"{'='*70}")
    print(f"Total:    {len(combos)}")
    print(f"Success:  {succeeded}")
    print(f"Failed:   {failed}")
    print(f"{'='*70}\n")
    
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()
