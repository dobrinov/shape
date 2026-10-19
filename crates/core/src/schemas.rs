//! Database schemas (`schema` blocks): a section whose tables come from a
//! schema file, read by [schema-core](https://github.com/dobrinov/schema):
//! a Postgres `structure.sql`, MySQL or SQLite dump, or Rails `schema.rb`.
//!
//! ```text
//! schema db: Database [file: db/structure.sql, only: users orders*] {
//!   entity users: Users {
//!     id:    uuid [pk]
//!     email: citext [unique]
//!   }
//! }
//! ```
//!
//! The tables are ordinary `entity` blocks on the board. [`sync_ops`] writes
//! and updates them from the file as ops, like any other change, so arrows,
//! comments, perspectives, prompts and branch diffs work on them as on any
//! table, and a colour, a relationship's name or a comment on a table
//! survives the next update.
use crate::model::{slug, Column, Document};
use serde_json::{json, Value};

/// What updating a schema block from its file does.
#[derive(Debug, Default)]
pub struct Sync {
    /// The ops that bring the block up to date (none when it is).
    pub ops: Vec<Value>,
    /// Tables added, removed and changed (their names).
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    /// What the parser could not read.
    pub warnings: Vec<String>,
}

impl Sync {
    /// "2 tables added, 1 changed", or "up to date".
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        for (n, what) in [
            (self.added.len(), "added"),
            (self.changed.len(), "changed"),
            (self.removed.len(), "removed"),
        ] {
            if n > 0 && parts.is_empty() {
                parts.push(format!(
                    "{n} {} {what}",
                    if n == 1 { "table" } else { "tables" }
                ));
            } else if n > 0 {
                parts.push(format!("{n} {what}"));
            }
        }
        if parts.is_empty() {
            return "up to date".into();
        }
        parts.join(", ")
    }
}

/// A table as the board writes it: its id, name and columns.
struct Want {
    id: String,
    label: String,
    columns: Vec<(String, Column)>,
}

/// Does `text` match one of the space-separated globs (`*` any run, `?` one
/// character)?
fn matches(globs: &str, text: &str) -> bool {
    fn glob(p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') => glob(&p[1..], t) || (!t.is_empty() && glob(p, &t[1..])),
            Some(b'?') => !t.is_empty() && glob(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && glob(&p[1..], &t[1..]),
        }
    }
    globs
        .split_whitespace()
        .any(|g| glob(g.as_bytes(), text.as_bytes()))
}

/// A default the board can write as it is (`now()`, `0`, `'draft'`), or
/// none: one with brackets or separators (`ARRAY[]::text[]`) is left out.
fn plain_default(d: &Option<String>) -> Option<String> {
    d.as_ref()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty() && d.len() <= 40 && !d.contains(['[', ']', '{', '}', ',', ';']))
}

/// The tables in `src` (matching `only`), as the board writes them in block
/// `gid` of `doc`.
fn wanted(doc: &Document, gid: &str, src: &str, only: Option<&str>) -> (Vec<Want>, Vec<String>) {
    let schema = schema_core::parse(src);
    let tables: Vec<&schema_core::model::Table> = schema
        .tables
        .iter()
        .filter(|t| t.partition_of.is_none())
        .filter(|t| match only {
            Some(o) => matches(o, &t.name) || matches(o, &t.id()),
            // Rails' own bookkeeping is not part of the model
            None => !matches!(
                t.name.as_str(),
                "schema_migrations" | "ar_internal_metadata"
            ),
        })
        .collect();
    // with tables in more than one schema, those outside `public` keep its
    // name (`billing.invoices`)
    let qualify = tables.iter().any(|t| t.schema != tables[0].schema);
    let name = |t: &schema_core::model::Table| {
        if qualify && !matches!(t.schema.as_str(), "public" | "main" | "") {
            format!("{}.{}", t.schema, t.name)
        } else {
            t.name.clone()
        }
    };
    // a table's id: its name, or the block's and its name when the name is
    // taken by something else on the board
    let ids: Vec<(String, String)> = tables
        .iter()
        .map(|t| {
            let mut id = slug(&name(t).replace('.', "_"));
            let ours = doc
                .group(&id)
                .is_some_and(|g| g.entity && g.parent.as_deref() == Some(gid));
            if !ours && doc.id_taken(&id) {
                id = format!("{gid}-{id}");
            }
            (t.id(), id)
        })
        .collect();
    let id_of = |qualified: &str| {
        ids.iter()
            .find(|(q, _)| q == qualified)
            .map(|(_, id)| id.clone())
    };
    let mut out = Vec::new();
    for t in &tables {
        let tid = id_of(&t.id()).unwrap();
        let columns = t
            .columns
            .iter()
            .map(|c| {
                let pk = t.is_pk(&c.name);
                let single_pk = t.primary_key.as_ref().is_some_and(|p| p.columns.len() == 1);
                // a foreign key of one column, to a table on the board
                let fk = t
                    .foreign_keys
                    .iter()
                    .find(|f| f.columns.len() == 1 && f.columns[0] == c.name)
                    .and_then(|f| {
                        let to = id_of(&f.ref_table)?;
                        let target = schema.tables.iter().find(|r| r.id() == f.ref_table);
                        let to_pk = target.is_some_and(|r| {
                            r.primary_key
                                .as_ref()
                                .is_some_and(|p| p.columns == f.ref_columns)
                        });
                        Some(if to_pk || f.ref_columns.is_empty() {
                            to
                        } else {
                            format!("{to}.{}", f.ref_columns.join(""))
                        })
                    });
                let col = Column {
                    ty: c
                        .data_type
                        .strip_prefix("public.")
                        .unwrap_or(&c.data_type)
                        .to_string(),
                    pk,
                    unique: t.is_unique(&c.name) && !(pk && single_pk),
                    null: c.nullable && !pk,
                    fk,
                    default: plain_default(&c.default),
                    rel: None,
                };
                (c.name.clone(), col)
            })
            .collect();
        out.push(Want {
            id: tid,
            label: name(t),
            columns,
        });
    }
    (out, schema.warnings.clone())
}

/// Ops that bring schema block `gid` up to date with schema source `src`
/// (the file's text): tables and columns added, changed and removed. What
/// the board adds to a column (a colour, a detail, a relationship's name)
/// stays.
pub fn sync_ops(doc: &Document, gid: &str, src: &str) -> Result<Sync, String> {
    let g = doc
        .group(gid)
        .filter(|g| g.schema)
        .ok_or_else(|| format!("no schema block `{gid}`"))?;
    let (want, warnings) = wanted(doc, gid, src, g.only.as_deref());
    if want.is_empty() && g.only.is_none() {
        return Err(
            "no tables found: is it a Postgres, MySQL or SQLite schema, or a Rails schema.rb?"
                .into(),
        );
    }
    let mut s = Sync {
        warnings,
        ..Default::default()
    };
    let have: Vec<&str> = doc
        .groups
        .iter()
        .filter(|t| t.entity && t.parent.as_deref() == Some(gid))
        .map(|t| t.id.as_str())
        .collect();
    for t in &want {
        let label = if t.label == t.id {
            ""
        } else {
            t.label.as_str()
        };
        if !have.contains(&t.id.as_str()) {
            s.ops.push(json!({"op": "add_group", "id": t.id, "label": label, "parent": gid, "entity": true}));
            for (name, col) in &t.columns {
                s.ops.push(json!({"op": "add_node", "id": format!("{}.{name}", t.id), "group": t.id, "label": name, "column": col}));
            }
            s.added.push(t.label.clone());
            continue;
        }
        let mut changed = false;
        let cols = crate::entities::columns_of(doc, &t.id);
        for (name, col) in &t.columns {
            let id = format!("{}.{name}", t.id);
            match cols.iter().find(|n| n.id == id) {
                None => {
                    s.ops.push(json!({"op": "add_node", "id": id, "group": t.id, "label": name, "column": col}));
                    changed = true;
                }
                Some(n) => {
                    let old = n.column.clone().unwrap_or_default();
                    let new = Column {
                        rel: old.rel.clone(),
                        ..col.clone()
                    };
                    if old != new {
                        s.ops
                            .push(json!({"op": "update_node", "id": id, "column": new}));
                        changed = true;
                    }
                }
            }
        }
        for n in &cols {
            if !t
                .columns
                .iter()
                .any(|(name, _)| n.id == format!("{}.{name}", t.id))
            {
                s.ops.push(json!({"op": "remove", "id": n.id}));
                changed = true;
            }
        }
        if changed {
            s.changed.push(t.label.clone());
        }
    }
    for id in have {
        if !want.iter().any(|t| t.id == id) {
            s.ops
                .push(json!({"op": "remove", "id": id, "keep_members": false}));
            s.removed.push(
                doc.group(id)
                    .map(|g| g.display().to_string())
                    .unwrap_or_default(),
            );
        }
    }
    Ok(s)
}

/// Ops that add a schema block, with its tables from `src`: `file` is where
/// they come from (relative to the board), if anywhere.
pub fn add_ops(
    doc: &Document,
    label: &str,
    file: Option<&str>,
    only: Option<&str>,
    src: &str,
) -> Result<Sync, String> {
    let base = file
        .and_then(|f| f.rsplit('/').next())
        .and_then(|f| f.split('.').next())
        .filter(|b| !b.is_empty() && *b != "structure" && *b != "schema")
        .unwrap_or("db");
    let mut id = slug(base);
    let mut i = 2;
    while doc.id_taken(&id) {
        id = format!("{}-{i}", slug(base));
        i += 1;
    }
    let add = json!({"op": "add_group", "id": id, "label": label, "schema": true, "file": file.unwrap_or(""), "only": only.unwrap_or("")});
    let mut with = doc.clone();
    crate::ops::apply(
        &mut with,
        &crate::ops::parse_ops(&json!({"ops": [add.clone()]}).to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut s = sync_ops(&with, &id, src)?;
    s.ops.insert(0, add);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQL: &str = "CREATE TABLE public.teams (\n  id uuid NOT NULL,\n  name text NOT NULL\n);\n\
        CREATE TABLE public.users (\n  id uuid NOT NULL,\n  team_id uuid,\n  email text NOT NULL,\n  created_at timestamp DEFAULT now() NOT NULL\n);\n\
        ALTER TABLE ONLY public.teams ADD CONSTRAINT teams_pkey PRIMARY KEY (id);\n\
        ALTER TABLE ONLY public.users ADD CONSTRAINT users_pkey PRIMARY KEY (id);\n\
        CREATE UNIQUE INDEX index_users_on_email ON public.users USING btree (email);\n\
        ALTER TABLE ONLY public.users ADD CONSTRAINT fk_team FOREIGN KEY (team_id) REFERENCES public.teams(id);\n";

    fn apply(doc: &mut Document, ops: &[Value]) {
        let ops = crate::ops::parse_ops(&json!({ "ops": ops }).to_string()).unwrap();
        crate::ops::apply(doc, &ops).unwrap();
    }

    #[test]
    fn a_schema_file_becomes_tables_and_updates_in_place() {
        let mut doc = crate::dsl::parse("title: Data\n").unwrap();
        let s = add_ops(&doc, "Database", Some("db/structure.sql"), None, SQL).unwrap();
        assert_eq!(s.added, vec!["teams", "users"]);
        apply(&mut doc, &s.ops);
        let text = crate::dsl::print(&doc);
        assert!(text.contains("schema db: Database [file: db/structure.sql] {\n  entity teams {\n    id:   uuid [pk]\n    name: text\n  }\n"), "{text}");
        assert!(text.contains("    team_id:    uuid [null, fk: teams]\n    email:      text [unique]\n    created_at: timestamp [default: now()]\n"), "{text}");
        assert_eq!(crate::dsl::parse(&text).unwrap(), doc);
        // up to date: nothing to do
        assert!(sync_ops(&doc, "db", SQL).unwrap().ops.is_empty());

        // a relationship's name on the board stays; a new column, a dropped
        // table and a changed type come through
        apply(
            &mut doc,
            &[
                json!({"op": "update_node", "id": "users.team_id", "column": {"type": "uuid", "null": true, "fk": "teams", "rel": "member of"}}),
            ],
        );
        let next = SQL.replace(
            "  email text NOT NULL,\n",
            "  email citext NOT NULL,\n  bio text,\n",
        );
        let s = sync_ops(&doc, "db", &next).unwrap();
        assert_eq!(
            (s.changed.clone(), s.summary()),
            (vec!["users".to_string()], "1 table changed".to_string())
        );
        apply(&mut doc, &s.ops);
        let text = crate::dsl::print(&doc);
        assert!(
            text.contains("team_id:    uuid [null, fk: teams, rel: member of]"),
            "{text}"
        );
        assert!(
            text.contains("email:      citext [unique]")
                && text.contains("bio:        text [null]"),
            "{text}"
        );
        assert_eq!(crate::dsl::parse(&text).unwrap(), doc);

        let only_users = "CREATE TABLE public.users (\n  id uuid NOT NULL\n);\n";
        let s = sync_ops(&doc, "db", only_users).unwrap();
        assert_eq!(s.removed, vec!["teams"]);
        apply(&mut doc, &s.ops);
        assert!(doc.group("teams").is_none() && doc.node("teams.id").is_none());
    }

    #[test]
    fn only_some_tables_and_names_taken_elsewhere() {
        let mut doc = crate::dsl::parse("users: Users box\n").unwrap();
        let s = add_ops(&doc, "Database", None, Some("user*"), SQL).unwrap();
        apply(&mut doc, &s.ops);
        // `users` is a box already: the table is the block's
        assert!(doc.group("db-users").is_some_and(|g| g.label == "users"));
        assert!(doc.group("teams").is_none());
        // a key to a table left out is just a column
        assert_eq!(
            doc.node("db-users.team_id")
                .unwrap()
                .column
                .as_ref()
                .unwrap()
                .fk,
            None
        );
        let text = crate::dsl::print(&doc);
        assert!(
            text.contains("schema db: Database [only: user*] {\n  entity db-users: users {"),
            "{text}"
        );
        assert_eq!(crate::dsl::parse(&text).unwrap(), doc);
    }

    #[test]
    fn rails_schema_rb_reads_too() {
        let rb = "ActiveRecord::Schema[7.1].define(version: 2026_10_19_000000) do\n  create_table \"posts\", force: :cascade do |t|\n    t.string \"title\", null: false\n    t.bigint \"author_id\"\n  end\n  create_table \"authors\", force: :cascade do |t|\n    t.string \"name\"\n  end\n  add_foreign_key \"posts\", \"authors\"\nend\n";
        let doc = crate::dsl::parse("title: Blog\n").unwrap();
        let s = add_ops(&doc, "Blog", Some("db/schema.rb"), None, rb).unwrap();
        assert_eq!(s.added, vec!["posts", "authors"]);
        let mut doc = doc;
        apply(&mut doc, &s.ops);
        assert_eq!(
            doc.node("posts.author_id")
                .unwrap()
                .column
                .as_ref()
                .unwrap()
                .fk
                .as_deref(),
            Some("authors")
        );
    }
}
