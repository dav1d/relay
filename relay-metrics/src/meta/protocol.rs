use std::collections::HashMap;

use chrono::{DateTime, Utc};
use relay_common::time::UnixTimestamp;
use serde::{Deserialize, Serialize};

use crate::MetricResourceIdentifier;

/// A metric metadata item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetricMeta {
    /// Timestamp scope for the contained metadata.
    ///
    /// Metric metadata is collected in daily intervals, so this may be truncated
    /// to the start of the day (UTC) already.
    pub timestamp: StartOfDayUnixTimestamp,

    /// The contained metadata mapped by MRI.
    pub mapping: HashMap<MetricResourceIdentifier<'static>, Vec<Item>>,
}

/// A metadata item.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Item {
    /// A location metadata pointing to the code location where the metric originates from.
    Location(Location),
    /// Unknown item.
    #[serde(other)]
    Unknown,
}

/// A code location.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Location {
    /// The relative file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// The absolute file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abs_path: Option<String>,
    /// The containing module name or path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    /// The containing function name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    /// The line number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineno: Option<u64>,
    /// Source code leading up to `lineno`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_context: Vec<Option<String>>,
    /// Source code of the current line (`lineno`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_line: Option<String>,
    /// Source code of the lines after `lineno`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_context: Vec<Option<String>>,
}

/// A Unix timestamp that is truncated to the start of the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StartOfDayUnixTimestamp(UnixTimestamp);

impl StartOfDayUnixTimestamp {
    /// Creates a new `StartOfDayUnixTimestamp` from a timestamp by truncating it.
    ///
    /// May return none when passed an invalid date, but in practice this never fails
    /// since the [`UnixTimestamp`] is already sufficiently validated.
    pub fn new(ts: UnixTimestamp) -> Option<Self> {
        let dt: DateTime<Utc> = DateTime::from_timestamp(ts.as_secs().try_into().ok()?, 0)?;
        let beginning_of_day = dt.date_naive().and_hms_opt(0, 0, 0)?.and_utc();
        Some(Self(UnixTimestamp::from_datetime(beginning_of_day)?))
    }

    /// Returns the underlying unix timestamp, truncated to the start of the day.
    pub fn as_timestamp(&self) -> UnixTimestamp {
        self.0
    }
}

impl std::ops::Deref for StartOfDayUnixTimestamp {
    type Target = UnixTimestamp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for StartOfDayUnixTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StartOfDayUnixTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ts = UnixTimestamp::deserialize(deserializer)?;
        StartOfDayUnixTimestamp::new(ts)
            .ok_or_else(|| serde::de::Error::custom("invalid timestamp"))
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;

    use super::*;

    #[test]
    fn test_deserialize_null_context() {
        let json = r#"{
            "timestamp": 1715904000,
            "mapping": {
                "d:memory.allocations@allocations": [{ 
                    "abs_path": "/rails/config/initializers/active_job.rb",
                    "function": "block in <main>",
                    "lineno": 5,
                    "filename": "config/initializers/active_job.rb",
                    "pre_context": [null, "  allocations = event.allocations\n", "  allocations=#{allocations}\"\n"],
                    "context_line": "  Sentry::Metrics.distribution('memory.allocations')\n",
                    "post_context": ["end\n",null,null],
                    "type":"location"
                }]
            }
        }"#;

        let r: MetricMeta = serde_json::from_str(json).unwrap();

        assert_json_snapshot!(r, @r###"
        {
          "timestamp": 1715904000,
          "mapping": {
            "d:custom/memory.allocations@allocations": [
              {
                "type": "location",
                "filename": "config/initializers/active_job.rb",
                "abs_path": "/rails/config/initializers/active_job.rb",
                "function": "block in <main>",
                "lineno": 5,
                "pre_context": [
                  null,
                  "  allocations = event.allocations\n",
                  "  allocations=#{allocations}\"\n"
                ],
                "context_line": "  Sentry::Metrics.distribution('memory.allocations')\n",
                "post_context": [
                  "end\n",
                  null,
                  null
                ]
              }
            ]
          }
        }
        "###);
    }
}
