# gpweasel

`gpweasel` is a small CLI-oriented Greenplum Database and YMatrix log parser for DBAs and operations engineers. It is adapted from the PostgreSQL-focused `pgweasel` idea, but understands the Greenplum `gpdb-*.csv` server log layout.

The goal is direct server-side triage: copy or build one binary, point it at existing master or segment log files, and quickly answer questions such as:

* What are the most frequent errors?
* When did errors spike?
* Which SQL statements were slowest?
* Did we have lock waits or lock timeouts?
* How many connections came from each host, database, or user?
* What system events, reloads, checkpoints, startup, or extension events happened?

## Supported Log Format

The primary target is Greenplum/YMatrix CSV server logs named like:

```text
gpdb-2026-06-03_095933.csv
```

Common locations:

```sh
$MASTER_DATA_DIRECTORY/log/gpdb-*.csv
$MASTER_DATA_DIRECTORY/pg_log/gpdb-*.csv
```

Greenplum CSV fields used by `gpweasel` include:

* field 1: `event_time`
* field 2: `user_name`
* field 3: `database_name`
* field 6: `remote_host`
* field 9: `transaction_id`
* field 10: `gp_session_id`
* field 11: `gp_command_count`
* field 12: `gp_segment`
* field 17: `event_severity`
* field 19: `event_message`

PostgreSQL CSV/plain logs remain partially supported for compatibility with the original parser.

## Build And Install

Install Rust with `rustup` on the database server, then build:

```sh
git clone git@github.com:xiongcccc/gpweasel.git
cd gpweasel
cargo build --release
```

The binary is:

```sh
target/release/gpweasel
```

Optional install:

```sh
sudo install -m 0755 target/release/gpweasel /usr/local/bin/gpweasel
gpweasel --help
```

If GitHub SSH port 22 is blocked, use SSH over port 443:

```sh
git clone ssh://git@ssh.github.com:443/xiongcccc/gpweasel.git
```

## Recommended Database Logging

For full validation and useful production diagnostics, these settings are helpful:

```sql
ALTER SYSTEM SET log_destination = 'csvlog';
ALTER SYSTEM SET logging_collector = on;
ALTER SYSTEM SET log_directory = 'log';
ALTER SYSTEM SET log_filename = 'gpdb-%Y-%m-%d_%H%M%S.csv';

ALTER SYSTEM SET log_connections = on;
ALTER SYSTEM SET log_disconnections = on;
ALTER SYSTEM SET log_min_duration_statement = 500;
ALTER SYSTEM SET log_lock_waits = on;
ALTER SYSTEM SET deadlock_timeout = '1s';

SELECT pg_reload_conf();
```

Notes:

* `logging_collector` usually requires restart if it was previously off.
* `log_min_duration_statement = 0` is useful for short tests, but too noisy for normal operations.
* `log_connections` and `log_disconnections` are required for meaningful `connections` output.
* `log_lock_waits` plus a reasonable `deadlock_timeout` is required for `locks`.

## Global Options

Global options must appear before the subcommand:

```sh
gpweasel [OPTIONS] <COMMAND>
```

Useful options:

```sh
-m, --mask <MASK>        Match records containing a timestamp prefix or any text mask
-b, --begin <BEGIN>      Start time, for example 10m, 2h, today, or a timestamp
-e, --end <END>          End time
-P, --page-size <LINES>  Pause after N output lines in an interactive terminal
-d, --debug              Show debug information
```

Examples:

```sh
gpweasel -m "2026-06-03 18:06" errors top $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -b 30m slow top --max 5 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -P 40 errors -l error $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

Paging behavior:

* `--page-size` only pauses when stdin/stdout are interactive terminals.
* It does not pause when output is redirected or piped.
* If a pipe is closed, for example after quitting `more`, `gpweasel` exits quietly instead of printing a broken pipe panic.

## Commands

### errors

List error, fatal, and panic records. The default minimum severity is `error`.

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel errors -l warning $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "2026-06-03 18:06" errors -l error $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

### errors top

Show the most frequent error messages.

```sh
gpweasel errors top $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel errors top --max 20 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

### errors hist

Show an error histogram over time.

```sh
gpweasel errors hist $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel errors hist -b 30m -l error $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

### slow

List statements whose log message contains `duration:` and whose duration is greater than the threshold.

```sh
gpweasel slow 500ms $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "2026-06-03 18:06" slow 1s $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

For large logs, combine `--mask`, `--begin`, or `--page-size` to keep output readable.

### slow top

Show the slowest statements. Default max is 10.

```sh
gpweasel slow top $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel slow top --max 5 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "2026-06-03 18:06" slow top --max 1 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

### locks

Show lock wait, deadlock, and recovery-conflict related records.

```sh
gpweasel locks $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "2026-06-03 18:06" locks $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

Typical Greenplum lock-wait messages include text like:

```text
process 2544911 still waiting for AccessShareLock ...
```

### connections

Summarize connection attempts and authenticated connections by host, database, user, application name, and time bucket.

```sh
gpweasel connections $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "2026-06-03 18:06" connections $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

Requires:

```sql
ALTER SYSTEM SET log_connections = on;
ALTER SYSTEM SET log_disconnections = on;
SELECT pg_reload_conf();
```

### system

Show lifecycle and internal events, such as startup, shutdown, reload, checkpoints, background workers, extensions, replication, and WAL-related messages.

```sh
gpweasel system $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "2026-06-03 18:06" system $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

## Validation Scenario

Use a scratch database or schema. The example below avoids `DELETE` and `DROP`.

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

To generate a lock wait:

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
gpweasel -m "YYYY-MM-DD HH:MM" errors top $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "YYYY-MM-DD HH:MM" slow top --max 3 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "YYYY-MM-DD HH:MM" locks $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -m "YYYY-MM-DD HH:MM" connections $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

## Troubleshooting

### No connection rows

Check:

```sql
SHOW log_connections;
SHOW log_disconnections;
```

Both should be `on` for useful connection analysis.

### No lock rows

Check:

```sql
SHOW log_lock_waits;
SHOW deadlock_timeout;
```

Lock waits are logged only after `deadlock_timeout`.

### Too much output

Use a narrower time window or built-in paging:

```sh
gpweasel -m "2026-06-03 18:06" slow 1s $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -b 30m slow top --max 5 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -P 40 errors -l error $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

### `more` or `head` closes the pipe

`gpweasel` handles closed pipes quietly. These are supported:

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/gpdb-*.csv | more
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/gpdb-*.csv | head
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
