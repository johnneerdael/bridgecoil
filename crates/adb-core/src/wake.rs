//! Wake-on-LAN: a broadcast that a sleeping TV's network card can be set to listen for.
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

const PORT: u16 = 9;

pub fn parse_address(value: &str) -> Option<[u8; 6]> {
    let mut bytes = [0u8; 6];
    let mut parts = value.trim().split(':');
    for byte in &mut bytes {
        let part = parts.next()?;
        if part.len() != 2 {
            return None;
        }
        *byte = u8::from_str_radix(part, 16).ok()?;
    }
    parts.next().is_none().then_some(bytes)
}

/// A real card's address. The low bit of the first byte marks group addresses.
fn usable(bytes: &[u8; 6]) -> bool {
    bytes != &[0; 6] && bytes[0] & 1 == 0
}

fn wired_or_wireless(name: &str) -> bool {
    ["eth", "wlan"].iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
    })
}

/// Reads `ip -o link`, and the `eth0 aa:bb:..` lines that stand in where it's refused.
pub fn parse_hardware_addresses(lines: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for line in lines {
        let words: Vec<_> = line.split_whitespace().collect();
        let (name, address) = match words.as_slice() {
            [name, address] => (*name, Some(*address)),
            [_, name, ..] if name.ends_with(':') => (
                name.trim_end_matches(':'),
                words
                    .iter()
                    .position(|w| *w == "link/ether")
                    .and_then(|i| words.get(i + 1).copied()),
            ),
            _ => continue,
        };
        let name = name.split('@').next().unwrap_or(name);
        let Some(address) = address.filter(|_| wired_or_wireless(name)) else {
            continue;
        };
        let address = address.to_ascii_lowercase();
        if parse_address(&address).is_some_and(|b| usable(&b)) && !found.contains(&address) {
            found.push(address);
        }
    }
    found.truncate(4);
    found
}

pub fn magic_packet(address: [u8; 6]) -> [u8; 102] {
    let mut packet = [0xff; 102];
    for chunk in packet[6..].chunks_exact_mut(6) {
        chunk.copy_from_slice(&address);
    }
    packet
}

/// Everywhere the packet goes. A TV asleep has no way to answer for its IP address, so broadcast
/// does the work. The network's size isn't known here, which is why several are tried: a /23 has
/// its broadcast outside the guessed /24. Sending to the TV itself helps while its address is
/// still cached, and takes the right network when the phone also has mobile data.
pub fn targets(host: &str) -> Vec<SocketAddr> {
    let mut all = vec![IpAddr::V4(Ipv4Addr::BROADCAST)];
    if let Ok(ip) = host.parse::<IpAddr>() {
        all.push(ip);
        if let IpAddr::V4(v4) = ip {
            let [a, b, c, _] = v4.octets();
            all.push(IpAddr::V4(Ipv4Addr::new(a, b, c, 255)));
            all.push(IpAddr::V4(Ipv4Addr::new(a, b, c | 1, 255)));
        }
    }
    all.dedup();
    all.into_iter().map(|ip| SocketAddr::new(ip, PORT)).collect()
}

/// Sends to every target and reports how many packets left this device. Reaching one is enough,
/// since a network commonly refuses some of these.
pub fn send(addresses: &[String], to: &[SocketAddr]) -> usize {
    let bind = |ip: IpAddr| UdpSocket::bind(SocketAddr::new(ip, 0)).ok();
    let v4 = bind(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let v6 = bind(IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED));
    if let Some(socket) = &v4 {
        let _ = socket.set_broadcast(true);
    }
    let mut sent = 0;
    for packet in addresses.iter().filter_map(|a| parse_address(a)) {
        let packet = magic_packet(packet);
        for target in to {
            let socket = if target.is_ipv4() { &v4 } else { &v6 };
            if socket.as_ref().is_some_and(|s| s.send_to(&packet, target).is_ok()) {
                sent += 1;
            }
        }
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_real_network_cards() {
        let lines = [
            "eth0 ",
            "wlan0 44:0f:b4:d3:f1:4d",
            "1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN \\    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00",
            "2: dummy0: <BROADCAST,NOARP,UP,LOWER_UP> mtu 1500 qdisc noqueue state UNKNOWN \\    link/ether 46:aa:bb:cc:dd:ee brd ff:ff:ff:ff:ff:ff",
            "3: ifb0: <BROADCAST,NOARP> mtu 1500 qdisc noop state DOWN \\    link/ether 4a:aa:bb:cc:dd:ee brd ff:ff:ff:ff:ff:ff",
            "9: eth0: <BROADCAST,MULTICAST> mtu 1500 qdisc mq state DOWN \\    link/ether 90:0E:B3:FD:FE:D0 brd ff:ff:ff:ff:ff:ff",
            "10: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc mq state UP \\    link/ether 44:0f:b4:d3:f1:4d brd ff:ff:ff:ff:ff:ff",
            "11: p2p0: <BROADCAST,MULTICAST> mtu 1500 \\    link/ether 46:0f:b4:d3:f1:4d brd ff:ff:ff:ff:ff:ff",
            "12: wlan0@if3: <UP> mtu 1500 \\    link/ether 00:00:00:00:00:00 brd ff:ff:ff:ff:ff:ff",
        ];
        assert_eq!(
            parse_hardware_addresses(&lines),
            ["44:0f:b4:d3:f1:4d", "90:0e:b3:fd:fe:d0"]
        );
    }

    #[test]
    fn ignores_output_that_is_not_an_address() {
        assert!(parse_hardware_addresses(&["", "eth0 cat: Permission denied", "wlan0 nope"]).is_empty());
        assert_eq!(parse_address("90:0e:b3:fd:fe:d0"), Some([0x90, 0x0e, 0xb3, 0xfd, 0xfe, 0xd0]));
        for bad in ["", "90:0e:b3:fd:fe", "90:0e:b3:fd:fe:d0:11", "900:e:b3:fd:fe:d0", "zz:0e:b3:fd:fe:d0"] {
            assert_eq!(parse_address(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_magic_packet_is_six_ff_then_the_address_sixteen_times() {
        let address = [0x90, 0x0e, 0xb3, 0xfd, 0xfe, 0xd0];
        let packet = magic_packet(address);
        assert_eq!(packet[..6], [0xff; 6]);
        assert!(packet[6..].chunks_exact(6).all(|c| c == address));
        assert_eq!(packet[6..].chunks_exact(6).count(), 16);
    }

    #[test]
    fn covers_a_network_larger_than_a_slash_24() {
        let ips: Vec<_> = targets("192.168.50.12").iter().map(|t| t.ip().to_string()).collect();
        assert_eq!(ips, ["255.255.255.255", "192.168.50.12", "192.168.50.255", "192.168.51.255"]);
        assert!(targets("192.168.50.12").iter().all(|t| t.port() == 9));
        assert_eq!(targets("tv.local").len(), 1);
        assert_eq!(targets("fe80::1").len(), 2);
    }

    #[test]
    fn the_packet_arrives() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        listener.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
        let to = [listener.local_addr().unwrap()];
        let sent = send(&["90:0e:b3:fd:fe:d0".into(), "not an address".into()], &to);
        assert_eq!(sent, 1);
        let mut buffer = [0; 200];
        let (n, _) = listener.recv_from(&mut buffer).unwrap();
        assert_eq!(buffer[..n], magic_packet([0x90, 0x0e, 0xb3, 0xfd, 0xfe, 0xd0]));
    }
}
