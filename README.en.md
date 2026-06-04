# gpweasel

[中文](README.md) | [English](README.en.md)

`gpweasel` is a CLI-oriented Greenplum Database / YMatrix log parser for DBAs and operations engineers. It follows the spirit of the PostgreSQL-focused `pgweasel`, but understands the Greenplum/YMatrix `gpdb-*.csv` server log layout.

The goal is not to generate an HTML report. The goal is quick server-side triage against existing log files:

* What are the most frequent errors?
* When did errors spike?
* Which SQL statements were slowest?
* Did lock waits, lock timeouts, or deadlocks happen?
* Where did connections come from: host, database, user?
* Which time windows produced the most log volume?
* What is the high-level health summary for a log set?
* Did startup, shutdown, reload, checkpoint, background worker, extension, replication, or WAL events happen?

## Supported Log Formats

The primary target is Greenplum/YMatrix CSV server logs, for example:

```text
gpdb-2026-06-03_095933.csv
```

Common log locations:

```sh
$MASTER_DATA_DIRECTORY/log/gpdb-*.csv
$MASTER_DATA_DIRECTORY/pg_log/gpdb-*.csv
```

`gpweasel` currently uses these Greenplum CSV fields:

```text
field 1   event_time
field 2   user_name
field 3   database_name
field 6   remote_host
field 9   transaction_id
field 10  gp_session_id
field 11  gp_command_count
field 12  gp_segment
field 17  event_severity
field 19  event_message
```

PostgreSQL CSV and plain logs remain partially supported for compatibility with the original parser.

Some Greenplum/YMatrix startup files may be named `startup.log` while still containing CSV-style fields. `gpweasel` falls back to CSV-style extraction for severity, message, host, user, and database fields, so broad globs are supported:

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
gpweasel connections $MASTER_DATA_DIRECTORY/log/*
```

## Build And Install

Install Rust on the database server, then build:

```sh
git clone git@github.com:xiongcccc/gpweasel.git
cd gpweasel
cargo build --release
```

The binary is:

```sh
target/release/gpweasel
```

Optional system-wide install:

```sh
sudo install -m 0755 target/release/gpweasel /usr/local/bin/gpweasel
gpweasel --help
```

If GitHub SSH port 22 is blocked, use SSH over port 443:

```sh
git clone ssh://git@ssh.github.com:443/xiongcccc/gpweasel.git
```

## Recommended Database Logging

For full validation and useful production diagnostics, these settings are recommended:

```sh
gpconfig -c log_destination -v csvlog
gpconfig -c logging_collector -v on
gpconfig -c log_directory -v log
gpconfig -c log_filename -v 'gpdb-%Y-%m-%d_%H%M%S.csv'

gpconfig -c log_connections -v on
gpconfig -c log_disconnections -v on
gpconfig -c log_min_duration_statement -v 500
gpconfig -c log_lock_waits -v on
gpconfig -c deadlock_timeout -v 1s

gpstop -u
```

Notes:

* Run `gpconfig` as the Greenplum/YMatrix administration user, for example `gpadmin` or `mxadmin`.
* `gpstop -u` reloads parameters that can be applied without a restart.
* `logging_collector` usually requires a database restart if it was previously `off`; use a planned maintenance window, for example `gpstop -ra`.
* `log_min_duration_statement = 0` is useful for short tests, but too noisy for normal production use.
* `log_connections` and `log_disconnections` are required for meaningful `connections` output.
* `log_lock_waits` plus a reasonable `deadlock_timeout` is required for lock-wait analysis.

## Command Layout

Basic syntax:

```sh
gpweasel [GLOBAL OPTIONS] <COMMAND> [COMMAND OPTIONS] <LOG FILES...>
```

Examples:

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
gpweasel errors top $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -b 30m slow top --max 5 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -P 40 errors -l error $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

## Global Options

Global options must appear before the subcommand:

```sh
gpweasel -b 30m stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-03 18:06" errors top $MASTER_DATA_DIRECTORY/log/*
```

Common global options:

```sh
-b, --begin <BEGIN>      Start-time filter
-e, --end <END>          End-time filter
-m, --mask <MASK>        Text contains filter, often used with a timestamp prefix
-P, --page-size <LINES>  Interactive paging; pause after N output lines
-d, --debug              Print debug information
```

### Global `-b, --begin`

When `-b` appears before the subcommand, it means start time. Only records after that time are analyzed.

Use cases:

* Look only at the most recent incident window.
* Exclude logs before a known incident start time.
* Reduce output when scanning large log sets.

Examples:

```sh
gpweasel -b 30m stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -b 2h errors top $MASTER_DATA_DIRECTORY/log/*
gpweasel -b today slow top --max 10 $MASTER_DATA_DIRECTORY/log/*
gpweasel -b "2026-06-03 18:00:00" locks $MASTER_DATA_DIRECTORY/log/*
```

### Global `-m, --mask`

When `-m` appears before the subcommand, it means mask filtering. It is a text-contains filter. DBAs commonly use it with a timestamp prefix to narrow output to a minute or an hour.

Use cases:

* You already know the incident minute, for example `2026-06-03 18:06`.
* You want to quickly inspect records containing a keyword.
* You use `peaks` first to find a busy window, then drill into it with `--mask`.

Examples:

```sh
gpweasel -m "2026-06-03 18:06" errors $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-03 18" stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "deadlock" errors -l log $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "connection authorized" connections $MASTER_DATA_DIRECTORY/log/*
```

### Global `-P, --page-size`

`-P` enables built-in interactive paging. It is useful when reading output directly on a server terminal.

```sh
gpweasel -P 40 errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel -P 20 slow 500ms $MASTER_DATA_DIRECTORY/log/*
```

Paging is disabled when output is redirected or piped. If tools such as `more` or `head` close the pipe early, `gpweasel` exits quietly instead of printing a broken pipe panic.

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/* | more
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/* | head
```

## How `-b` And `-m` Are Reused

The short options `-b` and `-m` have different meanings depending on where they appear. This is normal scoped argument behavior in clap-style CLIs.

### Before The Subcommand: Global Filters

```sh
gpweasel -b 2h stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-04 10:00" errors top $MASTER_DATA_DIRECTORY/log/*
```

Meaning:

```text
-b / --begin  Analyze records after a start time
-m / --mask   Analyze records containing a text or timestamp prefix
```

### After The Subcommand: Command-Specific Options

```sh
gpweasel peaks -b 10m -m 5 $MASTER_DATA_DIRECTORY/log/*
gpweasel errors hist -b 1h $MASTER_DATA_DIRECTORY/log/*
gpweasel errors top -m 20 $MASTER_DATA_DIRECTORY/log/*
gpweasel slow top -m 10 $MASTER_DATA_DIRECTORY/log/*
```

Meaning:

```text
peaks -b / --bucket       Bucket width for peak detection
peaks -m / --max          Maximum number of peak buckets to show
errors hist -b / --bucket Bucket width for the error histogram
errors top -m / --max     Maximum number of frequent errors to show
slow top -m / --max       Maximum number of slow SQL statements to show
```

### Recommended Style

To avoid confusion, prefer long options when both global and command-specific options are used:

```sh
# Find the top 5 busiest 10-minute windows in the last 2 hours.
gpweasel --begin 2h peaks --bucket 10m --max 5 $MASTER_DATA_DIRECTORY/log/*

# Find the most frequent errors in one known minute.
gpweasel --mask "2026-06-03 18:06" errors top --max 20 $MASTER_DATA_DIRECTORY/log/*

# Build today's error histogram with 30-minute buckets.
gpweasel --begin today errors hist --bucket 30m $MASTER_DATA_DIRECTORY/log/*
```

## Commands

### stats

Print a compact summary for a log set. This is a good first command when inspecting logs.

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -b 2h stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-03 18" stats $MASTER_DATA_DIRECTORY/log/*
```

Output includes:

* Total log records.
* Severity distribution.
* Number of duration records and maximum duration.
* Connection received / authorized / failed counts.
* Lock-related event count.
* Records with missing user/database/host fields.
* Top users / databases / hosts.

Use cases:

* Get an overview before drilling down.
* Quickly see whether errors, slow SQL, connections, or locks look abnormal.
* Choose the next command: `errors`, `slow`, `locks`, or `connections`.

### peaks

Count log records by time bucket and show the busiest windows.

```sh
gpweasel peaks $MASTER_DATA_DIRECTORY/log/*
gpweasel peaks --bucket 1m --max 10 $MASTER_DATA_DIRECTORY/log/*
gpweasel --begin 2h peaks --bucket 5m --max 20 $MASTER_DATA_DIRECTORY/log/*
```

Options:

```sh
-b, --bucket <INTERVAL>  Bucket width, for example 10s, 1m, 10m, 1h. Default: 10m
-m, --max <MAX>         Maximum number of peak buckets to show. Default: 20
```

Use cases:

* You do not know the incident time and need to find log-volume spikes.
* You want to find a busy window first, then drill into `errors`, `slow`, or `locks`.
* You want to spot scheduled jobs, connection storms, error storms, or lock-wait bursts.

Common workflow:

```sh
# Find the 10 busiest one-minute windows in the last 6 hours.
gpweasel --begin 6h peaks --bucket 1m --max 10 $MASTER_DATA_DIRECTORY/log/*

# Suppose 2026-06-04 10:00 is busy. Drill into warnings/errors.
gpweasel --mask "2026-06-04 10:00" errors -l warning $MASTER_DATA_DIRECTORY/log/*

# Check slow SQL in that minute.
gpweasel --mask "2026-06-04 10:00" slow 500ms $MASTER_DATA_DIRECTORY/log/*
```

### errors

List error records. The default minimum severity is `error`, so `ERROR`, `FATAL`, and `PANIC` are shown.

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/*
gpweasel errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" errors -l error $MASTER_DATA_DIRECTORY/log/*
```

### errors top

Show the most frequent error messages.

```sh
gpweasel errors top $MASTER_DATA_DIRECTORY/log/*
gpweasel errors top --max 20 $MASTER_DATA_DIRECTORY/log/*
gpweasel --begin 2h errors top --max 10 $MASTER_DATA_DIRECTORY/log/*
```

Use cases:

* Identify repetitive errors quickly.
* See whether many errors share the same SQL, object, user, or connection problem.
* Triage large repeated-error logs faster than manual reading.

### errors hist

Show an error histogram over time.

```sh
gpweasel errors hist $MASTER_DATA_DIRECTORY/log/*
gpweasel errors hist --bucket 30m -l error $MASTER_DATA_DIRECTORY/log/*
gpweasel --begin today errors hist --bucket 1h $MASTER_DATA_DIRECTORY/log/*
```

Use cases:

* See whether errors are concentrated in a specific time range.
* See whether errors are continuous or bursty.
* Compare error timing with deployments, scaling events, or scheduled jobs.

### slow

List SQL statements whose `duration:` is greater than the threshold. This depends on duration messages in the logs, usually controlled by `log_min_duration_statement`.

```sh
gpweasel slow 500ms $MASTER_DATA_DIRECTORY/log/*
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" slow 1s $MASTER_DATA_DIRECTORY/log/*
```

Use cases:

* Inspect slow SQL details in a time window.
* Investigate slow application response, high resource usage, or lock-related SQL latency.
* Combine with `--begin` or `--mask` to reduce output.

### slow top

Show the slowest SQL statements. The default max is 10.

```sh
gpweasel slow top $MASTER_DATA_DIRECTORY/log/*
gpweasel slow top --max 5 $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" slow top --max 3 $MASTER_DATA_DIRECTORY/log/*
```

### locks

Show lock waits, lock timeouts, deadlocks, and recovery-conflict related records.

```sh
gpweasel locks $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" locks $MASTER_DATA_DIRECTORY/log/*
```

Typical Greenplum lock-wait messages contain text like:

```text
process 2544911 still waiting for AccessShareLock ...
```

Required logging:

```sh
gpconfig -c log_lock_waits -v on
gpconfig -c deadlock_timeout -v 1s
gpstop -u
```

### connections

Summarize connection attempts and authorized connections by host, database, user, application name, and time bucket.

```sh
gpweasel connections $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" connections $MASTER_DATA_DIRECTORY/log/*
```

Required logging:

```sh
gpconfig -c log_connections -v on
gpconfig -c log_disconnections -v on
gpstop -u
```

Use cases:

* Detect connection storms.
* See whether connections mainly come from localhost, application servers, or an unexpected client.
* Count connections by database and user.

### system

Show lifecycle and internal events: startup, shutdown, reload, checkpoints, background workers, extensions, replication, WAL, and similar records.

```sh
gpweasel system $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" system $MASTER_DATA_DIRECTORY/log/*
```

Use cases:

* Investigate instance startup, shutdown, and reload events.
* Inspect checkpoint, background worker, replication, and extension events.
* Correlate system events with errors, slow SQL, or connection spikes.

## DBA Triage Workflow

### 1. Start With The Summary

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
```

Look at:

* Whether `error/fatal/panic` counts are abnormal in `Severity counts`.
* Whether `duration events` and `max duration` are abnormal.
* Whether `lock events` is greater than 0.
* Whether top hosts/databases/users match expectations.

### 2. Find Busy Time Windows

```sh
gpweasel peaks --bucket 1m --max 10 $MASTER_DATA_DIRECTORY/log/*
```

If a minute looks abnormal, for example `2026-06-04 10:00:00`, drill into it:

```sh
gpweasel --mask "2026-06-04 10:00" errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-04 10:00" slow 500ms $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-04 10:00" locks $MASTER_DATA_DIRECTORY/log/*
```

### 3. Check Frequent Errors

```sh
gpweasel errors top --max 20 $MASTER_DATA_DIRECTORY/log/*
```

For large log sets:

```sh
gpweasel --begin 2h errors top --max 20 $MASTER_DATA_DIRECTORY/log/*
```

### 4. Check Slow SQL

```sh
gpweasel slow top --max 10 $MASTER_DATA_DIRECTORY/log/*
```

For detailed records above a threshold:

```sh
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/*
```

### 5. Check Lock Waits

```sh
gpweasel locks $MASTER_DATA_DIRECTORY/log/*
```

If there are many lock waits, combine `peaks` and `--mask`:

```sh
gpweasel --mask "2026-06-04 10:00" locks $MASTER_DATA_DIRECTORY/log/*
```

### 6. Check Connection Sources

```sh
gpweasel connections $MASTER_DATA_DIRECTORY/log/*
```

Look for:

* Unexpected hosts.
* A database or user with a connection spike.
* Sudden increases in connection time buckets.

## Validation Scenario

Use a scratch database or schema. The following example avoids `DELETE` and `DROP`.

```sql
CREATE TABLE IF NOT EXISTS public.gpweasel_probe (
    id int primary key,
    note text,
    updated_at timestamp default now()
);

INSERT INTO public.gpweasel_probe(id, note)
VALUES (1, 'gpweasel validation seed')
ON CONFLICT (id) DO UPDATE
SET note = EXCLUDED.note,
    updated_at = now();

UPDATE public.gpweasel_probe
SET note = note || ' touched'
WHERE id = 1;

SELECT pg_sleep(1), count(*)
FROM public.gpweasel_probe;

SELECT gpweasel_missing_column
FROM public.gpweasel_probe;
```

Generate a lock wait:

Session 1:

```sql
BEGIN;
LOCK TABLE public.gpweasel_probe IN ACCESS EXCLUSIVE MODE;
SELECT pg_sleep(6);
COMMIT;
```

Session 2:

```sql
SET lock_timeout = '3s';
SELECT count(*) FROM public.gpweasel_probe;
```

Then verify:

```sh
gpweasel --mask "YYYY-MM-DD HH:MM" errors top $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" slow top --max 3 $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" locks $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" connections $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" peaks --bucket 1m --max 5 $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" stats $MASTER_DATA_DIRECTORY/log/*
```

## Troubleshooting

### No `connections` Output

Check:

```sh
gpconfig -s log_connections
gpconfig -s log_disconnections
```

You can also verify what the current session sees in `psql`:

```sql
SHOW log_connections;
SHOW log_disconnections;
```

Both should normally be `on`.

### No `locks` Output

Check:

```sh
gpconfig -s log_lock_waits
gpconfig -s deadlock_timeout
```

You can also verify in `psql`:

```sql
SHOW log_lock_waits;
SHOW deadlock_timeout;
```

Lock waits are logged only after `deadlock_timeout`.

### No `slow` Output

Check:

```sh
gpconfig -s log_min_duration_statement
```

You can also verify in `psql`:

```sql
SHOW log_min_duration_statement;
```

If the value is `-1`, statement durations are not logged. For short validation windows, use `0` or a small value. In production, use a reasonable threshold such as `500ms`, `1s`, or a value aligned with your service-level objective.

### Too Much Output

Prefer narrowing the time window:

```sh
gpweasel --begin 30m errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" slow 1s $MASTER_DATA_DIRECTORY/log/*
```

Or use built-in paging:

```sh
gpweasel -P 40 errors -l error $MASTER_DATA_DIRECTORY/log/*
```

### `more` Or `head` Reports Broken Pipe

Current versions handle closed pipes:

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/* | more
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/* | head
```

If an older build still reports broken pipe, pull the latest code and rebuild:

```sh
git pull
cargo build --release
```

## Development

Run checks:

```sh
cargo build --release
cargo test
cargo fmt --all -- --check
```

## License

This project keeps the upstream Apache License.
