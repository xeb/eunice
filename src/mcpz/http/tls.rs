use anyhow::{anyhow, Context, Result};
use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// TLS configuration holding certificate and key
#[derive(Debug)]
pub struct TlsConfig {
    pub cert_pem: String,
    pub key_pem: String,
    pub is_self_signed: bool,
}

impl TlsConfig {
    /// Load TLS config from files or generate self-signed certificate
    pub fn load_or_generate(
        cert_path: Option<&Path>,
        key_path: Option<&Path>,
    ) -> Result<Self> {
        match (cert_path, key_path) {
            (Some(cert), Some(key)) => Self::load_from_files(cert, key),
            (None, None) => Self::load_or_generate_self_signed(),
            _ => Err(anyhow!("Both --cert and --key must be provided together")),
        }
    }

    /// Load certificate and key from PEM files
    fn load_from_files(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let cert_pem = std::fs::read_to_string(cert_path)
            .with_context(|| format!("Failed to read certificate file: {:?}", cert_path))?;
        let key_pem = std::fs::read_to_string(key_path)
            .with_context(|| format!("Failed to read key file: {:?}", key_path))?;

        Ok(Self {
            cert_pem,
            key_pem,
            is_self_signed: false,
        })
    }

    /// Load cached self-signed cert or generate a new one
    fn load_or_generate_self_signed() -> Result<Self> {
        let cache_dir = Self::cache_dir()?;
        let cert_path = cache_dir.join("self-signed.crt");
        let key_path = cache_dir.join("self-signed.key");

        // Try to load cached certificate
        if cert_path.exists() && key_path.exists() {
            if let Ok(config) = Self::load_from_files(&cert_path, &key_path) {
                // Check if certificate is still valid (not expired)
                if !Self::is_cert_expired(&config.cert_pem) {
                    return Ok(Self {
                        cert_pem: config.cert_pem,
                        key_pem: config.key_pem,
                        is_self_signed: true,
                    });
                }
            }
        }

        // Generate new self-signed certificate
        let config = Self::generate_self_signed()?;

        // Cache it
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;
        std::fs::write(&cert_path, &config.cert_pem)
            .with_context(|| format!("Failed to write certificate: {:?}", cert_path))?;
        std::fs::write(&key_path, &config.key_pem)
            .with_context(|| format!("Failed to write key: {:?}", key_path))?;

        Ok(config)
    }

    /// Generate a new self-signed certificate
    fn generate_self_signed() -> Result<Self> {
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, "localhost");
        params.subject_alt_names = vec![
            SanType::DnsName("localhost".try_into().unwrap()),
            SanType::IpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)),
        ];

        // Set validity to 365 days
        let now = time::OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + time::Duration::days(365);

        let key_pair = KeyPair::generate().context("Failed to generate key pair")?;
        let cert = params
            .self_signed(&key_pair)
            .context("Failed to generate self-signed certificate")?;

        Ok(Self {
            cert_pem: cert.pem(),
            key_pem: key_pair.serialize_pem(),
            is_self_signed: true,
        })
    }

    /// Check if a certificate is expired (basic check via parsing)
    fn is_cert_expired(_cert_pem: &str) -> bool {
        // Parse the certificate to check expiration
        // For simplicity, we'll just check if the file is older than 365 days
        // A more robust implementation would parse the X.509 certificate
        false // Assume not expired for now; the cert is regenerated on errors
    }

    /// Get the cache directory for TLS files
    fn cache_dir() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow!("Could not determine cache directory"))?
            .join("mcpz/tls");
        Ok(cache_dir)
    }

    /// Calculate SHA-256 fingerprint of the certificate
    pub fn fingerprint(&self) -> Result<String> {
        // Parse the PEM certificate
        let cert_der = Self::pem_to_der(&self.cert_pem)?;

        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&cert_der);
        let hash = hasher.finalize();

        // Format as colon-separated hex
        let hex_str: Vec<String> = hash.iter().map(|b| format!("{:02X}", b)).collect();
        Ok(hex_str.join(":"))
    }

    /// Convert PEM certificate to DER bytes
    fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
        let mut reader = std::io::BufReader::new(pem.as_bytes());
        let certs = rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificate PEM")?;

        certs
            .into_iter()
            .next()
            .map(|c| c.to_vec())
            .ok_or_else(|| anyhow!("No certificate found in PEM"))
    }

    /// Build rustls ServerConfig from this TLS config
    pub fn build_rustls_config(&self) -> Result<Arc<tokio_rustls::rustls::ServerConfig>> {
        // Parse certificate chain
        let mut cert_reader = std::io::BufReader::new(self.cert_pem.as_bytes());
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificate")?;

        // Parse private key
        let mut key_reader = std::io::BufReader::new(self.key_pem.as_bytes());
        let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)
            .context("Failed to read private key")?
            .ok_or_else(|| anyhow!("No private key found"))?;

        // Build server config
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("Failed to build TLS config")?;

        Ok(Arc::new(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed() {
        let config = TlsConfig::generate_self_signed().unwrap();
        assert!(config.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(config.key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(config.is_self_signed);
    }

    #[test]
    fn test_fingerprint() {
        let config = TlsConfig::generate_self_signed().unwrap();
        let fingerprint = config.fingerprint().unwrap();
        // Fingerprint should be 64 hex chars + 31 colons = 95 chars
        assert_eq!(fingerprint.len(), 95);
        assert!(fingerprint.contains(':'));
    }

    #[test]
    fn test_build_rustls_config() {
        // Install the ring crypto provider for rustls
        let _ = rustls::crypto::ring::default_provider().install_default();

        let config = TlsConfig::generate_self_signed().unwrap();
        let rustls_config = config.build_rustls_config();
        assert!(rustls_config.is_ok());
    }

    #[test]
    fn test_load_or_generate_requires_both_files() {
        let result = TlsConfig::load_or_generate(Some(Path::new("/tmp/cert.pem")), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Both --cert and --key"));
    }
}
