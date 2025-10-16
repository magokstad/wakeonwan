use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use http::Uri;
use log::{debug, error, info};
use macaddr::MacAddr6;

/// Wake‑on‑WAN command‑line interface
#[derive(Parser, Debug)]
#[command(name = "wakeonwan")]
#[command(about = "Send Wake‑On‑LAN packets over a network.", long_about = None)]
#[command(version = "0.1.1")]
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

    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", if args.verbose { "trace" } else { "off" });
    env_logger::init_from_env(env);

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

/// Resolves a hostname or IP address to a socket address.
///
/// This function accepts both IPv4 and IPv6 addresses, with or without brackets.
/// It also performs DNS resolution for hostnames.
///
/// # Arguments
///
/// * `host` - The hostname or IP address to resolve (brackets are automatically stripped)
/// * `port` - The port number to use
///
/// # Returns
///
/// Returns a `SocketAddr` on success, or an error if resolution fails.
///
/// # Examples
///
/// ```ignore
/// let addr = resolve_destination("192.168.1.1", 9)?;
/// let addr = resolve_destination("[::1]", 9)?;
/// let addr = resolve_destination("my-host.local", 9)?;
/// ```
pub fn resolve_destination(host: &str, port: u16) -> Result<SocketAddr> {
    let host_clean = host.trim_start_matches('[').trim_end_matches(']');

    (host_clean, port)
        .to_socket_addrs()
        .with_context(|| format!("Resolving {host}"))?
        .next()
        .ok_or_else(|| anyhow!("No addresses found for {host}"))
}

/// Sends Wake-on-LAN magic packets to one or more MAC addresses.
///
/// # Arguments
///
/// * `src` - The UDP socket to send packets from
/// * `dest` - The destination socket address (IP and port)
/// * `cfg` - The command-line arguments containing MAC addresses and options
///
/// # Behavior
///
/// - Logs each packet send attempt
/// - In dry-run mode, only logs without actually sending
/// - Errors during send are logged but don't stop subsequent sends
pub fn send_magic_packets(src: UdpSocket, dest: SocketAddr, cfg: &Args) {
    for mac in &cfg.mac {
        info!("Sending magic packet to {} at {}", mac, dest);

        if cfg.dry_run {
            continue;
        }

        let pkt = magic_packet(mac.as_bytes());
        if let Err(e) = src.send_to(&pkt, dest) {
            error!("Can't send magic packet to {} on {}, {}", mac, dest, e);
        }
    }
}

/// Constructs a Wake-on-LAN magic packet for the given MAC address.
///
/// A magic packet consists of:
/// - 6 bytes of 0xFF (synchronization stream)
/// - The target MAC address repeated 16 times (96 bytes)
///
/// Total packet size: 102 bytes
///
/// # Arguments
///
/// * `mac` - A 6-byte slice containing the MAC address
///
/// # Returns
///
/// A 102-byte array containing the magic packet
///
/// # Examples
///
/// ```ignore
/// let mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
/// let packet = magic_packet(&mac);
/// ```
pub fn magic_packet(mac: &[u8]) -> [u8; 102] {
    let mut pkt = [0u8; 102];
    pkt[0..6].copy_from_slice(&[0xFF; 6]);
    for i in 0..16 {
        pkt[6 + i * 6..12 + i * 6].copy_from_slice(mac);
    }
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_packet_structure() {
        let mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let packet = magic_packet(&mac);

        // Verify packet length
        assert_eq!(packet.len(), 102);

        // Verify first 6 bytes are 0xFF (sync stream)
        for i in 0..6 {
            assert_eq!(packet[i], 0xFF, "Sync stream byte {} should be 0xFF", i);
        }

        // Verify MAC address is repeated 16 times
        for i in 0..16 {
            let offset = 6 + i * 6;
            assert_eq!(
                &packet[offset..offset + 6],
                &mac,
                "MAC repetition {} should match original MAC",
                i
            );
        }
    }

    #[test]
    fn test_resolve_destination_ipv4() {
        let result = resolve_destination("127.0.0.1", 9);
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 9);
        assert!(addr.is_ipv4());
    }

    #[test]
    fn test_resolve_destination_ipv6() {
        let result = resolve_destination("::1", 9);
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 9);
        assert!(addr.is_ipv6());
    }

    #[test]
    fn test_resolve_destination_with_brackets() {
        let result = resolve_destination("[::1]", 9);
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 9);
        assert!(addr.is_ipv6());
    }

    #[test]
    fn test_resolve_destination_invalid() {
        let result = resolve_destination("invalid.hostname.that.does.not.exist.local", 9);
        assert!(result.is_err());
    }
}
