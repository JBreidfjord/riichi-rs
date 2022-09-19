PRAGMA encoding = 'UTF-8';
PRAGMA defer_foreign_keys = true;
BEGIN TRANSACTION;


CREATE TABLE IF NOT EXISTS archives
(
    filename     TEXT PRIMARY KEY
        NOT NULL,
    url          TEXT,
    date_time_bin DATETIME
);
CREATE INDEX IF NOT EXISTS index_archives_datetime_bin ON archives (date_time_bin ASC);

CREATE TABLE IF NOT EXISTS games
(
    log_id           TEXT PRIMARY KEY
        NOT NULL,
    start_date_time  TEXT,
    duration_minutes INTEGER,
    rule_id          INTEGER,
    lobby_id         TEXT,
    game_json        BLOB,
    game_json_sha256 BLOB,
    last_checked     TEXT
);
CREATE INDEX IF NOT EXISTS index_games_start_datetime ON games (start_date_time ASC);


COMMIT TRANSACTION;
