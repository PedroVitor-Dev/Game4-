//! Collection domain types; file import and hashing are not implemented yet.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Unverified,
    Verified,
    Changed,
    Missing,
    Inaccessible,
}
