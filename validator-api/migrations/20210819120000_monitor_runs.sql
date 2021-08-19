-- keeping track of all monitor runs that have happened will help to
-- solve an issue of mixnode being online only for a single check and yet being assigned 100% uptime
CREATE TABLE monitor_run
(
    id        INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL
)