use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use http::Uri;
use log::{debug, error};
use macaddr::MacAddr6;

/// Wake‑on‑WAN command‑line interface
#[derive(Parser, Debug)]
#[command(name = "wakeonwan")]
#[command(about = "Send Wake‑On‑LAN packets over a network.", long_about = None)]
pub struct Args {
    /// Destination uri
    #[arg(short = 'i', long = "uri", default_value = "255.255.255.255")]
    host: Uri,

    /// Destination port
    #[arg(short = 'p', long = "port", default_value_t = 9)]
    port: u16,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Do not actually send the packet (dry‑run)
    #[arg(short = 'D', long)]
    dry_run: bool,

    /// MAC address(es) to wake.
    #[arg(required = true)]
    mac: Vec<MacAddr6>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    env_logger::Builder::from_default_env()
        .filter(
            None,
            if args.verbose {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            },
        )
        .init();

    let host = args
        .host
        .host()
        .ok_or_else(|| anyhow!("Uri {} has no hostname!", args.host))?
        .to_string();
    debug!("Resolved uri {} to hostname {}", args.host, host);

    let dest = resolve_destination(host.as_str(), args.port)?;
    debug!("Resolved hostname {} to ip {}", host, dest.ip());

    let src = match dest {
        SocketAddr::V4(_) => UdpSocket::bind("0.0.0.0:0")?,
        SocketAddr::V6(_) => UdpSocket::bind(("::", 0))?,
    };
    debug!("Bound to {}", src.local_addr().unwrap());

    if let SocketAddr::V4(_) = dest {
        src.set_broadcast(true)
            .map_err(|e| anyhow!("Failed to enable broadcast: {}", e))?;
    }

    send_magic_packets(src, dest, &args);
    Ok(())
}

pub fn resolve_destination(host: &str, port: u16) -> Result<SocketAddr> {
    let host_clean = host.trim_start_matches('[').trim_end_matches(']');

    (host_clean, port)
        .to_socket_addrs()
        .with_context(|| format!("Resolving {host}"))?
        .next()
        .ok_or_else(|| anyhow!("No addresses found for {host}"))
}

pub fn send_magic_packets(src: UdpSocket, dest: SocketAddr, cfg: &Args) {
    for mac in &cfg.mac {
        debug!("Sending magic packet to {} at {}", mac, dest);

        if cfg.dry_run {
            continue;
        }

        let pkt = magic_packet(mac.as_bytes());
        if let Err(e) = src.send_to(&pkt, dest) {
            error!("Can't send magic packet to {} on {}, {}", mac, dest, e);
        }
    }
}

pub fn magic_packet(mac: &[u8]) -> [u8; 102] {
    let mut pkt = [0u8; 102];
    pkt[0..6].copy_from_slice(&[0xFF; 6]);
    for i in 0..16 {
        pkt[6 + i * 6..12 + i * 6].copy_from_slice(mac);
    }
    pkt
}
