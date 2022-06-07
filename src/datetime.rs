//! Convert a Windows 64bits timestamp to a [DateTime].

use crate::error::RustructError;
use anyhow::Result;

use chrono::{DateTime, Utc, NaiveDateTime};

/// Convert an u64 (Windows 64bits timestamp) to a [DateTime].
pub struct WindowsTimestamp(pub u64);

impl WindowsTimestamp
{
  /// Return a [DateTime]::<[Utc]> from Windows 64bits timestamp. 
  pub fn to_datetime(&self) -> Result<DateTime::<Utc>>
  {
    if self.0 == 0
    {
      return Err(RustructError::Unknown("Can't convert to datetime, time is null".into()).into());
    }

    if self.0 < 116444736000000000
    {
      return Err(RustructError::Unknown("Can't convert to datetime, time value is too small".into()).into());
    }

    let time = (self.0 - 116444736000000000) / 10000000;
    let time = NaiveDateTime::from_timestamp(time as i64, 0);
    Ok(DateTime::<Utc>::from_utc(time, Utc))
  }
}
