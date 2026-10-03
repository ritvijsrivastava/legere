//! Cross-device sync over a user-supplied S3-compatible bucket (Cloudflare
//! R2, AWS S3, Backblaze B2, Minio, ...). See ARCHITECTURE.md's Sync
//! section for the full design: what syncs, the bucket layout, the
//! algorithm, and the conflict/tombstone rules this module's submodules
//! implement.
//!
//! This module is still scaffolding — [`client`] and [`manifest`] give
//! the primitives (a signed S3-compatible HTTP client with conditional
//! writes, and the manifest's data shape/(de)serialization), and
//! [`conflict`] gives the pure same-`conflict_key` merge logic. The sync
//! algorithm that drives them (the per-run pull/push/tombstone loop) is
//! not wired up yet — no Tauri command currently calls into this module.

pub mod bucket_gc;
pub mod client;
pub mod conflict;
pub mod credential_vault;
pub mod engine;
pub mod lazy_images;
pub mod link;
pub mod manifest;
pub mod orchestrate;
