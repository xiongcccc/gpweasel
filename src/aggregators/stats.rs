use std::{any::Any, collections::HashMap, time::Duration};

use chrono::{DateTime, Local};

use crate::{
    aggregators::Aggregator, duration::extract_duration, error::Result, format::Format,
    severity::Severity,
};

#[derive(Clone, Default)]
pub struct StatsAggregator {
    total_events: u64,
    severity_counts: HashMap<Severity, u64>,
    slow_events: u64,
    max_duration: Option<Duration>,
    connection_attempts: u64,
    authenticated_connections: u64,
    connection_failures: u64,
    lock_events: u64,
    missing_user_events: u64,
    missing_database_events: u64,
    missing_host_events: u64,
    by_user: HashMap<String, u64>,
    by_database: HashMap<String, u64>,
    by_host: HashMap<String, u64>,
}

impl StatsAggregator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Aggregator for StatsAggregator {
    fn update(
        &mut self,
        record: &[u8],
        fmt: &Format,
        severity: Severity,
        _log_time: DateTime<Local>,
    ) -> Result<()> {
        self.total_events += 1;
        *self.severity_counts.entry(severity).or_insert(0) += 1;

        if let Some(duration) = extract_duration(record) {
            self.slow_events += 1;
            self.max_duration = Some(self.max_duration.map_or(duration, |max| max.max(duration)));
        }

        if let Some(message) = fmt.message_from_bytes(record) {
            if message.starts_with(b"connection received:") {
                self.connection_attempts += 1;
            }
            if message.starts_with(b"connection authorized:") {
                self.authenticated_connections += 1;
            }
            if message_has_lock_signal(message) {
                self.lock_events += 1;
            }
        }

        if (severity == Severity::Fatal)
            && (memchr::memmem::find(record, b"password authentication failed").is_some()
                || memchr::memmem::find(record, b"is not permitted to log in").is_some())
        {
            self.connection_failures += 1;
        }

        if !count_field(fmt.user_from_bytes(record), &mut self.by_user) {
            self.missing_user_events += 1;
        }
        if !count_field(fmt.db_from_bytes(record), &mut self.by_database) {
            self.missing_database_events += 1;
        }
        if !count_field(fmt.host_from_bytes(record), &mut self.by_host) {
            self.missing_host_events += 1;
        }

        Ok(())
    }

    fn merge_box(&mut self, other: &dyn Aggregator) {
        let other = other
            .as_any()
            .downcast_ref::<StatsAggregator>()
            .expect("Aggregator type mismatch");

        self.total_events += other.total_events;
        self.slow_events += other.slow_events;
        self.connection_attempts += other.connection_attempts;
        self.authenticated_connections += other.authenticated_connections;
        self.connection_failures += other.connection_failures;
        self.lock_events += other.lock_events;
        self.missing_user_events += other.missing_user_events;
        self.missing_database_events += other.missing_database_events;
        self.missing_host_events += other.missing_host_events;
        self.max_duration = match (self.max_duration, other.max_duration) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        merge_counts(&mut self.severity_counts, &other.severity_counts);
        merge_counts(&mut self.by_user, &other.by_user);
        merge_counts(&mut self.by_database, &other.by_database);
        merge_counts(&mut self.by_host, &other.by_host);
    }

    fn print(&mut self) {
        crate::outln!("Log summary:");
        crate::outln!("  total events: {}", self.total_events);
        crate::outln!("  duration events: {}", self.slow_events);
        if let Some(max_duration) = self.max_duration {
            crate::outln!("  max duration: {:?}", max_duration);
        }
        crate::outln!("  lock events: {}", self.lock_events);
        crate::outln!(
            "  records without user/database/host: {}/{}/{}",
            self.missing_user_events,
            self.missing_database_events,
            self.missing_host_events
        );
        crate::outln!("  connection attempts: {}", self.connection_attempts);
        crate::outln!(
            "  authenticated connections: {}",
            self.authenticated_connections
        );
        crate::outln!("  connection failures: {}", self.connection_failures);

        crate::outln!("Severity counts:");
        for (severity, count) in sorted_counts(&self.severity_counts) {
            crate::outln!("  {count:>6}  {severity}");
        }

        print_top("Top users:", &self.by_user, 10);
        print_top("Top databases:", &self.by_database, 10);
        print_top("Top hosts:", &self.by_host, 10);
    }

    fn boxed_clone(&self) -> Box<dyn Aggregator> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn message_has_lock_signal(message: &[u8]) -> bool {
    memchr::memmem::find(message, b"still waiting").is_some()
        || memchr::memmem::find(message, b"deadlock").is_some()
        || memchr::memmem::find(message, b"lock timeout").is_some()
}

fn count_field(field: Option<&[u8]>, counts: &mut HashMap<String, u64>) -> bool {
    let Some(value) = field.filter(|value| !value.is_empty()) else {
        return false;
    };

    *counts
        .entry(String::from_utf8_lossy(value).to_string())
        .or_insert(0) += 1;
    true
}

fn merge_counts<K>(target: &mut HashMap<K, u64>, source: &HashMap<K, u64>)
where
    K: Eq + std::hash::Hash + Clone,
{
    for (key, count) in source {
        *target.entry(key.clone()).or_insert(0) += count;
    }
}

fn sorted_counts<K>(map: &HashMap<K, u64>) -> Vec<(&K, &u64)>
where
    K: Ord,
{
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    entries
}

fn print_top(title: &str, counts: &HashMap<String, u64>, limit: usize) {
    crate::outln!("{title}");
    for (value, count) in sorted_counts(counts).into_iter().take(limit) {
        crate::outln!("  {count:>6}  {value}");
    }
}
