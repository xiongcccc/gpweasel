use std::{any::Any, collections::BTreeMap, time::Duration};

use chrono::{DateTime, Local, TimeZone};

use crate::{aggregators::Aggregator, error::Result, format::Format, severity::Severity};

#[derive(Clone)]
pub struct PeaksAggregator {
    bucket_width: Duration,
    buckets: BTreeMap<i64, u64>,
    limit: usize,
}

impl PeaksAggregator {
    pub fn new(bucket_width: Duration, limit: usize) -> Self {
        Self {
            bucket_width,
            buckets: BTreeMap::new(),
            limit,
        }
    }

    fn bucket(&self, log_time: DateTime<Local>) -> Result<i64> {
        let width: i64 = self.bucket_width.as_secs().try_into().map_err(|_| {
            crate::error::Error::TimestampBeforeEpoch {
                timestamp: log_time.to_rfc2822(),
            }
        })?;

        Ok((log_time.timestamp() / width) * width)
    }
}

impl Aggregator for PeaksAggregator {
    fn update(
        &mut self,
        _record: &[u8],
        _fmt: &Format,
        _severity: Severity,
        log_time: DateTime<Local>,
    ) -> Result<()> {
        let bucket = self.bucket(log_time)?;
        *self.buckets.entry(bucket).or_insert(0) += 1;
        Ok(())
    }

    fn merge_box(&mut self, other: &dyn Aggregator) {
        let other = other
            .as_any()
            .downcast_ref::<PeaksAggregator>()
            .expect("Aggregator type mismatch");

        for (&bucket, &count) in &other.buckets {
            *self.buckets.entry(bucket).or_insert(0) += count;
        }
    }

    fn print(&mut self) {
        let mut entries: Vec<_> = self.buckets.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        crate::outln!("Top {} busiest time buckets:", entries.len().min(self.limit));
        for (&bucket, &count) in entries.into_iter().take(self.limit) {
            if let Some(time) = Local.timestamp_opt(bucket, 0).single() {
                crate::outln!("  {count:>6}  {}", time.format("%Y-%m-%d %H:%M:%S"));
            }
        }
    }

    fn boxed_clone(&self) -> Box<dyn Aggregator> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
