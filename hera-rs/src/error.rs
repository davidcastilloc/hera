use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Typed Domain & Control Plane Errors (`rust-skills` / `rust-patterns` / `hexagonal-architecture`).
/// Implements `std::error::Error`, `Display`, and `axum::response::IntoResponse` so all HTTP
/// handlers in `router.rs` can return `Result<T, HeraError>` using `?` without ad-hoc tuple plumbing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeraError {
    /// Missing or invalid authentication token (`401 Unauthorized`).
    Unauthorized(String),
    /// Swarm-First Policy / Quarantine / Poisoned Custody rejection (`403 Forbidden`).
    Forbidden(String),
    /// Requested track, DJ profile, or shard not found (`404 Not Found`).
    NotFound(String),
    /// Invalid client input or malformed request (`400 Bad Request`).
    BadRequest(String),
    /// Audio decoding or spectral analysis failure (`422 Unprocessable Entity`).
    UnprocessableEntity(String),
    /// R2 storage quota exceeded (`402 Payment Required` / `403 Forbidden`).
    QuotaExceeded(String),
    /// Poisoned lock or internal engine error (`500 Internal Server Error`).
    Internal(String),
}

impl fmt::Display for HeraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized(msg) => write!(f, "UNAUTHORIZED: {msg}"),
            Self::Forbidden(msg) => write!(f, "FORBIDDEN: {msg}"),
            Self::NotFound(msg) => write!(f, "NOT_FOUND: {msg}"),
            Self::BadRequest(msg) => write!(f, "BAD_REQUEST: {msg}"),
            Self::UnprocessableEntity(msg) => write!(f, "UNPROCESSABLE_ENTITY: {msg}"),
            Self::QuotaExceeded(msg) => write!(f, "R2_QUOTA_EXCEEDED: {msg}"),
            Self::Internal(msg) => write!(f, "INTERNAL_ERROR: {msg}"),
        }
    }
}

impl std::error::Error for HeraError {}

impl HeraError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) | Self::QuotaExceeded(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code_slug(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::NotFound(_) => "not_found",
            Self::BadRequest(_) => "bad_request",
            Self::UnprocessableEntity(_) => "unprocessable_entity",
            Self::QuotaExceeded(_) => "r2_quota_exceeded",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for HeraError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code_slug();
        let message = match &self {
            Self::Unauthorized(m)
            | Self::Forbidden(m)
            | Self::NotFound(m)
            | Self::BadRequest(m)
            | Self::UnprocessableEntity(m)
            | Self::QuotaExceeded(m)
            | Self::Internal(m) => m.clone(),
        };
        (
            status,
            Json(serde_json::json!({
                "error": message,
                "code": code
            })),
        )
            .into_response()
    }
}

/// Constant-time byte comparison to prevent timing side-channel attacks on HMAC tickets and Merkle proofs.
#[inline]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Strongly-typed Subscription Tier (`type-newtype-ids` / `anti-stringly-typed`).
/// Preserves 100% serde wire compatibility while enforcing Swarm-First invariants at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionTier {
    #[serde(rename = "trial_100mb")]
    Trial100Mb,
    #[serde(rename = "beta_mvp")]
    BetaMvp,
    #[serde(rename = "free")]
    Free,
    #[serde(rename = "local_unlimited")]
    LocalUnlimited,
    #[serde(rename = "beta_vip_10gb")]
    BetaVip10Gb,
    #[serde(rename = "pro_monthly")]
    ProMonthly,
    #[serde(rename = "pro")]
    Pro,
    #[serde(rename = "founder_unlimited")]
    FounderUnlimited,
}

impl SubscriptionTier {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim() {
            "trial_100mb" => Self::Trial100Mb,
            "free" => Self::Free,
            "local_unlimited" => Self::LocalUnlimited,
            "beta_vip_10gb" => Self::BetaVip10Gb,
            "pro_monthly" => Self::ProMonthly,
            "pro" => Self::Pro,
            "founder_unlimited" => Self::FounderUnlimited,
            _ => Self::BetaMvp,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trial100Mb => "trial_100mb",
            Self::BetaMvp => "beta_mvp",
            Self::Free => "free",
            Self::LocalUnlimited => "local_unlimited",
            Self::BetaVip10Gb => "beta_vip_10gb",
            Self::ProMonthly => "pro_monthly",
            Self::Pro => "pro",
            Self::FounderUnlimited => "founder_unlimited",
        }
    }

    pub const fn r2_quota_mb(&self) -> f32 {
        match self {
            Self::Trial100Mb => 100.0,
            Self::BetaMvp | Self::Free | Self::LocalUnlimited => 5_120.0,
            Self::BetaVip10Gb => 10_240.0,
            Self::ProMonthly | Self::Pro => 51_200.0,
            Self::FounderUnlimited => 1_000_000.0,
        }
    }

    /// Swarm-First Invariant ("Por el bien del enjambre"):
    /// Only `ProMonthly`, `Pro`, and `FounderUnlimited` can disable P2P sharing.
    pub const fn allows_private_vault(&self) -> bool {
        matches!(self, Self::ProMonthly | Self::Pro | Self::FounderUnlimited)
    }
}

/// Strongly-typed Storage Provider (`anti-stringly-typed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageProvider {
    #[serde(rename = "gdrive")]
    GDrive,
    #[serde(rename = "r2")]
    R2,
    #[serde(rename = "gdrive+r2")]
    GDrivePlusR2,
    #[serde(rename = "dropbox")]
    Dropbox,
    #[serde(rename = "onedrive")]
    OneDrive,
    #[serde(rename = "s3")]
    S3,
    #[serde(rename = "local_disk")]
    LocalDisk,
}

impl StorageProvider {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim() {
            "r2" | "cloudflare_r2" => Self::R2,
            "gdrive+r2" => Self::GDrivePlusR2,
            "dropbox" => Self::Dropbox,
            "onedrive" => Self::OneDrive,
            "s3" => Self::S3,
            "local_disk" | "local" => Self::LocalDisk,
            _ => Self::GDrive,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::GDrive => "gdrive",
            Self::R2 => "r2",
            Self::GDrivePlusR2 => "gdrive+r2",
            Self::Dropbox => "dropbox",
            Self::OneDrive => "onedrive",
            Self::S3 => "s3",
            Self::LocalDisk => "local_disk",
        }
    }
}

/// Strongly-typed Zero-Trust Spectral Forensic Verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForensicVerdict {
    #[serde(rename = "VERIFIED_STUDIO_MASTER")]
    VerifiedStudioMaster,
    #[serde(rename = "CLEAN_LOSSY_CLUB_READY")]
    CleanLossyClubReady,
    #[serde(rename = "REJECT_SYNTHETIC_TONE")]
    RejectSyntheticTone,
    #[serde(rename = "REJECT_FAKE_TRANSCODE")]
    RejectFakeTranscode,
    #[serde(rename = "REJECT_SUB_PHASE_CANCELLATION")]
    RejectSubPhaseCancellation,
    #[serde(rename = "QUARANTINE_LOW_QUALITY")]
    QuarantineLowQuality,
}

impl ForensicVerdict {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::VerifiedStudioMaster => "VERIFIED_STUDIO_MASTER",
            Self::CleanLossyClubReady => "CLEAN_LOSSY_CLUB_READY",
            Self::RejectSyntheticTone => "REJECT_SYNTHETIC_TONE",
            Self::RejectFakeTranscode => "REJECT_FAKE_TRANSCODE",
            Self::RejectSubPhaseCancellation => "REJECT_SUB_PHASE_CANCELLATION",
            Self::QuarantineLowQuality => "QUARANTINE_LOW_QUALITY",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq_and_subscription_tier_invariants() {
        assert!(constant_time_eq(b"super-secret-123", b"super-secret-123"));
        assert!(!constant_time_eq(b"super-secret-123", b"super-secret-124"));
        assert!(!constant_time_eq(b"short", b"longer-secret"));

        // Swarm-First invariant verification
        assert!(!SubscriptionTier::from_str_lossy("beta_mvp").allows_private_vault());
        assert!(!SubscriptionTier::from_str_lossy("free").allows_private_vault());
        assert!(!SubscriptionTier::from_str_lossy("local_unlimited").allows_private_vault());
        assert!(SubscriptionTier::from_str_lossy("pro_monthly").allows_private_vault());
        assert!(SubscriptionTier::from_str_lossy("founder_unlimited").allows_private_vault());
    }
}
