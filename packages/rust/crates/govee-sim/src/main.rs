//! Run a fake device on the real ports, for manual testing and the playground.
//!
//! ```text
//! govee-sim [--id <MAC>] [--sku <SKU>] [--ip <ADDR>] [--silent] [--latency-ms <N>]
//! ```
//!
//! Binding 4001 and 4003 needs no privileges, but a real device on the same
//! network answering the same scan will also reply — run this on a network
//! without one, or expect two devices.

// A binary reports and exits; the no-panic rule that protects a host
// application from a library does not apply here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use govee_sim::{Faults, Options, Simulator};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut id = "AA:BB:CC:DD:EE:FF".to_owned();
    let mut sku = "H61A0".to_owned();
    let mut ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let mut faults = Faults::default();

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().unwrap_or_default();
        match flag.as_str() {
            "--id" => id = value(),
            "--sku" => sku = value(),
            "--ip" => match value().parse() {
                Ok(parsed) => ip = parsed,
                Err(e) => return Err(std::io::Error::other(format!("--ip: {e}"))),
            },
            "--silent" => faults.silent = true,
            "--latency-ms" => {
                faults.latency = Duration::from_millis(value().parse().unwrap_or_default());
            }
            "--help" | "-h" => {
                println!(
                    "govee-sim [--id <MAC>] [--sku <SKU>] [--ip <ADDR>] [--silent] [--latency-ms <N>]"
                );
                return Ok(());
            }
            other => return Err(std::io::Error::other(format!("unknown flag `{other}`"))),
        }
    }

    let simulator = Simulator::start(Options {
        faults,
        ..Options::on_the_network(&id, &sku, ip)
    })
    .await?;

    println!(
        "{sku} {id} at {ip}\n  discovery {}\n  control   {}",
        simulator.scan_addr()?,
        simulator.control_addr()?
    );
    tokio::signal::ctrl_c().await?;
    println!("{} datagrams received", simulator.received_count());
    Ok(())
}
