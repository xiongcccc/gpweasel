# gpweasel

`gpweasel` 是一个面向 DBA 和运维人员的 Greenplum Database / YMatrix 日志解析 CLI 工具。它参考了 PostgreSQL 生态中 `pgweasel` 的思路，但针对 Greenplum/YMatrix 的 `gpdb-*.csv` 日志格式做了适配。

它的目标不是生成 HTML 报告，而是在数据库服务器上直接对现有日志做快速排查：

* 最近有哪些高频错误？
* 错误集中发生在哪些时间段？
* 哪些 SQL 最慢？
* 是否出现锁等待、锁超时或死锁？
* 连接主要来自哪些 host、database、user？
* 哪些时间窗口日志量突然升高？
* 当前一批日志的总体健康概览是什么？
* 是否发生过启动、关闭、reload、checkpoint、后台进程、扩展、复制等系统事件？

## 支持的日志格式

主要支持 Greenplum/YMatrix CSV server log，例如：

```text
gpdb-2026-06-03_095933.csv
```

常见日志目录：

```sh
$MASTER_DATA_DIRECTORY/log/gpdb-*.csv
$MASTER_DATA_DIRECTORY/pg_log/gpdb-*.csv
```

`gpweasel` 当前会使用这些 Greenplum CSV 字段：

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

为了兼容原始 `pgweasel` 的使用方式，PostgreSQL CSV/plain log 也保留了部分支持。

另外，部分 Greenplum/YMatrix 启动日志文件可能叫 `startup.log`，但内容仍然是 CSV 风格字段。`gpweasel` 会在解析 severity、message、host、user、database 时自动做 CSV fallback，因此可以直接使用较宽的日志通配符：

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
gpweasel connections $MASTER_DATA_DIRECTORY/log/*
```

## 编译安装

在数据库服务器上安装 Rust 后编译：

```sh
git clone git@github.com:xiongcccc/gpweasel.git
cd gpweasel
cargo build --release
```

生成的二进制文件位于：

```sh
target/release/gpweasel
```

可选安装到系统路径：

```sh
sudo install -m 0755 target/release/gpweasel /usr/local/bin/gpweasel
gpweasel --help
```

如果服务器访问 GitHub SSH 22 端口受限，可以使用 443 端口：

```sh
git clone ssh://git@ssh.github.com:443/xiongcccc/gpweasel.git
```

## 推荐数据库日志参数

为了完整验证和获得更有价值的生产诊断信息，建议开启这些参数：

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

说明：

* `logging_collector` 如果之前是 `off`，通常需要重启数据库才会生效。
* `log_min_duration_statement = 0` 适合短时间测试，但生产环境会非常吵，不建议长期打开。
* `log_connections` 和 `log_disconnections` 是 `connections` 命令有意义输出的前提。
* `log_lock_waits` 配合合适的 `deadlock_timeout`，才能看到锁等待类日志。

## 命令结构

基本格式：

```sh
gpweasel [全局参数] <命令> [命令参数] <日志文件...>
```

示例：

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
gpweasel errors top $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -b 30m slow top --max 5 $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
gpweasel -P 40 errors -l error $MASTER_DATA_DIRECTORY/log/gpdb-*.csv
```

## 全局参数

全局参数必须写在子命令前面：

```sh
gpweasel -b 30m stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-03 18:06" errors top $MASTER_DATA_DIRECTORY/log/*
```

常用全局参数：

```sh
-b, --begin <BEGIN>      起始时间过滤
-e, --end <END>          结束时间过滤
-m, --mask <MASK>        文本包含过滤，常用于按时间前缀过滤
-P, --page-size <LINES>  交互式分页，每 N 行暂停一次
-d, --debug              输出 debug 信息
```

### 全局 `-b, --begin`

`-b` 在子命令前表示开始时间，只分析这个时间之后的日志。

适用场景：

* 只看最近一段时间的故障。
* 已知故障从某个时间点开始，排除更早日志。
* 避免大日志全量扫描时输出过多。

示例：

```sh
gpweasel -b 30m stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -b 2h errors top $MASTER_DATA_DIRECTORY/log/*
gpweasel -b today slow top --max 10 $MASTER_DATA_DIRECTORY/log/*
gpweasel -b "2026-06-03 18:00:00" locks $MASTER_DATA_DIRECTORY/log/*
```

### 全局 `-m, --mask`

`-m` 在子命令前表示 mask 过滤，本质是文本包含匹配。DBA 最常用的方式是按日志时间前缀过滤到某一分钟或某一小时。

适用场景：

* 已经知道问题发生在某一分钟，例如 `2026-06-03 18:06`。
* 想快速查看某个关键字相关的记录。
* 先用 `peaks` 找到高峰时间，再用 `-m` 钻取该时间段。

示例：

```sh
gpweasel -m "2026-06-03 18:06" errors $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-03 18" stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "deadlock" errors -l log $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "connection authorized" connections $MASTER_DATA_DIRECTORY/log/*
```

### 全局 `-P, --page-size`

`-P` 用于内置分页，只在交互式终端生效。它适合服务器上直接查看大量输出时使用。

```sh
gpweasel -P 40 errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel -P 20 slow 500ms $MASTER_DATA_DIRECTORY/log/*
```

如果输出被重定向或接入管道，`-P` 不会暂停。`more`、`head` 等命令提前关闭管道时，`gpweasel` 会静默退出，避免 broken pipe panic。

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/* | more
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/* | head
```

## `-b` 和 `-m` 的复用说明

`-b` 和 `-m` 在不同位置含义不同，这是 clap CLI 中常见的局部参数作用域设计。

### 写在子命令前：全局过滤

```sh
gpweasel -b 2h stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-04 10:00" errors top $MASTER_DATA_DIRECTORY/log/*
```

含义：

```text
-b / --begin  从哪个时间开始分析
-m / --mask   只分析包含某个文本或时间前缀的记录
```

### 写在子命令后：该子命令自己的参数

```sh
gpweasel peaks -b 10m -m 5 $MASTER_DATA_DIRECTORY/log/*
gpweasel errors hist -b 1h $MASTER_DATA_DIRECTORY/log/*
gpweasel errors top -m 20 $MASTER_DATA_DIRECTORY/log/*
gpweasel slow top -m 10 $MASTER_DATA_DIRECTORY/log/*
```

含义：

```text
peaks -b / --bucket       峰值统计的时间桶宽度
peaks -m / --max          最多显示多少个高峰时间桶
errors hist -b / --bucket 错误直方图的时间桶宽度
errors top -m / --max     最多显示多少条高频错误
slow top -m / --max       最多显示多少条慢 SQL
```

### 推荐写法

为了避免混淆，建议长参数和短参数搭配使用：

```sh
# 最近 2 小时内，找日志量最高的 5 个 10 分钟窗口
gpweasel --begin 2h peaks --bucket 10m --max 5 $MASTER_DATA_DIRECTORY/log/*

# 某一分钟内，找最高频错误
gpweasel --mask "2026-06-03 18:06" errors top --max 20 $MASTER_DATA_DIRECTORY/log/*

# 今天的错误直方图，每 30 分钟一个桶
gpweasel --begin today errors hist --bucket 30m $MASTER_DATA_DIRECTORY/log/*
```

## 命令说明

### stats

输出一批日志的总体摘要，适合作为第一眼巡检命令。

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -b 2h stats $MASTER_DATA_DIRECTORY/log/*
gpweasel -m "2026-06-03 18" stats $MASTER_DATA_DIRECTORY/log/*
```

输出内容包括：

* 总日志记录数。
* severity 分布。
* duration 日志数量和最大 duration。
* connection received / authorized / failed 计数。
* lock 相关事件计数。
* 缺失 user/database/host 字段的记录数。
* Top users / databases / hosts。

适用场景：

* 拿到一批日志后先判断整体情况。
* 快速看错误、慢 SQL、连接、锁等待是否异常。
* 先找出高风险方向，再切换到 `errors`、`slow`、`locks` 等命令深入分析。

### peaks

按时间桶统计日志量，显示日志最密集的时间段。

```sh
gpweasel peaks $MASTER_DATA_DIRECTORY/log/*
gpweasel peaks --bucket 1m --max 10 $MASTER_DATA_DIRECTORY/log/*
gpweasel --begin 2h peaks --bucket 5m --max 20 $MASTER_DATA_DIRECTORY/log/*
```

参数：

```sh
-b, --bucket <INTERVAL>  时间桶宽度，例如 10s、1m、10m、1h，默认 10m
-m, --max <MAX>         最多显示多少个高峰桶，默认 20
```

适用场景：

* 不知道故障具体发生时间，先找日志量突增窗口。
* 与 `errors`、`slow`、`locks` 联动，先找峰值，再按时间过滤。
* 观察巡检任务、连接风暴、错误风暴、锁等待是否集中爆发。

常见排查组合：

```sh
# 先找最近 6 小时内最忙的 10 个一分钟窗口
gpweasel --begin 6h peaks --bucket 1m --max 10 $MASTER_DATA_DIRECTORY/log/*

# 假设发现 2026-06-04 10:00 最忙，再钻取这一分钟的错误
gpweasel --mask "2026-06-04 10:00" errors -l warning $MASTER_DATA_DIRECTORY/log/*

# 看这一分钟慢 SQL
gpweasel --mask "2026-06-04 10:00" slow 500ms $MASTER_DATA_DIRECTORY/log/*
```

### errors

列出错误日志。默认最低级别是 `error`，因此会显示 `ERROR`、`FATAL`、`PANIC`。

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/*
gpweasel errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" errors -l error $MASTER_DATA_DIRECTORY/log/*
```

### errors top

统计最高频错误消息。

```sh
gpweasel errors top $MASTER_DATA_DIRECTORY/log/*
gpweasel errors top --max 20 $MASTER_DATA_DIRECTORY/log/*
gpweasel --begin 2h errors top --max 10 $MASTER_DATA_DIRECTORY/log/*
```

适用场景：

* 快速识别重复错误。
* 判断错误是否由同一个 SQL、对象、用户或连接问题触发。
* 比直接翻日志更适合处理大量重复报错。

### errors hist

按时间桶显示错误直方图。

```sh
gpweasel errors hist $MASTER_DATA_DIRECTORY/log/*
gpweasel errors hist --bucket 30m -l error $MASTER_DATA_DIRECTORY/log/*
gpweasel --begin today errors hist --bucket 1h $MASTER_DATA_DIRECTORY/log/*
```

适用场景：

* 判断错误是否集中在某个时间段。
* 观察错误是否持续发生。
* 与业务变更、扩容、任务调度时间对齐。

### slow

列出超过阈值的 SQL。它依赖日志 message 中包含 `duration:`，通常需要配置 `log_min_duration_statement`。

```sh
gpweasel slow 500ms $MASTER_DATA_DIRECTORY/log/*
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" slow 1s $MASTER_DATA_DIRECTORY/log/*
```

适用场景：

* 找出某个时间窗口内的慢 SQL 明细。
* 排查业务响应慢、资源升高、锁等待后的 SQL 表现。
* 和 `--begin`、`--mask` 配合减少输出量。

### slow top

显示最慢的 SQL，默认显示 10 条。

```sh
gpweasel slow top $MASTER_DATA_DIRECTORY/log/*
gpweasel slow top --max 5 $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" slow top --max 3 $MASTER_DATA_DIRECTORY/log/*
```

### locks

显示锁等待、锁超时、死锁、恢复冲突等相关日志。

```sh
gpweasel locks $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" locks $MASTER_DATA_DIRECTORY/log/*
```

典型 Greenplum 锁等待日志可能包含：

```text
process 2544911 still waiting for AccessShareLock ...
```

前提参数：

```sql
ALTER SYSTEM SET log_lock_waits = on;
ALTER SYSTEM SET deadlock_timeout = '1s';
SELECT pg_reload_conf();
```

### connections

汇总连接尝试和认证成功连接，按 host、database、user、application name、时间桶统计。

```sh
gpweasel connections $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" connections $MASTER_DATA_DIRECTORY/log/*
```

前提参数：

```sql
ALTER SYSTEM SET log_connections = on;
ALTER SYSTEM SET log_disconnections = on;
SELECT pg_reload_conf();
```

适用场景：

* 观察连接风暴。
* 判断连接主要来自本机、应用服务器还是某个异常客户端。
* 统计连接主要打到哪些数据库和用户。

### system

显示系统生命周期和内部事件，例如启动、关闭、reload、checkpoint、后台进程、扩展、复制、WAL 等。

```sh
gpweasel system $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" system $MASTER_DATA_DIRECTORY/log/*
```

适用场景：

* 排查实例启动、停止、reload。
* 查看 checkpoint、后台 worker、复制、扩展等内部事件。
* 将系统事件与错误、慢 SQL、连接峰值进行时间关联。

## DBA 常用排查流程

### 1. 先看整体摘要

```sh
gpweasel stats $MASTER_DATA_DIRECTORY/log/*
```

关注：

* `Severity counts` 中 `error/fatal/panic` 是否异常。
* `duration events` 和 `max duration` 是否异常。
* `lock events` 是否大于 0。
* Top hosts/databases/users 是否符合预期。

### 2. 找日志高峰时间

```sh
gpweasel peaks --bucket 1m --max 10 $MASTER_DATA_DIRECTORY/log/*
```

如果发现某一分钟日志量异常，例如 `2026-06-04 10:00:00`，继续钻取：

```sh
gpweasel --mask "2026-06-04 10:00" errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-04 10:00" slow 500ms $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-04 10:00" locks $MASTER_DATA_DIRECTORY/log/*
```

### 3. 看高频错误

```sh
gpweasel errors top --max 20 $MASTER_DATA_DIRECTORY/log/*
```

如果日志量很大：

```sh
gpweasel --begin 2h errors top --max 20 $MASTER_DATA_DIRECTORY/log/*
```

### 4. 看慢 SQL

```sh
gpweasel slow top --max 10 $MASTER_DATA_DIRECTORY/log/*
```

如果要看某个阈值以上的明细：

```sh
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/*
```

### 5. 看锁等待

```sh
gpweasel locks $MASTER_DATA_DIRECTORY/log/*
```

如果锁等待很多，建议结合 peaks 找集中时间，再用 mask 缩小：

```sh
gpweasel --mask "2026-06-04 10:00" locks $MASTER_DATA_DIRECTORY/log/*
```

### 6. 看连接来源

```sh
gpweasel connections $MASTER_DATA_DIRECTORY/log/*
```

关注：

* 是否有异常 host。
* 是否某个 database/user 连接数暴涨。
* 时间桶中是否有突增。

## 验证场景

可以在测试库或测试 schema 中执行下面的 SQL。示例避免使用 `DELETE` 和 `DROP`。

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

构造锁等待：

Session 1：

```sql
BEGIN;
LOCK TABLE public.gpweasel_probe IN ACCESS EXCLUSIVE MODE;
SELECT pg_sleep(6);
COMMIT;
```

Session 2：

```sql
SET lock_timeout = '3s';
SELECT count(*) FROM public.gpweasel_probe;
```

验证命令：

```sh
gpweasel --mask "YYYY-MM-DD HH:MM" errors top $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" slow top --max 3 $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" locks $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" connections $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" peaks --bucket 1m --max 5 $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "YYYY-MM-DD HH:MM" stats $MASTER_DATA_DIRECTORY/log/*
```

## 常见问题

### connections 没有输出

检查：

```sql
SHOW log_connections;
SHOW log_disconnections;
```

建议两者都为 `on`。

### locks 没有输出

检查：

```sql
SHOW log_lock_waits;
SHOW deadlock_timeout;
```

锁等待只有超过 `deadlock_timeout` 后才会记录。

### slow 没有输出

检查：

```sql
SHOW log_min_duration_statement;
```

如果值为 `-1`，表示不记录 statement duration。短时间验证可以设置为 `0` 或较小值，生产环境建议设置为合理阈值，例如 `500ms`、`1s` 或按业务 SLA 调整。

### 输出太多

优先缩小时间窗口：

```sh
gpweasel --begin 30m errors -l warning $MASTER_DATA_DIRECTORY/log/*
gpweasel --mask "2026-06-03 18:06" slow 1s $MASTER_DATA_DIRECTORY/log/*
```

或者使用内置分页：

```sh
gpweasel -P 40 errors -l error $MASTER_DATA_DIRECTORY/log/*
```

### `more` 或 `head` 提示 broken pipe

当前版本会处理关闭的 pipe，支持：

```sh
gpweasel errors $MASTER_DATA_DIRECTORY/log/* | more
gpweasel slow 1s $MASTER_DATA_DIRECTORY/log/* | head
```

如果是在旧版本中看到 broken pipe，建议拉取最新代码后重新编译：

```sh
git pull
cargo build --release
```

## 开发

本地检查：

```sh
cargo build --release
cargo test
cargo fmt --all -- --check
```

## License

本项目沿用上游 Apache License。
