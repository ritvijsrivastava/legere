//! Device-link QR handshake: lets a device that already has cross-device
//! sync configured hand that configuration to a second device via a QR
//! code instead of the manual bucket-credentials form. See
//! ARCHITECTURE.md's Cross-device sync section for the full design.
//!
//! What's deliberately *not* carried across: `device_id` (each device
//! must keep/generate its own — it identifies a device's own writes, not
//! the bucket) and `conditional_writes_verified` (the receiving device
//! re-runs `S3Client::probe_conditional_write_support` itself rather than
//! trusting the sender's last result, since that's a property of this
//! device's actual network path to the bucket, not of the bucket alone).
//! Both exclusions are enforced here, in [`LinkPayload`], not left to
//! every caller to remember.
//!
//! Format: a versioned envelope (`LinkEnvelope`) carrying a random salt,
//! a random 96-bit AES-GCM nonce, and the ciphertext, all as raw bytes --
//! encoded with `postcard` (compact, no field names) rather than JSON, to
//! keep the payload small enough for a scannable QR given how long S3
//! secret keys tend to be. The *plaintext* underneath is still JSON
//! (`LinkPayload`), since that only has to be compact enough to survive
//! one extra AES-GCM/Argon2id round-trip, not fit in a QR cell count
//! itself.
//!
//! Key derivation: Argon2id over the passphrase + per-code salt, tuned
//! down from the crate's defaults (`ARGON2_M_COST`/`ARGON2_T_COST`) --
//! this is a short-lived, display-once secret guessable in 10^6 tries at
//! most (see [`generate_passphrase`]'s 6 digits), not a long-term
//! password hash, so spending more than a few hundred milliseconds
//! deriving the key on a phone would only hurt the scan-to-decrypt UX
//! without meaningfully raising the attack cost.

use aes_gcm::aead::{Aead, KeyInit, generic_array::GenericArray};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::Argon2;
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::sync_config::RemoteSyncConfig;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const ENVELOPE_VERSION: u8 = 1;

// Tuned for a sub-second derivation on a mid-range phone while still
// costing a brute-forcer real time per guess; see module doc for why this
// doesn't need to match a long-term-password Argon2 profile.
const ARGON2_M_COST: u32 = 19_456; // KiB
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("failed to encode link payload: {0}")]
    Encode(#[from] postcard::Error),
    #[error("malformed or unsupported link code")]
    Malformed,
    #[error("incorrect passphrase or corrupted QR code")]
    DecryptFailed,
    #[error("failed to render link QR code: {0}")]
    QrEncode(#[from] qrcode::types::QrError),
    #[error("failed to render link QR code image: {0}")]
    ImageEncode(#[from] image::ImageError),
}

/// The subset of `RemoteSyncConfig` carried across devices. See module
/// doc for why `device_id` and `conditional_writes_verified` aren't
/// here.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkPayload {
    endpoint: String,
    bucket_name: String,
    region: String,
    use_path_style: bool,
    access_key: String,
    secret_key: String,
    sync_interval_hours: i64,
}

impl From<&RemoteSyncConfig> for LinkPayload {
    fn from(config: &RemoteSyncConfig) -> Self {
        LinkPayload {
            endpoint: config.endpoint.clone(),
            bucket_name: config.bucket_name.clone(),
            region: config.region.clone(),
            use_path_style: config.use_path_style,
            access_key: config.access_key.clone(),
            secret_key: config.secret_key.clone(),
            sync_interval_hours: config.sync_interval_hours,
        }
    }
}

/// What a successful [`decrypt`] produces: everything a receiving device
/// needs, with `enabled` defaulted to `true` (the whole point of scanning
/// a code is to turn sync on, mirroring what the manual setup form does
/// on first save) and the two device-local fields left for the caller to
/// fill in exactly as `db::sync_config::save_remote_sync_config` already
/// does for `device_id`, and as `commands::remote_sync::import_sync_qr`
/// is responsible for `conditional_writes_verified`.
impl LinkPayload {
    fn into_config(self) -> RemoteSyncConfig {
        RemoteSyncConfig {
            enabled: true,
            endpoint: self.endpoint,
            bucket_name: self.bucket_name,
            region: self.region,
            use_path_style: self.use_path_style,
            access_key: self.access_key,
            secret_key: self.secret_key,
            // Plaintext straight out of the QR payload, not yet through
            // `credential_vault` -- `save_remote_sync_config` is what
            // actually sets this (ignoring whatever's set here), the
            // first time the receiving device saves this config.
            credentials_encrypted: false,
            device_id: String::new(),
            conditional_writes_verified: false,
            sync_interval_hours: self.sync_interval_hours,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LinkEnvelope {
    version: u8,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
    ciphertext: Vec<u8>,
}

fn derive_key(passphrase: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; 32], LinkError> {
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(32))
            .map_err(|_| LinkError::Malformed)?,
    );
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|_| LinkError::Malformed)?;
    Ok(key)
}

/// A Signal-style six-digit numeric passphrase, generated fresh for every
/// QR code rather than typed by the user -- short-lived (the dialog
/// discards it on close/regenerate) and never reused, so memorability
/// doesn't matter, only that it's quick to read off one screen and type
/// on another.
pub fn generate_passphrase() -> String {
    let digits: u32 = OsRng.next_u32() % 1_000_000;
    format!("{digits:06}")
}

/// Encrypts `config` for the device-link QR code, returning the raw
/// envelope bytes to hand to a QR encoder.
pub fn encrypt(config: &RemoteSyncConfig, passphrase: &str) -> Result<Vec<u8>, LinkError> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = postcard::to_allocvec(&LinkPayload::from(config))?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| LinkError::Malformed)?;

    let envelope = LinkEnvelope {
        version: ENVELOPE_VERSION,
        salt,
        nonce: nonce_bytes,
        ciphertext,
    };
    Ok(postcard::to_allocvec(&envelope)?)
}

/// Decrypts a scanned QR code's raw bytes back into a `RemoteSyncConfig`.
/// Returns [`LinkError::DecryptFailed`] for both a wrong passphrase and a
/// corrupted/tampered payload -- AES-GCM's authentication tag makes the
/// two indistinguishable by design, and the caller (the scan UI) should
/// present them identically rather than imply a decode-level difference
/// that isn't actually knowable.
pub fn decrypt(envelope_bytes: &[u8], passphrase: &str) -> Result<RemoteSyncConfig, LinkError> {
    let envelope: LinkEnvelope =
        postcard::from_bytes(envelope_bytes).map_err(|_| LinkError::Malformed)?;
    if envelope.version != ENVELOPE_VERSION {
        return Err(LinkError::Malformed);
    }

    let key = derive_key(passphrase, &envelope.salt)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = Nonce::from_slice(&envelope.nonce);

    let plaintext = cipher
        .decrypt(nonce, envelope.ciphertext.as_ref())
        .map_err(|_| LinkError::DecryptFailed)?;
    let payload: LinkPayload =
        postcard::from_bytes(&plaintext).map_err(|_| LinkError::DecryptFailed)?;
    Ok(payload.into_config())
}

/// Renders `message` (the base64 text produced by Base64-encoding
/// [`encrypt`]'s output -- see `commands::remote_sync::generate_sync_qr`)
/// as a PNG QR code. Takes text rather than raw bytes so the scanning
/// side can treat the QR purely as a text code (what every JS QR-decode
/// library is built around) without any risk of a byte-mode decoder's
/// string conversion mangling non-ASCII bytes -- base64's output alphabet
/// is pure ASCII, so that round-trip is lossless.
pub fn render_qr_png(message: &str) -> Result<Vec<u8>, LinkError> {
    let code = qrcode::QrCode::with_error_correction_level(message.as_bytes(), qrcode::EcLevel::M)?;
    let image = code
        .render::<image::Luma<u8>>()
        .max_dimensions(640, 640)
        .build();

    let mut png_bytes = Vec::new();
    image::DynamicImage::ImageLuma8(image).write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;
    Ok(png_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> RemoteSyncConfig {
        RemoteSyncConfig {
            enabled: true,
            endpoint: "https://example.r2.cloudflarestorage.com".into(),
            bucket_name: "legere-sync".into(),
            region: "auto".into(),
            use_path_style: false,
            access_key: "AKIAEXAMPLE".into(),
            secret_key: "super-secret-value".into(),
            credentials_encrypted: false,
            device_id: "this-devices-own-id".into(),
            conditional_writes_verified: true,
            sync_interval_hours: 6,
        }
    }

    #[test]
    fn round_trips_the_syncable_fields() {
        let config = sample_config();
        let passphrase = generate_passphrase();
        let bytes = encrypt(&config, &passphrase).unwrap();
        let decoded = decrypt(&bytes, &passphrase).unwrap();

        assert_eq!(decoded.endpoint, config.endpoint);
        assert_eq!(decoded.bucket_name, config.bucket_name);
        assert_eq!(decoded.region, config.region);
        assert_eq!(decoded.use_path_style, config.use_path_style);
        assert_eq!(decoded.access_key, config.access_key);
        assert_eq!(decoded.secret_key, config.secret_key);
        assert_eq!(decoded.sync_interval_hours, config.sync_interval_hours);
        assert!(decoded.enabled);
    }

    #[test]
    fn never_carries_device_identity_or_verification_state() {
        let config = sample_config();
        let passphrase = generate_passphrase();
        let bytes = encrypt(&config, &passphrase).unwrap();
        let decoded = decrypt(&bytes, &passphrase).unwrap();

        assert_ne!(decoded.device_id, config.device_id);
        assert_eq!(decoded.device_id, "");
        assert!(!decoded.conditional_writes_verified);
    }

    #[test]
    fn rejects_wrong_passphrase() {
        let config = sample_config();
        let bytes = encrypt(&config, &generate_passphrase()).unwrap();
        let err = decrypt(&bytes, "000000").unwrap_err();
        assert!(matches!(err, LinkError::DecryptFailed));
    }

    #[test]
    fn rejects_tampered_ciphertext() {
        let config = sample_config();
        let passphrase = generate_passphrase();
        let mut bytes = encrypt(&config, &passphrase).unwrap();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let err = decrypt(&bytes, &passphrase).unwrap_err();
        assert!(matches!(err, LinkError::DecryptFailed));
    }

    #[test]
    fn rejects_garbage_bytes() {
        let err = decrypt(b"not a real envelope", "123456").unwrap_err();
        assert!(matches!(err, LinkError::Malformed));
    }

    #[test]
    fn passphrase_is_six_digits() {
        for _ in 0..50 {
            let p = generate_passphrase();
            assert_eq!(p.len(), 6);
            assert!(p.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn renders_a_decodable_qr_png() {
        let config = sample_config();
        let passphrase = generate_passphrase();
        let envelope = encrypt(&config, &passphrase).unwrap();
        let text = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &envelope);

        let png = render_qr_png(&text).unwrap();
        // A real PNG signature, not an empty/garbage buffer.
        assert_eq!(&png[..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
