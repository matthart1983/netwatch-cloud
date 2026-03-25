-- Add swap metrics to snapshots
ALTER TABLE snapshots ADD COLUMN swap_total_bytes BIGINT;
ALTER TABLE snapshots ADD COLUMN swap_used_bytes BIGINT;

-- Add disk I/O to snapshots  
ALTER TABLE snapshots ADD COLUMN disk_read_bytes BIGINT;
ALTER TABLE snapshots ADD COLUMN disk_write_bytes BIGINT;

-- Add TCP connection states to snapshots
ALTER TABLE snapshots ADD COLUMN tcp_time_wait INTEGER;
ALTER TABLE snapshots ADD COLUMN tcp_close_wait INTEGER;

-- Disk usage per snapshot (separate table, like interface_metrics)
CREATE TABLE disk_metrics (
    id              BIGSERIAL PRIMARY KEY,
    snapshot_id     BIGINT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    host_id         UUID NOT NULL,
    time            TIMESTAMPTZ NOT NULL,
    mount_point     TEXT NOT NULL,
    device          TEXT NOT NULL,
    total_bytes     BIGINT NOT NULL,
    used_bytes      BIGINT NOT NULL,
    available_bytes BIGINT NOT NULL,
    usage_pct       DOUBLE PRECISION NOT NULL
);

CREATE INDEX idx_disk_host_time ON disk_metrics(host_id, time DESC);
CREATE INDEX idx_disk_snapshot ON disk_metrics(snapshot_id);
