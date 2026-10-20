use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use tracing::info;

pub const AIRDROP_SERVICE_TYPE: &str = "_airdrop._tcp.local.";

/// Holds resolved details of a discovered AirDrop receiver.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub instance_name: String,
    pub host_name: String,
    pub port: u16,
    pub ip_addresses: Vec<IpAddr>,
    pub txt_records: HashMap<String, String>,
}

/// Registers the local machine/device as an AirDrop receiver over mDNS.
pub fn register_airdrop_service(
    daemon: &ServiceDaemon,
    instance_name: &str,
    host_name: &str,
    port: u16,
    is_everyone: bool,
) -> Result<(), mdns_sd::Error> {
    let mut properties = HashMap::new();
    properties.insert("flags".to_string(), if is_everyone { "1" } else { "3" }.to_string());
    properties.insert("airdrop".to_string(), "1.0".to_string());
    properties.insert("phash".to_string(), "0000000000000000000000000000000000000000000000000000000000000000".to_string());

    let service_type = AIRDROP_SERVICE_TYPE;
    let my_service = ServiceInfo::new(
        service_type,
        instance_name,
        host_name,
        "", // Auto resolve IP
        port,
        properties,
    )?;

    daemon.register(my_service)?;
    info!("Registered mDNS AirDrop service: {}.{}", instance_name, service_type);
    Ok(())
}

/// Discovers nearby AirDrop receivers on the local network within a timeout window.
pub async fn discover_airdrop_peers(
    daemon: &ServiceDaemon,
    timeout: Duration,
) -> Result<Vec<DiscoveredPeer>, mdns_sd::Error> {
    let receiver = daemon.browse(AIRDROP_SERVICE_TYPE)?;
    let mut peers: HashMap<String, DiscoveredPeer> = HashMap::new();

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Ok(event) = receiver.recv_timeout(Duration::from_millis(150)) {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let instance = info.get_fullname().to_string();
                    let ips: Vec<IpAddr> = info.get_addresses().iter().cloned().collect();
                    let host = info.get_hostname().to_string();
                    let port = info.get_port();

                    let mut txt = HashMap::new();
                    for prop in info.get_properties().iter() {
                        txt.insert(prop.key().to_string(), prop.val_str().to_string());
                    }

                    info!("Found AirDrop peer: {} at {:?}:{}", instance, ips, port);
                    peers.insert(
                        instance.clone(),
                        DiscoveredPeer {
                            instance_name: instance,
                            host_name: host,
                            port,
                            ip_addresses: ips,
                            txt_records: txt,
                        },
                    );
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    peers.remove(&fullname);
                }
                _ => {}
            }
        }
    }

    Ok(peers.into_values().collect())
}
