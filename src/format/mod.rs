mod csv;
mod plain;

use chrono::{DateTime, Local};

use crate::error::Result;
use crate::severity::Severity;
use crate::util::parse_timestamp_from_string;

pub enum Format {
    Csv,
    Plain,
}

impl Format {
    pub fn from_file_extension(file_name: &str) -> Self {
        if std::path::Path::new(file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        {
            Format::Csv
        } else {
            Format::Plain
        }
    }

    pub fn severity_from_string(&self, text: &str) -> Severity {
        match self {
            Format::Csv => crate::format::csv::severity(text.as_bytes()),
            Format::Plain => {
                let severity = Severity::from_log_string(text);
                if severity == Severity::Log {
                    crate::format::csv::severity(text.as_bytes())
                } else {
                    severity
                }
            }
        }
    }

    pub fn timestamp_from_bytes(&self, record: &[u8]) -> Result<DateTime<Local>> {
        let timestamp = match self {
            Format::Csv => crate::format::csv::timestamp(record),
            Format::Plain => {
                crate::format::plain::timestamp(record)
                    .or_else(|| crate::format::csv::timestamp(record))
            }
        }
        .ok_or("Missing timestamp")?;

        let timestamp = std::str::from_utf8(timestamp).map_err(|err| err.to_string())?;
        Ok(parse_timestamp_from_string(timestamp)?)
    }

    pub fn message_from_bytes<'a>(&self, record: &'a [u8]) -> Option<&'a [u8]> {
        match self {
            Format::Plain => {
                crate::format::plain::message(record)
                    .or_else(|| crate::format::csv::message(record))
            }
            Format::Csv => crate::format::csv::message(record),
        }
    }

    pub fn host_from_bytes<'a>(&self, record: &'a [u8]) -> Option<&'a [u8]> {
        match self {
            Format::Csv => crate::format::csv::host(record),
            Format::Plain => {
                extract_after_needle(record, b"host=")
                    .or_else(|| crate::format::csv::host(record))
            }
        }
    }

    pub fn user_from_bytes<'a>(&self, record: &'a [u8]) -> Option<&'a [u8]> {
        match self {
            Format::Csv => crate::format::csv::user(record),
            Format::Plain => {
                extract_after_needle(record, b"user=")
                    .or_else(|| crate::format::csv::user(record))
            }
        }
    }

    pub fn db_from_bytes<'a>(&self, record: &'a [u8]) -> Option<&'a [u8]> {
        match self {
            Format::Csv => crate::format::csv::db(record),
            Format::Plain => {
                extract_after_needle(record, b"database=")
                    .or_else(|| crate::format::csv::db(record))
            }
        }
    }

    pub fn appname_from_bytes<'a>(&self, record: &'a [u8]) -> Option<&'a [u8]> {
        match self {
            Format::Csv => crate::format::csv::appname(record),
            Format::Plain => extract_after_needle(record, b"application_name=")
                .or_else(|| crate::format::csv::appname(record)),
        }
    }
}

#[inline]
pub fn extract_after_needle<'a>(record: &'a [u8], needle: &'a [u8]) -> Option<&'a [u8]> {
    if let Some(pos) = memchr::memmem::find(record, needle) {
        let start = pos + needle.len();
        let mut end = start + 1;
        while end < record.len()
            && record[end] != b' '
            && record[end] != b','
            && record[end] != b'\"'
        {
            end += 1;
        }
        Some(&record[start..end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_extract_after_csv() {
        let record = b"2025-12-01 08:50:20.071 EET,\"binsy\",\"binsy\",1653291,\"10.203.8.108:50372\",692d3aac.193a2b,3,\"authentication\",2025-12-01 08:50:20 EET,104/121,0,LOG,00000,\"connection authorized: user=binsy database=binsy\",,,,,,,,,\"\",\"client backend\",,0";
        let extracted = Format::Csv.user_from_bytes(record).unwrap();
        assert_eq!(extracted, b"binsy");
    }

    #[test]
    fn test_user_extract_after_log() {
        let record = b"2021-02-14 01:34:02 CET [30291]: db=template1,user=postgres,app=[unknown],client=[local] LOG:  connection authorized: user=postgres database=template1 application_name=psql";
        let needle = b"user=";
        let extracted = extract_after_needle(record, needle).unwrap();
        assert_eq!(extracted, b"postgres");
    }
}
