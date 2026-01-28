// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Serde helpers for Duration serialization.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct DurationDef {
    secs: u64,
    nanos: u32,
}

pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    DurationDef {
        secs: duration.as_secs(),
        nanos: duration.subsec_nanos(),
    }
    .serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let def = DurationDef::deserialize(deserializer)?;
    Ok(Duration::new(def.secs, def.nanos))
}
