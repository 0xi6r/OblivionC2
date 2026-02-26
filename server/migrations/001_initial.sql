-- Campaigns table
CREATE TABLE IF NOT EXISTS campaigns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    operator_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('planning', 'active', 'paused', 'closing', 'archived')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    ended_at DATETIME,
    metadata TEXT -- JSON
);

-- Sessions (implants) table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    implant_id TEXT NOT NULL,
    hostname TEXT NOT NULL,
    username TEXT,
    os_version TEXT,
    process_id INTEGER,
    public_key BLOB,
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL CHECK (status IN ('active', 'idle', 'stale', 'terminated')),
    metadata TEXT, -- JSON
    encryption_key BLOB -- For session-specific data encryption
);

-- Tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL,
    payload BLOB,
    issued_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    issued_by TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'assigned', 'in_progress', 'completed', 'failed', 'timeout')),
    timeout_seconds INTEGER,
    result BLOB,
    error_message TEXT,
    completed_at DATETIME,
    execution_time_ms INTEGER
);

-- Operator audit log
CREATE TABLE IF NOT EXISTS operator_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    operator_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target_session_id TEXT,
    target_campaign_id TEXT,
    details TEXT, -- JSON
    ip_address TEXT,
    success BOOLEAN NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_campaign ON sessions(campaign_id);
CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks(session_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON operator_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_logs_operator ON operator_logs(operator_id);

-- Triggers for automatic timestamp updates
CREATE TRIGGER IF NOT EXISTS update_session_last_seen
AFTER UPDATE ON sessions
FOR EACH ROW
BEGIN
    UPDATE sessions SET last_seen = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;