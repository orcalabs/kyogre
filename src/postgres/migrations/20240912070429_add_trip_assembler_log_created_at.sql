TRUNCATE trip_assembler_logs;

ALTER TABLE trip_assembler_logs
ADD COLUMN created_at timestamptz NOT NULL DEFAULT NOW();
