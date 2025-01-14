TRUNCATE processing_runs;

ALTER TABLE processing_runs
ADD CONSTRAINT processing_runs_pkey PRIMARY KEY (processor_id);
