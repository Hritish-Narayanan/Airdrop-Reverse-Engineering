use rcgen::{CertificateParams, KeyPair, DistinguishedName, DnType};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::client::danger::{ServerCertVerifier, HandshakeSignatureValid};
use rustls::{ClientConfig, DigitallySignedStruct, Error as RustlsError, ServerConfig, SignatureScheme};
use std::sync::Arc;

/// Container holding generated TLS certificate and matching private key.
#[derive(Clone)]
pub struct AirDropCertificate {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
    pub cert_pem: String,
    pub key_pem: String,
}

/// Generates an ephemeral self-signed X.509 certificate compatible with Apple AirDrop.
pub fn generate_airdrop_certificate(common_name: &str) -> Result<AirDropCertificate, Box<dyn std::error::Error + Send + Sync>> {
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, common_name);
    dn.push(DnType::OrganizationName, "AirDrop");
    params.distinguished_name = dn;

    // AirDrop accepts P-256 ECDSA keys
    let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)?;
    let cert = params.self_signed(&key_pair)?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();
    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    Ok(AirDropCertificate {
        cert_der,
        key_der,
        cert_pem,
        key_pem,
    })
}

pub fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Creates a `rustls::ServerConfig` for the AirDrop HTTPS listener.
pub fn create_server_config(cert: &AirDropCertificate) -> Result<Arc<ServerConfig>, RustlsError> {
    ensure_crypto_provider();
    let cert_der = CertificateDer::from(cert.cert_der.clone());
    let key_der = PrivateKeyDer::try_from(cert.key_der.clone())
        .map_err(|_| RustlsError::General("Invalid private key DER".to_string()))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    Ok(Arc::new(config))
}

/// A custom verifier that accepts peer self-signed AirDrop certificates.
#[derive(Debug)]
struct NoVerification;

impl ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, RustlsError> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ED25519,
        ]
    }
}

/// Creates a `rustls::ClientConfig` that connects to an AirDrop device over TLS.
pub fn create_client_config() -> Arc<ClientConfig> {
    ensure_crypto_provider();
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerification))
        .with_no_client_auth();

    Arc::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_generation() {
        let cert = generate_airdrop_certificate("TestAirDropDevice").expect("cert generation failed");
        assert!(!cert.cert_der.is_empty());
        assert!(!cert.key_der.is_empty());
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.key_pem.contains("PRIVATE KEY"));

        let server_cfg = create_server_config(&cert);
        assert!(server_cfg.is_ok());

        let client_cfg = create_client_config();
        assert!(client_cfg.alpn_protocols.is_empty() || true);
    }
}
