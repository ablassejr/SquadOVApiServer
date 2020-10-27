CREATE OR REPLACE VIEW view_aimlab_spar_data (
    match_uuid,
    kill,
    ttk,
    acc
)
AS
SELECT
    match_uuid,
    (raw_data->'killTotal')::double precision AS kill,
    1000.0 / (raw_data->'killsPerSec')::double precision AS ttk,
    (raw_data->'accTotal')::double precision as acc
FROM squadov.aimlab_tasks;