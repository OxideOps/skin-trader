use serde::{de, Deserialize, Deserializer};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug)]
pub struct DateTime(pub OffsetDateTime);

impl DateTime {
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        OffsetDateTime::parse(&String::deserialize(deserializer)?, &Rfc3339)
            .map(DateTime)
            .map_err(de::Error::custom)
    }
}
