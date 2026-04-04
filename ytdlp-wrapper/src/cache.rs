use snafu::{OptionExt, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CacheError {
    #[snafu(display("Invalid duration: {}", duration))]
    InvalidDuration { duration: u64 },
    #[snafu(display("Failed to get item right after inserting it"))]
    RoundtripFailed,
}

type Result<T> = std::result::Result<T, CacheError>;

pub struct CacheItem<T> {
    pub item: T,
    expiration: std::time::SystemTime,
}

impl<T> CacheItem<T> {
    pub fn with_duration(item: T, duration: std::time::Duration) -> Result<Self> {
        let expiration =
            std::time::SystemTime::now()
                .checked_add(duration)
                .context(InvalidDurationSnafu {
                    duration: duration.as_secs(),
                })?;
        Ok(Self { item, expiration })
    }

    pub fn is_expired(&self, cmp_time: &std::time::SystemTime) -> bool {
        self.expiration.ge(cmp_time)
    }
}
