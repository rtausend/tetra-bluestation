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
import shutil
import csv
import glob
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

def cleanup_config(config_path):
    """
    Clean up config file by removing obsolete fields.
    Creates a temporary cleaned copy if needed.
    """
    config_path = Path(config_path)
    config_text = config_path.read_text()
    original_text = config_text
    
    # Remove lines with obsolete fields
    obsolete_fields = [
        'restart_process_per_combo',
    ]
    
    lines = config_text.split('\n')
    cleaned_lines = []
    for line in lines:
        # Skip lines that contain obsolete field names
        if any(field in line for field in obsolete_fields):
            continue
        cleaned_lines.append(line)
    
    config_text = '\n'.join(cleaned_lines)
    
    # If we made changes, write to a temp file
    if config_text != original_text:
        temp_config = config_path.parent / f".{config_path.name}.cleaned"
        temp_config.write_text(config_text)
        return temp_config
    
    return config_path

def merge_csv_results(config_dir, output_csv):
    """
    Merge all generated CSV files into one master CSV.
    Looks for rx_gain_window_*.csv (detailed results) first, then rx_gain_run_*.csv.
    """
    config_dir = Path(config_dir)
    
    # Try to find window CSV files first (detailed per-combo measurements)
    csv_files = sorted(glob.glob(str(config_dir / "rx_gain_window_*.csv")))
    file_type = "window"
    
    # Fallback to run CSV files if window files not found
    if not csv_files:
        csv_files = sorted(glob.glob(str(config_dir / "rx_gain_run_*.csv")))
        file_type = "run"
    
    if not csv_files:
        print(f"No CSV files found in {config_dir}")
        print(f"  Searched for: rx_gain_window_*.csv and rx_gain_run_*.csv")
        return False
    
    all_rows = []
    header = None
    
    # Collect all rows from all CSV files
    for csv_file in csv_files:
        try:
            with open(csv_file, 'r') as f:
                reader = csv.DictReader(f)
                if header is None and reader.fieldnames:
                    header = reader.fieldnames
                for row in reader:
                    all_rows.append(row)
        except Exception as e:
            print(f"Warning: Could not read {csv_file}: {e}")
            continue
    
    if not all_rows or header is None:
        print(f"No valid CSV data found")
        return False
    
    # Write merged CSV
    try:
        with open(output_csv, 'w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=header)
            writer.writeheader()
            writer.writerows(all_rows)
        print(f"\n✓ Merged {len(csv_files)} {file_type} CSV files → {output_csv}")
        print(f"  Total rows: {len(all_rows)}")
        return True
    except Exception as e:
        print(f"ERROR: Failed to write merged CSV: {e}")
        return False

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

def run_single_combo(binary_path, config_path, combo, combo_idx, total_combos, output_csv=None, processed_files=None):
    """Run bluestation-bs with a single gain combo."""
    if processed_files is None:
        processed_files = set()
    
    combo_str = f"lna={format_float(combo['lna'])} pga={format_float(combo['pga'])}"
    
    print(f"\n{'='*70}")
    print(f"[{combo_idx+1}/{total_combos}] Running: {combo_str}")
    print(f"{'='*70}")
    
    # Resolve to absolute paths so subprocess can find them
    binary_abs = binary_path.resolve()
    config_abs = config_path.resolve()
    
    cmd = [
        str(binary_abs),
        str(config_abs),
        "--rx-gain-test-single-combo",
        combo_str
    ]
    
    try:
        result = subprocess.run(cmd, timeout=120)  # 2 min timeout per combo
        success = result.returncode == 0
        
        # If output_csv specified, collect results from newly generated files
        if output_csv and success:
            config_dir = config_abs.parent
            csv_files = sorted(glob.glob(str(config_dir / "rx_gain_window_*.csv")))
            
            for csv_file in csv_files:
                if csv_file not in processed_files:
                    processed_files.add(csv_file)
                    # Append this file's rows to output
                    try:
                        with open(csv_file, 'r') as f:
                            reader = csv.DictReader(f)
                            if reader.fieldnames:
                                rows = list(reader)
                                if rows:
                                    # Write header if output doesn't exist
                                    write_header = not Path(output_csv).exists()
                                    with open(output_csv, 'a', newline='') as out:
                                        writer = csv.DictWriter(out, fieldnames=reader.fieldnames)
                                        if write_header:
                                            writer.writeheader()
                                        writer.writerows(rows)
                    except Exception as e:
                        print(f"  Warning: Could not append {csv_file}: {e}")
        
        return success
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
        default=0.5,
        help="Delay (seconds) between combos for driver recovery"
    )
    parser.add_argument(
        "--output",
        help="Optional: Write all results to this CSV file instead of separate files"
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
    
    # Clean up config file (remove obsolete fields)
    print("Cleaning up config file...")
    config_path = cleanup_config(config_path)
    
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
    processed_files = set()
    
    for idx, combo in enumerate(combos):
        success = run_single_combo(binary_path, config_path, combo, idx, len(combos), 
                                   output_csv=args.output, processed_files=processed_files)
        
        if success:
            succeeded += 1
        else:
            failed += 1
        
        # Wait before next combo (allow driver to recover)
        if idx < len(combos) - 1:
            print(f"Waiting {args.delay}s for driver recovery...")
            time.sleep(args.delay)
    
    # Clean up temporary config file if it was created
    if config_path.name.startswith('.'):
        try:
            config_path.unlink()
        except Exception:
            pass
    
    # Summary
    print(f"\n{'='*70}")
    print(f"Sweep Complete!")
    print(f"{'='*70}")
    print(f"Total:    {len(combos)}")
    print(f"Success:  {succeeded}")
    print(f"Failed:   {failed}")
    if args.output:
        output_path = Path(args.output).resolve()
        print(f"Output:   {output_path}")
        if output_path.exists():
            # Count rows in output file
            try:
                with open(output_path, 'r') as f:
                    row_count = sum(1 for _ in f) - 1  # -1 for header
                print(f"Rows:     {row_count}")
            except:
                pass
    print(f"{'='*70}\n")
    
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()
