CREATE VIEW view_clip_react_count (
    clip_uuid,
    count
)
AS
SELECT
    cr.clip_uuid,
    COUNT(cr.user_id)
FROM clip_reacts AS cr
GROUP BY cr.clip_uuid;

CREATE VIEW view_clip_comment_count (
    clip_uuid,
    count
)
AS
SELECT
    cc.clip_uuid,
    COUNT(cc.user_id)
FROM clip_comments AS cc
GROUP BY cc.clip_uuid;