//! ADB DNS-SD only: discovery never authenticates, pairs or connects to an ADB socket.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::IpAddr};

pub const SERVICE_TYPES: [&str; 3] = [
    "_adb-tls-connect._tcp.local.",
    "_adb-tls-pairing._tcp.local.",
    "_adb._tcp.local.",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceKind {
    Connect,
    Pairing,
    Legacy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredService {
    pub name: String,
    pub kind: ServiceKind,
    pub host: String,
    pub endpoint: String,
}
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub services: Vec<DiscoveredService>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawService {
    pub name: String,
    pub service_type: String,
    pub host: String,
    pub port: u16,
}
#[derive(Default, Deserialize)]
pub struct RawDiscovery {
    pub services: Vec<RawService>,
    pub warnings: Vec<String>,
}

pub fn normalize(raw: RawDiscovery) -> DiscoveryResult {
    let mut services = Vec::new();
    let mut seen = HashSet::new();
    for row in raw.services.into_iter().take(256) {
        let ty = row.service_type.to_ascii_lowercase();
        let ty = ty.trim_end_matches('.').trim_end_matches(".local");
        let kind = match ty {
            "_adb-tls-connect._tcp" => ServiceKind::Connect,
            "_adb-tls-pairing._tcp" => ServiceKind::Pairing,
            "_adb._tcp" => ServiceKind::Legacy,
            _ => continue,
        };
        if row.port == 0 || row.host.len() > 255 {
            continue;
        }
        let (ip, scope) = row
            .host
            .split_once('%')
            .map_or((row.host.as_str(), None), |(ip, scope)| (ip, Some(scope)));
        let Ok(ip) = ip.parse::<IpAddr>() else {
            continue;
        };
        if ip.is_unspecified() || ip.is_multicast() || ip.is_loopback() {
            continue;
        }
        if let Some(scope) = scope {
            if !ip.is_ipv6()
                || scope.is_empty()
                || !scope
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
            {
                continue;
            }
        }
        let host = format!("{ip}{}", scope.map_or(String::new(), |s| format!("%{s}")));
        let endpoint = if ip.is_ipv6() {
            format!("[{host}]:{}", row.port)
        } else {
            format!("{host}:{}", row.port)
        };
        if !seen.insert((kind, endpoint.clone())) {
            continue;
        }
        let mut name: String = row
            .name
            .chars()
            .filter(|c| !c.is_control())
            .take(160)
            .collect();
        for service_type in SERVICE_TYPES {
            if let Some(instance) = name.strip_suffix(&format!(".{service_type}")) {
                name = instance.to_owned();
                break;
            }
        }
        if name.is_empty() {
            name = host.clone();
        }
        services.push(DiscoveredService {
            name,
            kind,
            host,
            endpoint,
        });
        if services.len() == 64 {
            break;
        }
    }
    services.sort_by(|a, b| a.name.cmp(&b.name).then(a.endpoint.cmp(&b.endpoint)));
    DiscoveryResult {
        services,
        warnings: raw
            .warnings
            .into_iter()
            .take(3)
            .map(|w| w.chars().take(200).collect())
            .collect(),
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn discover(
    duration: std::time::Duration,
) -> Result<DiscoveryResult, crate::backend::PocError> {
    use mdns_sd::{ServiceDaemon, ServiceEvent};
    use std::time::Instant;
    let error = |e| {
        crate::backend::PocError::new(
            "discovery",
            format!("Could not search for ADB devices: {e}"),
        )
    };
    let daemon = ServiceDaemon::new().map_err(error)?;
    // Always stop the daemon, including when starting one of the browsers fails.
    let result = (|| {
        let monitor = daemon.monitor().map_err(error)?;
        let receivers = SERVICE_TYPES
            .iter()
            .map(|ty| daemon.browse(ty).map(|rx| (*ty, rx)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?;
        let end = Instant::now() + duration.min(std::time::Duration::from_secs(8));
        let mut raw = RawDiscovery::default();
        while Instant::now() < end {
            for (ty, receiver) in &receivers {
                for event in receiver.try_iter().take(256) {
                    match event {
                        ServiceEvent::ServiceResolved(info) => {
                            raw.services.retain(|row| row.name != info.fullname);
                            let mut addresses: Vec<_> = info.addresses.iter().collect();
                            addresses.sort_by_key(|address| !address.is_ipv4());
                            for address in addresses.into_iter().take(4) {
                                let host = address.to_string();
                                if raw.services.len() < 256 {
                                    raw.services.push(RawService {
                                        name: info.fullname.clone(),
                                        service_type: (*ty).to_owned(),
                                        host,
                                        port: info.port,
                                    });
                                }
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, name) => {
                            raw.services.retain(|r| r.name != name);
                        }
                        _ => {}
                    }
                }
            }
            for event in monitor.try_iter().take(64) {
                if let mdns_sd::DaemonEvent::Error(_) = event {
                    raw.warnings = vec![
                        "Discovery encountered a network error. Some devices may be missing."
                            .into(),
                    ];
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Ok(normalize(raw))
    })();
    if let Ok(stopped) = daemon.shutdown() {
        let _ = stopped.recv_timeout(std::time::Duration::from_secs(1));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(ty: &str, host: &str, port: u16) -> RawService {
        RawService {
            name: "TV".into(),
            service_type: ty.into(),
            host: host.into(),
            port,
        }
    }
    #[test]
    fn keeps_pairing_and_connection_ports_distinct_and_deduplicates() {
        let result = normalize(RawDiscovery {
            services: vec![
                row(SERVICE_TYPES[0], "192.0.2.12", 40000),
                row(SERVICE_TYPES[1], "192.0.2.12", 35000),
                row(SERVICE_TYPES[0], "192.0.2.12", 40000),
            ],
            warnings: vec![],
        });
        assert_eq!(result.services.len(), 2);
        assert!(
            result
                .services
                .iter()
                .any(|r| r.kind == ServiceKind::Pairing && r.endpoint.ends_with(":35000"))
        );
    }
    #[test]
    fn rejects_unrelated_services_and_unusable_endpoints() {
        let result = normalize(RawDiscovery {
            services: vec![
                row("_http._tcp", "192.0.2.12", 80),
                row(SERVICE_TYPES[0], "0.0.0.0", 1),
                row(SERVICE_TYPES[0], "192.0.2.12", 0),
                row(SERVICE_TYPES[0], "bad host", 1),
            ],
            warnings: vec![],
        });
        assert!(result.services.is_empty());
    }
    #[test]
    fn preserves_ipv6_scope_and_brackets_endpoint() {
        let result = normalize(RawDiscovery {
            services: vec![row("_adb._tcp.", "fe80::1%en0", 5555)],
            warnings: vec![],
        });
        assert_eq!(result.services[0].endpoint, "[fe80::1%en0]:5555");
        assert_eq!(result.services[0].kind, ServiceKind::Legacy);
    }
}
