use std::{any::Any, collections::HashMap, time::Duration};

use chrono::{DateTime, Local, TimeZone};

use crate::{aggregators::Aggregator, error::Result, format::Format, severity::Severity};

#[derive(Clone, Debug, Default)]
pub struct ConnectionsAggregator {
    total_connection_attempts: u64,
    total_authenticated: u64,
    total_authenticated_ssl: u64,
    connection_failures: u64,
    connections_by_host: HashMap<String, u64>,
    connections_by_database: HashMap<String, u64>,
    connections_by_user: HashMap<String, u64>,
    connections_by_appname: HashMap<String, u64>,
    connection_attempts_by_time_bucket: HashMap<String, u64>,
    bucket_interval: Duration,
}

impl ConnectionsAggregator {
    pub fn new() -> Self {
        ConnectionsAggregator {
            total_connection_attempts: 0,
            total_authenticated: 0,
            total_authenticated_ssl: 0,
            connection_failures: 0,
            connections_by_host: HashMap::new(),
            connections_by_database: HashMap::new(),
            connections_by_user: HashMap::new(),
            connections_by_appname: HashMap::new(),
            connection_attempts_by_time_bucket: HashMap::new(),
            bucket_interval: Duration::from_mins(10),
        }
    }
}

impl Aggregator for ConnectionsAggregator {
    fn update(
        &mut self,
        record: &[u8],
        fmt: &Format,
        severity: Severity,
        log_time: DateTime<Local>,
    ) -> Result<()> {
        if (severity == Severity::Fatal)
            && (memchr::memmem::find(record, b"password authentication failed").is_some()
                || memchr::memmem::find(record, b"is not permitted to log in").is_some())
        {
            self.connection_failures += 1;
            return Ok(());
        }

        if severity != Severity::Log {
            return Ok(());
        }

        let Some(message) = fmt.message_from_bytes(record) else {
            return Ok(());
        };

        if message.starts_with(b"connection received:") {
            self.total_connection_attempts += 1;
            let host = fmt.host_from_bytes(record).unwrap_or(b"unknown");
            self.connections_by_host
                .entry(String::from_utf8_lossy(host).to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            let bucket_time = round_floor(log_time, self.bucket_interval)?;
            let bucket_time_str = bucket_time.to_string();
            self.connection_attempts_by_time_bucket
                .entry(bucket_time_str)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        if message.starts_with(b"connection authorized:") {
            self.total_authenticated += 1;
            if memchr::memmem::find(message, b"SSL enabled").is_some() {
                self.total_authenticated_ssl += 1;
            }

            let user = fmt.user_from_bytes(record).unwrap_or(b"unknown");
            self.connections_by_user
                .entry(String::from_utf8_lossy(user).to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            let db = fmt.db_from_bytes(record).unwrap_or(b"unknown");
            self.connections_by_database
                .entry(String::from_utf8_lossy(db).to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            let appname = fmt.appname_from_bytes(record).unwrap_or(b"unknown");
            self.connections_by_appname
                .entry(String::from_utf8_lossy(appname).to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        Ok(())
    }

    fn merge_box(&mut self, other: &dyn Aggregator) {
        let other = other
            .as_any()
            .downcast_ref::<ConnectionsAggregator>()
            .expect("Aggregator type mismatch");

        self.total_connection_attempts += other.total_connection_attempts;
        self.total_authenticated += other.total_authenticated;
        self.total_authenticated_ssl += other.total_authenticated_ssl;
        self.connection_failures += other.connection_failures;

        for (host, count) in &other.connections_by_host {
            *self.connections_by_host.entry(host.clone()).or_insert(0) += count;
        }

        for (user, count) in &other.connections_by_user {
            *self.connections_by_user.entry(user.clone()).or_insert(0) += count;
        }

        for (db, count) in &other.connections_by_database {
            *self.connections_by_database.entry(db.clone()).or_insert(0) += count;
        }

        for (appname, count) in &other.connections_by_appname {
            *self
                .connections_by_appname
                .entry(appname.clone())
                .or_insert(0) += count;
        }

        for (bucket, count) in &other.connection_attempts_by_time_bucket {
            *self
                .connection_attempts_by_time_bucket
                .entry(bucket.clone())
                .or_insert(0) += count;
        }
    }

    fn print(&mut self) {
        crate::outln!(
            "Total connection attempts: {}",
            self.total_connection_attempts
        );
        crate::outln!(
            "Total authenticated connections: {}",
            self.total_authenticated
        );
        crate::outln!(
            "Total authenticated SSL connections: {}",
            self.total_authenticated_ssl
        );
        crate::outln!("Total connection failures: {}", self.connection_failures);
        crate::outln!("Connections by host:");
        for (host, count) in sorted_counts(&self.connections_by_host) {
            crate::outln!("  {count:>6}  {host}");
        }
        crate::outln!("Connections by database:");
        for (db, count) in sorted_counts(&self.connections_by_database) {
            crate::outln!("  {count:>6}  {db}");
        }
        crate::outln!("Connections by user:");
        for (user, count) in sorted_counts(&self.connections_by_user) {
            crate::outln!("  {count:>6}  {user}");
        }
        crate::outln!("Connections by application name:");
        for (appname, count) in sorted_counts(&self.connections_by_appname) {
            crate::outln!("  {count:>6}  {appname}");
        }
        crate::outln!("Connections by time bucket:");
        for (appname, count) in sorted_counts(&self.connection_attempts_by_time_bucket) {
            crate::outln!("  {count:>6}  {appname}");
        }
    }

    fn boxed_clone(&self) -> Box<dyn Aggregator> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn sorted_counts(map: &HashMap<String, u64>) -> Vec<(&String, &u64)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    entries
}

fn local_datetime_to_u128(dt: DateTime<Local>) -> Result<u128> {
    let ts = dt.timestamp();
    ts.try_into()
        .map_err(|_| crate::Error::TimestampBeforeEpoch {
            timestamp: dt.to_string(),
        })
}

fn duration_to_nanos(d: Duration) -> u128 {
    u128::from(d.as_secs()) * 1_000_000_000 + u128::from(d.subsec_nanos())
}

fn datetime_to_nanos(dt: DateTime<Local>) -> Result<u128> {
    let ts = local_datetime_to_u128(dt)?;
    Ok(ts as u128 * 1_000_000_000 + u128::from(dt.timestamp_subsec_nanos()))
}

fn nanos_to_datetime(nanos: u128) -> Result<DateTime<Local>> {
    let secs = nanos / 1_000_000_000;
    let nsecs = (nanos % 1_000_000_000) as u32;

    Ok(Local
        .timestamp_opt(
            secs.try_into()
                .map_err(|_| crate::Error::TimestampBeforeEpoch {
                    timestamp: format!("nanos: {nanos}"),
                })?,
            nsecs,
        )
        .single()
        .expect("valid timestamp"))
}

pub fn round_floor(dt: DateTime<Local>, interval: Duration) -> Result<DateTime<Local>> {
    let i = duration_to_nanos(interval);
    let t = datetime_to_nanos(dt)?;

    nanos_to_datetime(t - (t % i))
}
