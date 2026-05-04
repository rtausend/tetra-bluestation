#!/bin/bash
# RX Gain Sweep Orchestrator (Bash version)
# Runs bluestation-bs with different gain combinations in sequence
# Each run tests a single gain combo and exits automatically

set -e

BINARY="${BINARY:-./target/release/bluestation-bs}"
CONFIG="${CONFIG:-./example_config/config.toml}"
DELAY="${DELAY:-1}"

LNA_FROM="${LNA_FROM:-24}"
LNA_TO="${LNA_TO:-48}"
LNA_STEP="${LNA_STEP:-12}"

PGA_FROM="${PGA_FROM:-0}"
PGA_TO="${PGA_TO:-30}"
PGA_STEP="${PGA_STEP:-2}"

# Validate paths
if [[ ! -f "$CONFIG" ]]; then
    echo "ERROR: Config file not found: $CONFIG"
    exit 1
fi

if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: Binary not found: $BINARY"
    exit 1
fi

# Count total combinations
total=0
for ((lna = LNA_FROM; lna <= LNA_TO; lna += LNA_STEP)); do
    for ((pga = PGA_FROM; pga <= PGA_TO; pga += PGA_STEP)); do
        ((total++))
    done
done

echo "=================================="
echo "RX Gain Sweep Orchestrator"
echo "=================================="
echo "Binary:  $BINARY"
echo "Config:  $CONFIG"
echo "Total combos: $total"
echo "LNA: $LNA_FROM to $LNA_TO dB (step $LNA_STEP)"
echo "PGA: $PGA_FROM to $PGA_TO dB (step $PGA_STEP)"
echo "Delay:   ${DELAY}s"
echo "=================================="
echo

counter=0
succeeded=0
failed=0

for ((lna = LNA_FROM; lna <= LNA_TO; lna += LNA_STEP)); do
    for ((pga = PGA_FROM; pga <= PGA_TO; pga += PGA_STEP)); do
        ((counter++))
        
        echo ""
        echo "=========================================="
        echo "[$counter/$total] LNA=$lna, PGA=$pga"
        echo "=========================================="
        
        if "$BINARY" "$CONFIG" --rx-gain-test-single-combo "lna=$lna pga=$pga"; then
            ((succeeded++))
        else
            echo "ERROR: Combo $counter failed!"
            ((failed++))
        fi
        
        # Wait for driver recovery
        if [[ $counter -lt $total ]]; then
            echo "Waiting ${DELAY}s for driver recovery..."
            sleep "$DELAY"
        fi
    done
done

echo ""
echo "=========================================="
echo "Sweep Complete!"
echo "=========================================="
echo "Total:    $total"
echo "Success:  $succeeded"
echo "Failed:   $failed"
echo "=========================================="
echo

if [[ $failed -eq 0 ]]; then
    exit 0
else
    exit 1
fi
