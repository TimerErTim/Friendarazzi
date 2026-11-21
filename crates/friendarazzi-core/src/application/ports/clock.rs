use chrono::{DateTime, NaiveDate, Utc};

#[async_trait::async_trait]
pub trait Clock: Send + Sync {
    /// Returns the current date and time in the UTC timezone.
    fn now(&self) -> DateTime<Utc>;

    /// Returns the current date in the UTC timezone.
    fn today(&self) -> NaiveDate;

    /// Suspend execution until the clock reaches `deadline`.
    async fn sleep_until(&self, deadline: DateTime<Utc>);
}
