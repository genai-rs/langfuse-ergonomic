//! Selection of the Langfuse server API generation the client talks to.

use std::fmt;
use std::str::FromStr;

use crate::error::Error;

/// The Langfuse server API generation used for writes (and, where they differ, reads).
///
/// The setting is explicit: the client never probes the server to detect its version.
///
/// | | [`ApiVersion::V3`] | [`ApiVersion::V4`] (default) |
/// |---|---|---|
/// | Traces and observations | `POST /api/public/ingestion` batches | OTLP/HTTP JSON to `POST /api/public/otel/v1/traces` with `x-langfuse-ingestion-version: 4` |
/// | Scores | `score-create` ingestion events | `POST /api/public/scores` |
/// | Ids | Arbitrary strings | OpenTelemetry hex ids, see [`crate::otel`] |
///
/// Langfuse Cloud removes the v3 ingestion of trace and observation events on
/// 2026-11-16. Self-hosted v3 servers keep the v3 endpoints until they are upgraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ApiVersion {
    /// Langfuse v3: the legacy batch ingestion API.
    V3,
    /// Langfuse v4: OpenTelemetry ingestion and the v4 scores API.
    #[default]
    V4,
}

impl ApiVersion {
    /// Name of the environment variable read by [`crate::ClientBuilder::from_env`].
    pub const ENV_VAR: &'static str = "LANGFUSE_API_VERSION";
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiVersion::V3 => f.write_str("v3"),
            ApiVersion::V4 => f.write_str("v4"),
        }
    }
}

impl FromStr for ApiVersion {
    type Err = Error;

    /// Parse `v3`/`3` or `v4`/`4`, ignoring case and surrounding whitespace.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "v3" | "3" => Ok(ApiVersion::V3),
            "v4" | "4" => Ok(ApiVersion::V4),
            other => Err(Error::Configuration(format!(
                "invalid Langfuse API version '{other}': expected 'v3' or 'v4'"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn default_is_v4() {
        assert_eq!(ApiVersion::default(), ApiVersion::V4);
    }

    #[test]
    fn parses_common_spellings() {
        for (input, expected) in [
            ("v3", ApiVersion::V3),
            ("V3", ApiVersion::V3),
            ("3", ApiVersion::V3),
            (" v4 ", ApiVersion::V4),
            ("4", ApiVersion::V4),
        ] {
            assert_eq!(input.parse::<ApiVersion>().unwrap(), expected, "{input}");
        }
        assert!("v5".parse::<ApiVersion>().is_err());
        assert!("".parse::<ApiVersion>().is_err());
    }

    #[test]
    fn display_round_trips() {
        for version in [ApiVersion::V3, ApiVersion::V4] {
            assert_eq!(version.to_string().parse::<ApiVersion>().unwrap(), version);
        }
    }
}
