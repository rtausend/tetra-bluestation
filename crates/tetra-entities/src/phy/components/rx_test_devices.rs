use tetra_config::bluestation::CfgRxGainSweep;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxTestBurstMode {
    Bursted,
    Continuous,
}

#[derive(Debug, Clone)]
pub struct RxTestDeviceMetadata {
    pub test_device_type: String,
    pub operation_profile: String,
    pub signal_profile: String,
    pub burst_mode: RxTestBurstMode,
}

pub trait RxTestDeviceAdapter: Send + Sync {
    fn device_name(&self) -> &'static str;
    fn validate_for_run(&self, sweep: &CfgRxGainSweep) -> Result<(), String>;
    fn metadata(&self, sweep: &CfgRxGainSweep) -> RxTestDeviceMetadata;
}

pub struct Stabilock4032Adapter;

impl Stabilock4032Adapter {
    fn signal_profile(sweep: &CfgRxGainSweep) -> String {
        sweep
            .test_signal_profile
            .clone()
            .unwrap_or_else(|| "t1_all_ul_slots".to_string())
    }

    fn burst_mode_from_profile(profile: &str) -> RxTestBurstMode {
        match profile {
            "bit_pattern" => RxTestBurstMode::Continuous,
            _ => RxTestBurstMode::Bursted,
        }
    }
}

impl RxTestDeviceAdapter for Stabilock4032Adapter {
    fn device_name(&self) -> &'static str {
        "stabilock4032"
    }

    fn validate_for_run(&self, sweep: &CfgRxGainSweep) -> Result<(), String> {
        let profile = Self::signal_profile(sweep);
        let allowed = ["t1_all_ul_slots", "t1_single_slot", "tch_7_2", "sch_f", "bit_pattern"];
        if !allowed.iter().any(|p| *p == profile) {
            return Err(format!(
                "Unsupported test_signal_profile '{}' for {} (allowed: {:?})",
                profile,
                self.device_name(),
                allowed
            ));
        }

        Ok(())
    }

    fn metadata(&self, sweep: &CfgRxGainSweep) -> RxTestDeviceMetadata {
        let signal_profile = Self::signal_profile(sweep);
        RxTestDeviceMetadata {
            test_device_type: self.device_name().to_string(),
            operation_profile: "rx_mask_specials_gen_a_def_par".to_string(),
            burst_mode: Self::burst_mode_from_profile(&signal_profile),
            signal_profile,
        }
    }
}

pub fn build_adapter(device_type: Option<&str>) -> Result<Box<dyn RxTestDeviceAdapter>, String> {
    let dtype = device_type.unwrap_or("stabilock4032").to_ascii_lowercase();
    match dtype.as_str() {
        "stabilock4032" => Ok(Box::new(Stabilock4032Adapter)),
        _ => Err(format!("Unsupported test device type '{}'", dtype)),
    }
}
