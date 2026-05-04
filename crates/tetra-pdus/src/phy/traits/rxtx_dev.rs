use std::collections::HashMap;

use tetra_core::TdmaTime;
use tetra_core::TrainingSequence;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RxTxDevError {
    RxEndOfData,
    RxReadError,
}

#[derive(Debug, Default)]
pub struct RxBurstBits<'a> {
    pub train_type: TrainingSequence,
    /// Number of bit errors for the selected training sequence match.
    pub train_errs: usize,
    /// Position (in demodulated bits) where the burst starts inside the analyzed slot window.
    pub burst_pos: usize,
    /// Burst length in bits.
    pub burst_len: usize,
    pub bits: &'a [u8],
}

#[derive(Debug, Default)]
pub struct RxSlotBits<'a> {
    /// Number of slot received
    pub time: TdmaTime,
    /// Burst received in full slot
    pub slot: RxBurstBits<'a>,
    /// Burst received in subslot 1
    pub subslot1: RxBurstBits<'a>,
    /// Burst received in subslot 2
    pub subslot2: RxBurstBits<'a>,
}

#[derive(Debug, Default)]
pub struct TxSlotBits<'a> {
    /// Number of slot to transmit
    pub time: TdmaTime,
    /// Burst to transmit in full slot
    pub slot: Option<&'a [u8]>,
    // /// Burst to transmit in subslot 1
    // pub subslot1: Option<&'a [u8]>,
    // /// Burst to transmit in subslot 2
    // pub subslot2: Option<&'a [u8]>,
}

/// Trait for RX/TX devices that work with full slots.
pub trait RxTxDev {
    fn rxtx_timeslot(&mut self, tx_slot: &[TxSlotBits]) -> Result<Vec<Option<RxSlotBits<'_>>>, RxTxDevError>;

    /// Internal runtime hook for autonomous RX gain sweeps.
    fn apply_rx_gain_combo(&mut self, _gains: &HashMap<String, f64>) -> Result<(), RxTxDevError> {
        Ok(())
    }

    /// Optional runtime hook to reinitialize the device between gain combos.
    fn reinitialize_for_gain_sweep(&mut self) -> Result<(), RxTxDevError> {
        Ok(())
    }

    /// Optional runtime hook to reinitialize and apply gain combo atomically.
    fn reinitialize_and_apply_rx_gain_combo(&mut self, gains: &HashMap<String, f64>) -> Result<(), RxTxDevError> {
        self.reinitialize_for_gain_sweep()?;
        self.apply_rx_gain_combo(gains)
    }
}
