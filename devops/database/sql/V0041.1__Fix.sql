
DELETE FROM wow_match_view_events AS t1
USING wow_match_view_events AS t2
WHERE t1.view_id = t2.view_id
    AND t1.log_line = t2.log_line
    AND t1.ctid < t2.ctid;
    
ALTER TABLE wow_match_view_events
ADD CONSTRAINT wow_match_view_events_view_id_log_line_key UNIQUE(view_id, log_line);