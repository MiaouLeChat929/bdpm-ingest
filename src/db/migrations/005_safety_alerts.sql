-- Migration 005: Add safety_alerts table for CIS_InfoImportantes data.
-- Safety alerts are imported from the on-demand generated CIS_InfoImportantes.txt file.
-- Each row: CIS code, start/end dates, warning text (HTML stripped), source URL.

CREATE TABLE safety_alerts (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    cis                 TEXT NOT NULL,
    start_date          TEXT,                               -- ISO-8601
    end_date            TEXT,                               -- ISO-8601
    message_plain       TEXT,                               -- HTML-stripped warning text
    source_url          TEXT,                               -- ANSM link (nullable)
    imported_at         DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_safety_alerts_cis ON safety_alerts(cis);
CREATE INDEX idx_safety_alerts_dates ON safety_alerts(start_date, end_date);
