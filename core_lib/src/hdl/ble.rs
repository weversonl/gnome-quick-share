use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

use anyhow::anyhow;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::stream::StreamExt;
use tokio::sync::broadcast::Sender;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use uuid::{Uuid, uuid};

use super::{EndpointInfo, EndpointTransport};
use crate::utils::DeviceType;

const SERVICE_UUID_SHARING: Uuid = uuid!("0000fe2c-0000-1000-8000-00805f9b34fb");
const BLE_TIMEOUT_SECS: u64 = 30;

// Nearby Share BLE service data byte 0: bits [4:2] = device_type
fn parse_ble_device_type(data: &[u8]) -> DeviceType {
    if data.is_empty() {
        return DeviceType::Unknown;
    }
    DeviceType::from_raw_value((data[0] >> 2) & 0x7)
}

async fn make_adapter() -> Result<Adapter, anyhow::Error> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no bluetooth adapter"))
}

// BleListener: fires a wake signal when any nearby device is sharing via Quick Share
pub struct BleListener {
    adapter: Adapter,
    sender: Sender<()>,
}

impl BleListener {
    pub async fn new(sender: Sender<()>) -> Result<Self, anyhow::Error> {
        Ok(Self {
            adapter: make_adapter().await?,
            sender,
        })
    }

    pub async fn run(self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        info!("BleListener: service starting");

        let mut events = self.adapter.events().await?;
        // Empty filter: BlueZ's UUID filter drops advertisements that carry the
        // service only inside service_data (common for Android Quick Share).
        self.adapter.start_scan(ScanFilter::default()).await?;

        let mut last_alert = SystemTime::UNIX_EPOCH;

        loop {
            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("BleListener: tracker cancelled, breaking");
                    break;
                }
                Some(e) = events.next() => {
                    if let CentralEvent::ServiceDataAdvertisement { id, service_data } = e {
                        if !service_data.contains_key(&SERVICE_UUID_SHARING) {
                            continue;
                        }
                        let now = SystemTime::now();
                        if now.duration_since(last_alert).unwrap_or_default() <= Duration::from_secs(30) {
                            continue;
                        }
                        debug!("BleListener: device ({id:?}) is sharing nearby");
                        let _ = self.sender.send(());
                        last_alert = now;
                    }
                }
            }
        }

        Ok(())
    }
}

// BleDiscovery: discovers nearby Quick Share devices via BLE and emits EndpointInfo
pub struct BleDiscovery {
    adapter: Adapter,
    sender: Sender<EndpointInfo>,
}

impl BleDiscovery {
    pub async fn new(sender: Sender<EndpointInfo>) -> Result<Self, anyhow::Error> {
        Ok(Self {
            adapter: make_adapter().await?,
            sender,
        })
    }

    pub async fn run(self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        info!("BleDiscovery: service starting");

        let mut events = self.adapter.events().await?;
        // Empty filter: BlueZ's UUID filter drops advertisements that carry the
        // service only inside service_data (common for Android Quick Share).
        self.adapter.start_scan(ScanFilter::default()).await?;

        // Bootstrap: BlueZ caches peripherals from existing scans (e.g. BleListener).
        // New scan sessions don't re-emit events for already-cached devices, so we
        // enumerate them proactively on startup.
        let mut seen: HashMap<String, (EndpointInfo, Instant)> = HashMap::new();
        match self.adapter.peripherals().await {
            Ok(existing) => {
                info!(
                    "BleDiscovery: bootstrap found {} cached peripheral(s)",
                    existing.len()
                );
                for p in existing {
                    let addr = p.address().to_string();
                    match p.properties().await {
                        Ok(Some(props)) => {
                            info!(
                                "BleDiscovery: bootstrap peripheral {} name={:?} services={:?} service_data_keys={:?}",
                                addr,
                                props.local_name,
                                props.services,
                                props.service_data.keys().collect::<Vec<_>>(),
                            );
                            if props.services.contains(&SERVICE_UUID_SHARING)
                                || props.service_data.contains_key(&SERVICE_UUID_SHARING)
                            {
                                let ep_id = format!("ble:{addr}");
                                let device_type = props
                                    .service_data
                                    .get(&SERVICE_UUID_SHARING)
                                    .map(|d| parse_ble_device_type(d))
                                    .unwrap_or(DeviceType::Unknown);
                                let display_name = props
                                    .local_name
                                    .unwrap_or_else(|| format!("Bluetooth ({})", addr));
                                let ei = build_endpoint(ep_id.clone(), display_name, device_type);
                                info!("BleDiscovery: bootstrapped QuickShare device {:?}", ei);
                                seen.insert(ep_id, (ei.clone(), Instant::now()));
                                let _ = self.sender.send(ei);
                            }
                        }
                        Ok(None) => {
                            info!(
                                "BleDiscovery: bootstrap peripheral {} has no properties",
                                addr
                            );
                        }
                        Err(e) => {
                            info!(
                                "BleDiscovery: bootstrap peripheral {} properties error: {}",
                                addr, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                info!("BleDiscovery: bootstrap peripherals() failed: {}", e);
            }
        }

        let mut cleanup = interval(Duration::from_secs(10));
        cleanup.tick().await; // discard first immediate tick

        let mut event_count: u64 = 0;
        let mut match_count: u64 = 0;

        loop {
            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("BleDiscovery: cancelled (total events={}, matches={})", event_count, match_count);
                    break;
                }
                _ = cleanup.tick() => {
                    info!("BleDiscovery: heartbeat events={} matches={} known={}", event_count, match_count, seen.len());
                    let now = Instant::now();
                    let expired: Vec<String> = seen
                        .iter()
                        .filter_map(|(id, (_, last))| {
                            if now.duration_since(*last).as_secs() > BLE_TIMEOUT_SECS {
                                Some(id.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    for id in expired {
                        if let Some((ei, _)) = seen.remove(&id) {
                            info!("BleDiscovery: device {} timed out, removing", ei.id);
                            let _ = self.sender.send(EndpointInfo {
                                id: ei.id,
                                present: Some(false),
                                ..Default::default()
                            });
                        }
                    }
                }
                Some(e) = events.next() => {
                    event_count += 1;
                    match e {
                        // Fast path: service data explicitly carries the UUID
                        CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                            debug!(
                                "BleDiscovery: ServiceDataAdvertisement id={:?} uuids={:?}",
                                id,
                                service_data.keys().collect::<Vec<_>>(),
                            );
                            if !service_data.contains_key(&SERVICE_UUID_SHARING) {
                                continue;
                            }
                            match_count += 1;
                            let (ep_id, name) = self.get_peripheral_info(&id).await;
                            if let Some((_, last)) = seen.get_mut(&ep_id) {
                                *last = Instant::now();
                                continue;
                            }
                            let device_type = parse_ble_device_type(&service_data[&SERVICE_UUID_SHARING]);
                            let addr = ep_id.trim_start_matches("ble:").to_string();
                            let display_name = name.unwrap_or_else(|| format!("Bluetooth ({})", addr));
                            let ei = build_endpoint(ep_id.clone(), display_name, device_type);
                            info!("BleDiscovery: new device (service_data) {:?}", ei);
                            seen.insert(ep_id, (ei.clone(), Instant::now()));
                            let _ = self.sender.send(ei);
                        }
                        // Slow path: check properties for the service UUID.
                        // Needed for devices already cached in BlueZ before this scan started.
                        CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                            if let Ok(p) = self.adapter.peripheral(&id).await {
                                let addr = p.address().to_string();
                                let ep_id = format!("ble:{addr}");

                                // Already known — just refresh timestamp
                                if let Some((_, last)) = seen.get_mut(&ep_id) {
                                    *last = Instant::now();
                                    continue;
                                }

                                if let Ok(Some(props)) = p.properties().await {
                                    debug!(
                                        "BleDiscovery: DeviceDiscovered/Updated {} name={:?} services={:?} service_data_keys={:?}",
                                        addr,
                                        props.local_name,
                                        props.services,
                                        props.service_data.keys().collect::<Vec<_>>(),
                                    );
                                    if props.services.contains(&SERVICE_UUID_SHARING)
                                        || props.service_data.contains_key(&SERVICE_UUID_SHARING)
                                    {
                                        match_count += 1;
                                        let device_type = props
                                            .service_data
                                            .get(&SERVICE_UUID_SHARING)
                                            .map(|d| parse_ble_device_type(d))
                                            .unwrap_or(DeviceType::Unknown);
                                        let display_name = props
                                            .local_name
                                            .unwrap_or_else(|| format!("Bluetooth ({})", addr));
                                        let ei = build_endpoint(ep_id.clone(), display_name, device_type);
                                        info!("BleDiscovery: new device (properties) {:?}", ei);
                                        seen.insert(ep_id, (ei.clone(), Instant::now()));
                                        let _ = self.sender.send(ei);
                                    }
                                }
                            }
                        }
                        other => {
                            debug!("BleDiscovery: other event: {:?}", other);
                        }
                    }
                }
            }
        }

        // Mark all known devices as gone when discovery stops
        for (_, (ei, _)) in seen {
            let _ = self.sender.send(EndpointInfo {
                id: ei.id,
                present: Some(false),
                ..Default::default()
            });
        }

        Ok(())
    }

    async fn get_peripheral_info(
        &self,
        id: &btleplug::platform::PeripheralId,
    ) -> (String, Option<String>) {
        match self.adapter.peripheral(id).await {
            Ok(p) => {
                let addr = p.address().to_string();
                let name = match p.properties().await {
                    Ok(Some(props)) => props.local_name,
                    _ => None,
                };
                (format!("ble:{addr}"), name)
            }
            Err(_) => (format!("ble:{id:?}"), None),
        }
    }
}

fn build_endpoint(ep_id: String, display_name: String, device_type: DeviceType) -> EndpointInfo {
    EndpointInfo {
        fullname: ep_id.clone(),
        id: ep_id,
        name: Some(display_name),
        ip: None,
        port: None,
        rtype: Some(device_type),
        present: Some(true),
        transport: Some(EndpointTransport::BleDiscovery),
        wifi_direct_peer_path: None,
        wifi_direct_peer_mac: None,
    }
}
