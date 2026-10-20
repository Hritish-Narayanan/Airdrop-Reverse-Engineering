pub mod ble;
pub mod client;
pub mod contacts;
pub mod cpio;
pub mod mdns;
pub mod protocol;
pub mod server;
pub mod tls;

pub use ble::AirDropBlePayload;
pub use client::AirDropClient;
pub use contacts::{compute_contact_hash, compute_truncated_hash, normalize_email, normalize_phone};
pub use cpio::{build_cpio_archive, extract_cpio_to_directory, parse_cpio_archive, CpioEntry};
pub use mdns::{discover_airdrop_peers, register_airdrop_service, DiscoveredPeer};
pub use protocol::{AskFile, AskRequest, AskResponse, DiscoverRequest, DiscoverResponse, UploadResponse};
pub use server::{run_airdrop_server, AirDropServerConfig};
pub use tls::{ensure_crypto_provider, generate_airdrop_certificate, AirDropCertificate};
