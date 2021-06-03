INSERT INTO user_autosharing_settings (
    source_user_id,
    dest_user_id,
    dest_squad_id,
    can_share,
    can_clip
)
SELECT sra.user_id, NULL, sra.squad_id, TRUE, TRUE
FROM squad_role_assignments AS sra
ON CONFLICT DO NOTHING;

INSERT INTO squadov.user_autosharing_settings_games (
    id,
    game
)
SELECT uas.id, g.game
FROM user_autosharing_settings AS uas
CROSS JOIN UNNEST(ARRAY[0,1,2,3,4,5,6]) AS g(game);