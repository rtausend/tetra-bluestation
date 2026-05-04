#!/bin/bash
# Quick setup script for device - copies latest orchestrator and binary from dev PC

set -e

echo "=================================="
echo "TETRA BlueStation RX Gain Sweep"
echo "Quick Setup for Device"
echo "=================================="
echo

# Check if running on device
if [[ ! -d "/opt/tetra" ]]; then
    echo "ERROR: Not in /opt/tetra directory"
    echo "Run this script on the target device in /opt/tetra"
    exit 1
fi

# Backup old config
if [[ -f "sweep_config.toml" ]]; then
    echo "Backing up existing sweep_config.toml..."
    cp sweep_config.toml sweep_config.toml.backup
fi

echo
echo "Expected file layout on device (/opt/tetra/):"
echo "  bluestation-bs-rx-test         (binary with new CLI support)"
echo "  sweep_config.toml              (config - will be auto-cleaned)"
echo "  sweep.py                       (orchestrator script)"
echo

# Make sweep.py executable
chmod +x sweep.py 2>/dev/null || true

echo "Setup complete!"
echo
echo "Usage:"
echo "  ./sweep.py sweep_config.toml \\"
echo "    --lna-from 24 --lna-to 48 --lna-step 12 \\"
echo "    --pga-from 0 --pga-to 30 --pga-step 2 \\"
echo "    --binary ./bluestation-bs-rx-test"
echo
echo "Features:"
echo "  ✓ Auto-cleans obsolete config fields"
echo "  ✓ Resolves relative binary paths"
echo "  ✓ 1s delay between combos for driver recovery"
echo "  ✓ Detailed progress output"
echo "  ✓ Results append to CSV with run_id timestamp"
echo
