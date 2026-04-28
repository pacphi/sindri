//! [`SecretValue`] — a zeroising byte buffer with an optional description.
//!
//! The `Debug` implementation intentionally masks the bytes so that accidental
//! `{:?}` formatting in logs does not leak secrets.

use serde::{Deserialize, Serialize};

/// A secret value held as raw bytes plus an optional human-readable description.
///
/// ## Security properties
///
/// * **Drop-zeroing** — the byte buffer is overwritten with zeros when the
///   value is dropped, limiting the window for an attacker to find secret
///   material in heap memory.
/// * **Debug masking** — `{:?}` output always renders as `SecretValue { bytes:
///   [REDACTED NN bytes], description: … }` so secrets never appear in logs.
#[derive(Clone, Serialize, Deserialize)]
pub struct SecretValue {
    #[serde(with = "serde_bytes_vec")]
    bytes: Vec<u8>,
    pub description: Option<String>,
}

impl SecretValue {
    /// Construct a new `SecretValue` from a byte vector.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            description: None,
        }
    }

    ///
    /// Named `from_plaintext` rather than `from_str` to avoid confusion with
    /// the standard [`std::str::FromStr`] trait method.
    pub fn from_plaintext(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }

    /// Attach a human-readable description (stored alongside the secret, never
    /// the secret itself).
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Expose the raw bytes.  Treat the returned slice carefully — do not
    /// assign it to a `String` log message.
    pub fn expose_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Attempt to decode the bytes as a UTF-8 string.
    pub fn expose_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes)
    }

    /// Return the number of bytes stored.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return `true` if the byte buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// Zeroise the bytes on drop to reduce the window for heap leaks.
impl Drop for SecretValue {
    fn drop(&mut self) {
        for b in self.bytes.iter_mut() {
            *b = 0;
        }
    }
}

/// Mask the byte contents to avoid accidental log leaks.
impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretValue")
            .field("bytes", &format!("[REDACTED {} bytes]", self.bytes.len()))
            .field("description", &self.description)
            .finish()
    }
}

// ── serde helper that serialises Vec<u8> as a base64 string ─────────────────

mod serde_bytes_vec {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let b64 = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(serde::de::Error::custom)
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_does_not_contain_secret_bytes() {
        let v = SecretValue::from_plaintext("super-secret-value-abc123");
        let dbg = format!("{:?}", v);
        assert!(
            !dbg.contains("super-secret-value-abc123"),
            "Debug output leaked secret: {dbg}"
        );
        assert!(
            dbg.contains("REDACTED"),
            "Debug output missing REDACTED marker: {dbg}"
        );
    }

    #[test]
    fn expose_str_round_trips() {
        let original = "hello sindri";
        let v = SecretValue::from_plaintext(original);
        assert_eq!(v.expose_str().unwrap(), original);
    }

    #[test]
    fn len_and_is_empty() {
        let empty = SecretValue::new(vec![]);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let nonempty = SecretValue::from_plaintext("x");
        assert!(!nonempty.is_empty());
        assert_eq!(nonempty.len(), 1);
    }

    #[test]
    fn description_round_trips() {
        let v = SecretValue::from_plaintext("tok").with_description("my API token");
        assert_eq!(v.description.as_deref(), Some("my API token"));
    }

    #[test]
    fn serde_round_trip() {
        let v = SecretValue::from_plaintext("round-trip").with_description("test");
        let json = serde_json::to_string(&v).unwrap();
        let back: SecretValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back.expose_str().unwrap(), "round-trip");
        assert_eq!(back.description.as_deref(), Some("test"));
    }
}
