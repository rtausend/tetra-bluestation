use clap::Parser;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(name = "soapy-gain-setter")]
#[command(about = "Set RX gains on SoapySDR device")]
struct Args {
    /// Gain name and value pairs (e.g., PGA 12.0 LNA 36.0)
    #[arg(value_name = "NAME VALUE", num_args = 2.., trailing_var_arg = true)]
    gains: Vec<String>,

    /// Device select string (default: auto-detect)
    #[arg(short, long)]
    device: Option<String>,

    /// RX channel (default: 0)
    #[arg(short, long, default_value = "0")]
    channel: usize,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    if args.gains.len() < 2 || args.gains.len() % 2 != 0 {
        eprintln!("Error: Gains must be provided as name-value pairs");
        std::process::exit(1);
    }

    // Parse gains
    let mut gains: HashMap<String, f64> = HashMap::new();
    for i in (0..args.gains.len()).step_by(2) {
        let name = &args.gains[i];
        let value_str = &args.gains[i + 1];
        
        match value_str.parse::<f64>() {
            Ok(value) => {
                gains.insert(name.clone(), value);
            }
            Err(_) => {
                eprintln!("Error: Failed to parse gain value '{}' as float", value_str);
                std::process::exit(1);
            }
        }
    }

    // Open device (ignore device arg for now, will auto-detect)
    let dev = match soapysdr::Device::new(soapysdr::Args::new()) {
        Ok(dev) => dev,
        Err(err) => {
            eprintln!("Error: Failed to open SoapySDR device: {}", err);
            std::process::exit(1);
        }
    };

    let rx_ch = args.channel;

    // Apply gains
    for (name, value) in gains {
        match dev.set_gain_element(soapysdr::Direction::Rx, rx_ch, name.as_str(), value) {
            Ok(_) => {
                // Verify
                match dev.gain_element(soapysdr::Direction::Rx, rx_ch, name.as_str()) {
                    Ok(applied) => {
                        let delta = (applied - value).abs();
                        if delta <= 0.25 {
                            println!("{}: {} (applied: {}, delta: {})", name, value, applied, delta);
                        } else {
                            eprintln!("Error: Gain {} mismatch: requested {}, applied {}", name, value, applied);
                            std::process::exit(1);
                        }
                    }
                    Err(err) => {
                        eprintln!("Error: Failed to read back gain {}: {}", name, err);
                        std::process::exit(1);
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: Failed to set gain {}: {}", name, err);
                std::process::exit(1);
            }
        }
    }

    println!("All gains set successfully");
}
