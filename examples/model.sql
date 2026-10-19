-- Shape's board model, as if it were a database: the example board's
-- "Data & flows" canvas reads it (`schema model: … [file: model.sql]`).
CREATE TABLE boards (
  path  text PRIMARY KEY,
  title text NOT NULL,
  theme text
);

CREATE TABLE nodes (
  id     text PRIMARY KEY,
  board  text NOT NULL REFERENCES boards (path),
  parent text,
  label  text NOT NULL DEFAULT ''
);

CREATE TABLE edges (
  id     text PRIMARY KEY,
  "from" text NOT NULL REFERENCES nodes (id),
  "to"   text NOT NULL REFERENCES nodes (id)
);
