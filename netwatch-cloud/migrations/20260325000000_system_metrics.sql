-- Add system metrics to snapshots
ALTER TABLE snapshots ADD COLUMN cpu_usage_pct DOUBLE PRECISION;
ALTER TABLE snapshots ADD COLUMN memory_total_bytes BIGINT;
ALTER TABLE snapshots ADD COLUMN memory_used_bytes BIGINT;
ALTER TABLE snapshots ADD COLUMN memory_available_bytes BIGINT;
ALTER TABLE snapshots ADD COLUMN load_avg_1m DOUBLE PRECISION;
ALTER TABLE snapshots ADD COLUMN load_avg_5m DOUBLE PRECISION;
ALTER TABLE snapshots ADD COLUMN load_avg_15m DOUBLE PRECISION;

-- Add hardware info to hosts
ALTER TABLE hosts ADD COLUMN cpu_model TEXT;
ALTER TABLE hosts ADD COLUMN cpu_cores INTEGER;
ALTER TABLE hosts ADD COLUMN memory_total_bytes BIGINT;
