use crate::severity::Severity;

const POSTGRES_SEVERITY_FIELD: usize = 12;
const POSTGRES_MESSAGE_FIELD: usize = 14;
const POSTGRES_USER_FIELD: usize = 2;
const POSTGRES_DB_FIELD: usize = 3;
const POSTGRES_HOST_FIELD: usize = 5;
const POSTGRES_APP_FIELD: usize = 22;

const GREENPLUM_SEVERITY_FIELD: usize = 17;
const GREENPLUM_MESSAGE_FIELD: usize = 19;
const GREENPLUM_USER_FIELD: usize = 2;
const GREENPLUM_DB_FIELD: usize = 3;
const GREENPLUM_HOST_FIELD: usize = 6;

pub fn timestamp(record: &[u8]) -> Option<&[u8]> {
    field(record, 1)
}

pub fn severity(record: &[u8]) -> Severity {
    severity_field(record)
        .map(|field| Severity::from_bytes_uppercase(field))
        .unwrap_or(Severity::Log)
}

pub fn message(record: &[u8]) -> Option<&[u8]> {
    if is_greenplum_record(record) {
        field(record, GREENPLUM_MESSAGE_FIELD)
    } else {
        field(record, POSTGRES_MESSAGE_FIELD)
    }
}

pub fn host(record: &[u8]) -> Option<&[u8]> {
    if is_greenplum_record(record) {
        field(record, GREENPLUM_HOST_FIELD)
    } else {
        field(record, POSTGRES_HOST_FIELD).map(strip_port)
    }
}

pub fn user(record: &[u8]) -> Option<&[u8]> {
    if is_greenplum_record(record) {
        field(record, GREENPLUM_USER_FIELD)
    } else {
        field(record, POSTGRES_USER_FIELD)
    }
}

pub fn db(record: &[u8]) -> Option<&[u8]> {
    if is_greenplum_record(record) {
        field(record, GREENPLUM_DB_FIELD)
    } else {
        field(record, POSTGRES_DB_FIELD)
    }
}

pub fn appname(record: &[u8]) -> Option<&[u8]> {
    if is_greenplum_record(record) {
        None
    } else {
        field(record, POSTGRES_APP_FIELD)
    }
}

fn severity_field(record: &[u8]) -> Option<&[u8]> {
    let greenplum_severity = field(record, GREENPLUM_SEVERITY_FIELD);
    if greenplum_severity.is_some_and(is_severity) {
        return greenplum_severity;
    }

    let postgres_severity = field(record, POSTGRES_SEVERITY_FIELD);
    if postgres_severity.is_some_and(is_severity) {
        return postgres_severity;
    }

    None
}

fn is_greenplum_record(record: &[u8]) -> bool {
    field(record, GREENPLUM_SEVERITY_FIELD).is_some_and(is_severity)
}

fn is_severity(field: &[u8]) -> bool {
    matches!(
        field,
        b"DEBUG5"
            | b"DEBUG4"
            | b"DEBUG3"
            | b"DEBUG2"
            | b"DEBUG1"
            | b"LOG"
            | b"INFO"
            | b"NOTICE"
            | b"WARNING"
            | b"ERROR"
            | b"FATAL"
            | b"PANIC"
    )
}

/// Extracts nth field from CSV record
/// Field index is 1-based.
pub fn field(record: &[u8], field_index: usize) -> Option<&[u8]> {
    if field_index == 0 {
        return None;
    }

    let mut in_quotes = false;
    let mut current_field = 1;
    let mut field_start = 0;

    let mut i = 0;
    while i < record.len() {
        match record[i] {
            b'"' => {
                if in_quotes && i + 1 < record.len() && record[i + 1] == b'"' {
                    i += 1; // escaped quote
                } else {
                    in_quotes = !in_quotes;
                }
            }
            b',' if !in_quotes => {
                if current_field == field_index {
                    return Some(strip_csv_quotes(&record[field_start..i]));
                }
                current_field += 1;
                field_start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }

    // Handle last field
    if current_field == field_index {
        Some(strip_csv_quotes(&record[field_start..]))
    } else {
        None
    }
}

#[inline]
fn strip_csv_quotes(field: &[u8]) -> &[u8] {
    if field.len() >= 2 && field[0] == b'"' && field[field.len() - 1] == b'"' {
        &field[1..field.len() - 1]
    } else {
        field
    }
}

fn strip_port(field: &[u8]) -> &[u8] {
    field
        .iter()
        .position(|&b| b == b':')
        .map_or(field, |pos| &field[..pos])
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_message() {
        let line = b"2025-12-01 01:56:57.080 EET,,,1637804,\"10.203.8.108:53096\",692cd9c9.18fdac,1,\"\",2025-12-01 01:56:57 EET,,0,LOG,00000,\"connection received: host=10.203.8.108 port=53096\",,,,,,,,,\"\",\"not initialized\",,0
";

        assert_eq!(
            message(line),
            Some(b"connection received: host=10.203.8.108 port=53096".as_slice())
        );
    }

    #[test]
    fn test_greenplum_message_and_severity() {
        let line = b"2026-06-03 10:15:01.123 UTC,gpadmin,sales,p12345,th-1,10.1.2.3,5432,2026-06-03 10:14:58 UTC,77,con42,cmd7,seg-1,,dx12,100,1,ERROR,42601,\"syntax error at or near \"\"select\"\"\",,,,,,,,,,,";

        assert_eq!(severity(line), Severity::Error);
        assert_eq!(
            message(line),
            Some(b"syntax error at or near \"\"select\"\"".as_slice())
        );
        assert_eq!(user(line), Some(b"gpadmin".as_slice()));
        assert_eq!(db(line), Some(b"sales".as_slice()));
        assert_eq!(host(line), Some(b"10.1.2.3".as_slice()));
    }
}
