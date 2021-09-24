DELETE FROM steam_user_links AS a USING (
    SELECT MIN(ctid) AS ctid, steam_id, user_id
    FROM steam_user_links
    GROUP BY steam_id, user_id HAVING COUNT(*) > 1
) AS b
WHERE a.steam_id = b.steam_id AND a.user_id = b.user_id AND a.ctid <> b.ctid;

ALTER TABLE steam_user_links
ADD CONSTRAINT steam_user_links_steam_id_user_id_key UNIQUE (steam_id, user_id);