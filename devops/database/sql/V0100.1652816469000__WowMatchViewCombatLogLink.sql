ALTER TABLE wow_match_view
ADD COLUMN combat_log_partition_id VARCHAR REFERENCES combat_logs(partition_id) ON DELETE SET NULL;