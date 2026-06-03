# gpweasel

A simple CLI-oriented Greenplum Database log parser, adapted from the PostgreSQL-focused `pgweasel` idea.

`gpweasel` is meant for DBAs who want a small, dependency-light command they can run directly on database hosts against master or segment log files. It focuses on Greenplum `gpdb-*.csv` server logs and keeps compatibility with common PostgreSQL CSV/plain logs where the existing parser can still understand them.

Greenplum server logs are CSV records. The parser understands the Greenplum field layout documented for database server logs, including:

* `event_time` in field 1
* `user_name` and `database_name` in fields 2 and 3
* `remote_host` in field 6
* `gp_session_id`, `gp_command_count`, and segment/slice fields for correlation
* `event_severity` in field 17
* `event_message` in field 19

## Status

This project is in beta. Commands and output may still change.

## Build

```sh
cargo build --release
```

The built binary will be in `target/release/gpweasel`.

## Usage

### errors

```sh
gpweasel errors /data/master/gpseg-1/pg_log/gpdb-2026-06-03_000000.csv
gpweasel errors -l error /data/master/gpseg-1/pg_log/gpdb-*.csv
gpweasel errors --begin 10m /data/master/gpseg-1/pg_log/gpdb-*.csv
gpweasel errors top /data/master/gpseg-1/pg_log/gpdb-*.csv
gpweasel errors hist -b 10m -l warning /data/master/gpseg-1/pg_log/gpdb-*.csv
```

### slow

```sh
gpweasel slow 1s /data/master/gpseg-1/pg_log/gpdb-*.csv
gpweasel slow top /data/master/gpseg-1/pg_log/gpdb-*.csv
```

### locks

```sh
gpweasel locks /data/master/gpseg-1/pg_log/gpdb-*.csv
```

### system

```sh
gpweasel system /data/master/gpseg-1/pg_log/gpdb-*.csv
```

### connections

```sh
gpweasel connections /data/master/gpseg-1/pg_log/gpdb-*.csv
```

`connections` summarizes connection attempts, authorized connections, hosts, users, databases, application names when available, and 10-minute time buckets.

## Notes

* Greenplum CSV records can contain multiline fields; `gpweasel` uses timestamp-like record starts to keep multiline records together.
* For segment-wide investigations, collect or glob master and segment `pg_log/gpdb-*.csv` files and pass them as input paths.
* `gplogfilter` remains the canonical Greenplum utility. `gpweasel` is intended to complement it with compact summaries such as top errors, histograms, slow-query filtering, and connection aggregation.

## License

This project keeps the upstream Apache License.
