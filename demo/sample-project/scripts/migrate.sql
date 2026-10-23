CREATE TABLE IF NOT EXISTS jobs (
    id          BIGINT PRIMARY KEY,
    name        TEXT NOT NULL,
    priority    SMALLINT NOT NULL DEFAULT 5,
    status      TEXT NOT NULL CHECK (status IN ('queued', 'running', 'failed', 'done')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS jobs_status_idx ON jobs (status, priority DESC);

-- Jobs that have been running for more than an hour
SELECT id, name, now() - created_at AS age
FROM jobs
WHERE status = 'running' AND created_at < now() - interval '1 hour'
ORDER BY age DESC;
