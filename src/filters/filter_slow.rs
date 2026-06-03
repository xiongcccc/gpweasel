use std::time::Duration;

use crate::{duration::extract_duration, filters::Filter, format::Format};

#[derive(Clone)]
pub struct FilterSlow {
    threshold: Duration,
}

impl FilterSlow {
    pub fn new(threshold: Duration) -> Self {
        FilterSlow { threshold }
    }
}

impl Filter for FilterSlow {
    fn matches(&self, record: &[u8], _fmt: &Format) -> bool {
        if let Some(duration) = extract_duration(record)
            && duration > self.threshold
        {
            return true;
        }
        false
    }
}
