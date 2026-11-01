//! Finds this TV's wireless ADB port. Termux's own adb can't, and nothing else there can read it.
use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener},
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;
use tokio::time::timeout;

pub const CONNECT_SERVICE: &str = "_adb-tls-connect._tcp.local";
pub const PAIRING_SERVICE: &str = "_adb-tls-pairing._tcp.local";

/// A one-question DNS query for `name`, asking for a PTR record with a unicast answer.
pub fn query(name: &str) -> Vec<u8> {
    let mut packet = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in name.split('.') {
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.extend_from_slice(&[0, 0, 12, 0x80, 1]);
    packet
}

/// The offset just past the name at `offset`, following no pointers.
fn skip_name(packet: &[u8], mut offset: usize) -> Option<usize> {
    loop {
        let size = *packet.get(offset)?;
        if size & 0xC0 == 0xC0 {
            return Some(offset + 2);
        }
        offset += 1;
        if size == 0 {
            return Some(offset);
        }
        offset += usize::from(size);
    }
}

/// Every port in the SRV records of an mDNS answer. Malformed packets give what was read so far.
pub fn ports(packet: &[u8]) -> Vec<u16> {
    let mut found = Vec::new();
    let number = |at: usize| Some(u16::from_be_bytes([*packet.get(at)?, *packet.get(at + 1)?]));
    let (Some(questions), Some(answers), Some(authority), Some(extra)) =
        (number(4), number(6), number(8), number(10))
    else {
        return found;
    };
    let mut offset = 12;
    for _ in 0..questions {
        let Some(next) = skip_name(packet, offset) else {
            return found;
        };
        offset = next + 4;
    }
    for _ in 0..u32::from(answers) + u32::from(authority) + u32::from(extra) {
        let Some(next) = skip_name(packet, offset) else {
            return found;
        };
        let (Some(kind), Some(length)) = (number(next), number(next + 8)) else {
            return found;
        };
        let data = next + 10;
        if kind == 33 {
            match number(data + 4) {
                Some(port) if !found.contains(&port) => found.push(port),
                _ => {}
            }
        }
        offset = data + usize::from(length);
    }
    found
}

/// Whether something on this TV is listening on `port`. Trying to take the port tells without
/// touching what's there, and a connection to a pairing port would use up one of its attempts.
fn listening(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err()
}

/// The port of the TV's wireless ADB. A TV can advertise an old port too, so only a port that's
/// listening counts.
pub async fn find(service: &str, wait: Duration) -> Option<u16> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await.ok()?;
    socket.set_multicast_ttl_v4(255).ok()?;
    let target = SocketAddr::from((Ipv4Addr::new(224, 0, 0, 251), 5353));
    let question = query(service);
    let deadline = Instant::now() + wait;
    let mut packet = vec![0; 9000];
    while Instant::now() < deadline {
        socket.send_to(&question, target).await.ok()?;
        let Ok(Ok(size)) = timeout(Duration::from_millis(700), socket.recv(&mut packet)).await
        else {
            continue;
        };
        if let Some(port) = ports(&packet[..size]).into_iter().find(|port| listening(*port)) {
            return Some(port);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An answer as a TV sends it, with the service name compressed.
    fn answer(port: u16) -> Vec<u8> {
        let mut packet = vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 1];
        packet.extend_from_slice(&query(CONNECT_SERVICE)[12..]);
        packet.truncate(packet.len() - 4);
        packet.extend_from_slice(&[0, 12, 0, 1, 0, 0, 0, 120, 0, 2, 0xC0, 12]);
        packet.extend_from_slice(&[0xC0, 12, 0, 33, 0x80, 1, 0, 0, 0, 120, 0, 8, 0, 0, 0, 0]);
        packet.extend_from_slice(&port.to_be_bytes());
        packet.extend_from_slice(&[0xC0, 12]);
        packet
    }

    #[test]
    fn reads_the_port_from_an_answer() {
        assert_eq!(ports(&answer(44121)), [44121]);
    }

    #[test]
    fn a_port_counts_only_while_something_listens_on_it() {
        let server = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = server.local_addr().unwrap().port();
        assert!(listening(port));
        drop(server);
        assert!(!listening(port));
    }

    #[test]
    fn a_query_is_a_ptr_question_for_the_service() {
        let packet = query("_a._tcp.local");
        assert_eq!(&packet[..12], [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&packet[12..15], [2, b'_', b'a']);
        assert!(packet.ends_with(&[0, 0, 12, 0x80, 1]));
    }

    #[test]
    fn a_broken_packet_finds_nothing_and_never_panics() {
        assert!(ports(&[]).is_empty());
        assert!(ports(&[0; 11]).is_empty());
        let good = answer(5555);
        for length in 0..good.len() {
            let _ = ports(&good[..length]);
        }
        let mut looped = good;
        looped[12] = 0xC0;
        looped[13] = 12;
        let _ = ports(&looped);
    }
}
