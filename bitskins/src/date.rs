use derive_more::{Deref, Display, From};
use serde::{de, Deserialize, Deserializer};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Deref, From, Clone, Copy, Display, PartialOrd, PartialEq)]
pub struct DateTime(pub OffsetDateTime);

impl DateTime {
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    pub fn min() -> Self {
        Self(OffsetDateTime::from_unix_timestamp(0).unwrap())
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        OffsetDateTime::parse(&String::deserialize(deserializer)?, &Rfc3339)
            .map(DateTime)
            .map_err(de::Error::custom)
    }
}
