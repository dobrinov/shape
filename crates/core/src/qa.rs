//! QA: how the board says things work, and the test scenarios for it.
//!
//! A table with an `expected` column is a table of test scenarios: a row
//! per scenario, its first cell the scenario, then (by column name) its
//! steps, the expected result, what it covers and its status (`to run`,
//! `pass`, `fail`, `blocked`). [`scenarios`] finds them on the whole board,
//! [`parts`] lists what there is to test (sequences, journeys, screens) and
//! [`covered`] says whether a scenario covers a part. [`draft_prompt`] asks a
//! model for new scenarios as a table, and [`draft_ops`] turns its reply
//! into ops: a canvas of test scenarios next to what they test, or new rows
//! in the table already there.
use crate::dashboard::{docs, At};
use crate::model::{slug, Document, Group, Node};

/// The columns of a table of test scenarios, as drafted.
pub const COLUMNS: [&str; 5] = ["scenario", "steps", "expected", "covers", "status"];

/// A table of test scenarios: it has an `expected` column.
pub fn is_tests(g: &Group) -> bool {
    g.table
        && g.columns
            .iter()
            .any(|c| c.trim().to_lowercase().starts_with("expected"))
}

/// How a scenario last went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Fail,
    Blocked,
    ToRun,
    Pass,
}

impl Outcome {
    pub fn word(self) -> &'static str {
        match self {
            Outcome::Fail => "fail",
            Outcome::Blocked => "blocked",
            Outcome::ToRun => "to run",
            Outcome::Pass => "pass",
        }
    }
    /// The row's tint on the board and in summaries.
    pub fn color(self) -> Option<&'static str> {
        match self {
            Outcome::Fail => Some("red"),
            Outcome::Blocked => Some("orange"),
            Outcome::ToRun => None,
            Outcome::Pass => Some("green"),
        }
    }
    pub fn from_word(s: &str) -> Outcome {
        let s = s.trim().to_lowercase();
        if s.starts_with("pass") || s == "ok" || s == "done" || s.starts_with('✓') {
            Outcome::Pass
        } else if s.starts_with("fail") || s.starts_with('✗') || s == "broken" {
            Outcome::Fail
        } else if s.starts_with("block") {
            Outcome::Blocked
        } else {
            Outcome::ToRun
        }
    }
}

/// A row's cell under the column named like one of `names` ("" if none).
pub fn cell<'a>(g: &Group, n: &'a Node, names: &[&str]) -> &'a str {
    let at = g.columns.iter().position(|c| {
        let c = c.trim().to_lowercase();
        names.iter().any(|x| c.starts_with(x))
    });
    match at {
        Some(0) => n.label.as_str(),
        Some(i) => n.cells.get(i - 1).map(String::as_str).unwrap_or(""),
        None => "",
    }
}

/// A test scenario on the board.
#[derive(Debug, Clone)]
pub struct Scenario {
    pub at: At,
    pub label: String,
    pub covers: String,
    pub outcome: Outcome,
}

/// Every test scenario on the board, its canvases included.
pub fn scenarios(doc: &Document) -> Vec<Scenario> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for g in d.groups.iter().filter(|g| is_tests(g)) {
            for n in d
                .nodes
                .iter()
                .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
            {
                out.push(Scenario {
                    at: At {
                        path: path.clone(),
                        id: n.id.clone(),
                    },
                    label: n.label.clone(),
                    covers: cell(g, n, &["cover", "area", "feature"]).to_string(),
                    outcome: Outcome::from_word(cell(g, n, &["status", "result"])),
                });
            }
        }
    }
    out
}

/// Something to test: a sequence, a journey (swimlanes) or a screen.
#[derive(Debug, Clone)]
pub struct Part {
    pub at: At,
    pub label: String,
    pub kind: &'static str,
}

/// What there is to test on the board: its sequences, journeys and screens.
pub fn parts(doc: &Document) -> Vec<Part> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for g in &d.groups {
            let kind = if g.sequence && g.parent.is_none() {
                "sequence"
            } else if g.lanes && !g.kanban && g.parent.is_none() {
                "journey"
            } else if g.screen.is_some() {
                "screen"
            } else {
                continue;
            };
            out.push(Part {
                at: At {
                    path: path.clone(),
                    id: g.id.clone(),
                },
                label: g.display().to_string(),
                kind,
            });
        }
    }
    out
}

/// Whether a scenario covers `part`: its covers cell names the part (its
/// title or id), or an arrow joins the two.
pub fn covered(doc: &Document, part: &Part, all: &[Scenario]) -> bool {
    let label = part.label.trim().to_lowercase();
    all.iter().any(|s| {
        let c = s.covers.to_lowercase();
        let named = (!label.is_empty() && c.contains(&label)) || c.contains(&part.at.id);
        let joined = s.at.path == part.at.path
            && doc.at(&s.at.path).is_some_and(|d| {
                d.edges.iter().any(|e| {
                    (e.from == s.at.id && e.to == part.at.id)
                        || (e.to == s.at.id && e.from == part.at.id)
                })
            });
        named || joined
    })
}

/// Where the scenarios for `doc` are: the canvas path below it (empty: in
/// `doc` itself) and the table's id.
fn tests_table(doc: &Document) -> Option<(Vec<String>, String)> {
    docs(doc).into_iter().find_map(|(path, d)| {
        d.groups
            .iter()
            .find(|g| is_tests(g))
            .map(|g| (path, g.id.clone()))
    })
}

/// A prompt asking a model to draft test scenarios for `doc` (the open
/// canvas, with the canvases inside it): from the flows, screens, rules,
/// data and risks on it, new scenarios only. The reply is a table (see
/// [`draft_ops`]).
pub fn draft_prompt(doc: &Document) -> String {
    let title = doc.title.as_deref().unwrap_or("Untitled board");
    let mut s = format!("# Draft test scenarios for \"{title}\"\n\n");
    s.push_str("You are the QA engineer on this project. The board below says how the product works: its flows and journeys, screens, data, the rules and decisions behind them, and what could go wrong. It is written in the `shape` language (`id: Label [attrs]` is a box, `a -> b: label` an arrow, `group id: Label { … }` a section; `sequence`, `lanes`, `screen`, `table`, `entity`, `adr`, `risks` and `todo` blocks hold their items one per line). Write the test scenarios a tester would run to check it works as the board says.\n\n");
    if let Some(d) = &doc.description {
        s.push_str(&format!("About it: {d}\n\n"));
    }
    s.push_str(&format!(
        "## The board\n\n```shape\n{}```\n\n",
        crate::dsl::print_lean(doc)
    ));
    let found = tests_table(doc);
    for (path, d) in docs(doc).into_iter().skip(1) {
        // the scenarios' own canvas is listed below, not shown
        if found.as_ref().is_some_and(|(p, _)| p == &path) && d.groups.iter().all(|g| g.table) {
            continue;
        }
        s.push_str(&format!(
            "## Inside it: canvas `{}` (“{}”)\n\n```shape\n{}```\n\n",
            path.join("/"),
            d.title.as_deref().unwrap_or(""),
            crate::dsl::print_lean(d)
        ));
    }
    let what = parts(doc);
    if !what.is_empty() {
        let all = scenarios(doc);
        s.push_str("## What there is to test\n\n");
        for p in &what {
            s.push_str(&format!(
                "- {} ({}){}\n",
                p.label,
                p.kind,
                if covered(doc, p, &all) {
                    ""
                } else {
                    ": no scenario yet"
                }
            ));
        }
        s.push('\n');
    }
    let have = scenarios(doc);
    if !have.is_empty() {
        s.push_str(&format!(
            "## Scenarios already written ({}): do not repeat them\n\n",
            have.len()
        ));
        for x in &have {
            s.push_str(&format!("- {}\n", x.label));
        }
        s.push('\n');
    }
    s.push_str("## How to respond\n\nReply with a single ```shape code block holding one table, new scenarios only, for example:\n\n```shape\ntable scenarios: Test scenarios {\n  scenario | steps | expected | covers | status\n  Pay with a saved card | 1. Open checkout\\n2. Pick the saved card\\n3. Pay | The order is paid and confirmed, and a receipt is emailed | Checkout | to run\n  Card declined | 1. Pay with a card the bank declines | The order stays unpaid; the page says why and offers another card | Checkout | to run\n}\n```\n\nRules:\n- One row per scenario; the first cell names it in a few words.\n- `steps`: what the tester does, numbered, separated by `\\n`. `expected`: what they should see happen, observable and specific. `covers`: the part it tests, named as in the list above (or the element's label). `status`: `to run`.\n- Cover the main paths first, then edge cases (empty, limits, duplicates, timeouts, going back), errors and permissions, the rules in decision records and what the risks say could go wrong. Parts with no scenario yet come first.\n- 8 to 20 scenarios, the most important first. Write `\\|` for a `|` inside a cell.\n- Only what the board says or clearly implies; no explanation outside the code block.\n");
    s
}

/// The ops that put a model's [`draft_prompt`] reply on `doc` (the open
/// canvas): its scenarios as new rows of the table of test scenarios
/// already there, or, with none, in a new canvas "Test scenarios" that a
/// box on `doc` opens. Returns `{"ops": […]}` (JSON) or why it could not.
pub fn draft_ops(doc: &Document, reply: &str) -> Result<String, String> {
    let body = fenced(reply).unwrap_or(reply);
    let got = crate::dsl::parse(body).map_err(|errs| {
        errs.iter()
            .map(|e| format!("line {}: {}", e.line, e.message))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let Some(t) = got.groups.iter().find(|g| g.table) else {
        return Err("the reply has no table of scenarios in a ```shape block".into());
    };
    let rows: Vec<Vec<String>> = got
        .nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(t.id.as_str()))
        .map(|n| {
            COLUMNS
                .iter()
                .map(|c| {
                    let v = cell(t, n, &[c]).trim().to_string();
                    if *c == "status" && v.is_empty() {
                        "to run".into()
                    } else {
                        v
                    }
                })
                .collect()
        })
        .filter(|r: &Vec<String>| !r[0].is_empty())
        .collect();
    if rows.is_empty() {
        return Err("the table in the reply has no scenarios".into());
    }
    let ops: Vec<serde_json::Value> = match tests_table(doc) {
        Some((path, tid)) => {
            let d = doc.at(&path).ok_or("the scenarios' canvas is gone")?;
            let g = d.group(&tid).ok_or("the scenarios' table is gone")?;
            let mut taken: Vec<String> = d.nodes.iter().map(|n| n.id.clone()).collect();
            let known: Vec<String> = d
                .nodes
                .iter()
                .filter(|n| n.group.as_deref() == Some(tid.as_str()))
                .map(|n| n.label.trim().to_lowercase())
                .collect();
            let adds: Vec<serde_json::Value> = rows
                .iter()
                .filter(|r| !known.contains(&r[0].to_lowercase()))
                .map(|r| {
                    // in the table's own columns, by name
                    let cells: Vec<String> = g
                        .columns
                        .iter()
                        .skip(1)
                        .map(|c| {
                            let c = c.trim().to_lowercase();
                            COLUMNS
                                .iter()
                                .position(|x| c.starts_with(x) || (*x == "status" && c.starts_with("result")))
                                .map(|i| r[i].clone())
                                .unwrap_or_default()
                        })
                        .collect();
                    let id = fresh(&format!("{tid}.{}", slug(&r[0])), &mut taken);
                    serde_json::json!({"op": "add_node", "id": id, "label": r[0], "group": tid, "cells": cells})
                })
                .collect();
            if adds.is_empty() {
                return Err("the reply only repeats scenarios already written".into());
            }
            if path.is_empty() {
                adds
            } else {
                vec![serde_json::json!({"op": "in", "canvas": path.join("/"), "ops": adds})]
            }
        }
        None => {
            let mut taken: Vec<String> = doc
                .nodes
                .iter()
                .map(|n| n.id.clone())
                .chain(doc.groups.iter().map(|g| g.id.clone()))
                .chain(doc.canvases.iter().map(|c| c.id.clone()))
                .collect();
            let cid = fresh("tests", &mut taken);
            let link = fresh("tests-link", &mut taken);
            let mut table = String::from("table scenarios: Test scenarios {\n  ");
            table.push_str(&COLUMNS.join(" | "));
            table.push('\n');
            for r in &rows {
                let cells: Vec<String> = r
                    .iter()
                    .map(|c| c.replace('|', "\\|").replace('\n', "\\n"))
                    .collect();
                table.push_str(&format!("  {}\n", cells.join(" | ")));
            }
            table.push_str("}\n");
            vec![
                serde_json::json!({"op": "add_canvas", "id": cid, "title": "Test scenarios"}),
                serde_json::json!({"op": "add_node", "id": link, "label": "Test scenarios", "shape": "pill", "canvas": cid, "tags": ["qa"]}),
                serde_json::json!({"op": "in", "canvas": cid, "ops": [{"op": "merge", "dsl": table}]}),
            ]
        }
    };
    Ok(serde_json::json!({ "ops": ops }).to_string())
}

fn fresh(want: &str, taken: &mut Vec<String>) -> String {
    let mut id = want.to_string();
    let mut i = 2;
    while taken.contains(&id) {
        id = format!("{want}-{i}");
        i += 1;
    }
    taken.push(id.clone());
    id
}

fn fenced(text: &str) -> Option<&str> {
    let start = text.find("```")?;
    let rest = &text[start + 3..];
    let nl = rest.find('\n')?;
    let body = &rest[nl + 1..];
    Some(&body[..body.find("```")?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::parse;

    const SHOP: &str = "title: Web shop\n\nlanes checkout: Checkout {\n  lane shopper: Shopper {\n    cart: Open the cart\n    pay: Pay\n  }\n}\nscreen login: Sign in [web] {\n  input email: Email\n}\n";

    const REPLY: &str = "Here they are:\n```shape\ntable t: Tests {\n  scenario | steps | expected | covers | status\n  Pay with a saved card | 1. Open the cart\\n2. Pay | The order is paid | Checkout | to run\n  Wrong password | 1. Sign in with a wrong password | It says the password is wrong | Sign in |\n}\n```\n";

    fn apply(doc: &mut Document, ops: &str) {
        let ops = crate::ops::parse_ops(ops).unwrap();
        crate::ops::apply(doc, &ops).unwrap();
    }

    #[test]
    fn a_draft_goes_in_a_canvas_of_its_own_then_adds_rows() {
        let mut doc = parse(SHOP).unwrap();
        assert_eq!(
            parts(&doc).iter().map(|p| p.kind).collect::<Vec<_>>(),
            ["journey", "screen"]
        );
        let p = draft_prompt(&doc);
        assert!(p.contains("- Checkout (journey): no scenario yet"));
        assert!(p.contains("lanes checkout"));

        let ops = draft_ops(&doc, REPLY).unwrap();
        apply(&mut doc, &ops);
        let c = doc.canvas("tests").unwrap();
        assert_eq!(
            doc.node("tests-link").unwrap().canvas.as_deref(),
            Some("tests")
        );
        let g = c.doc.group("scenarios").unwrap();
        assert!(is_tests(g));
        let all = scenarios(&doc);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].at.path, ["tests"]);
        assert_eq!(all[1].outcome, Outcome::ToRun);
        assert!(parts(&doc).iter().all(|p| covered(&doc, p, &all)));
        // the board still reads back as written
        let text = crate::dsl::print(&doc);
        assert_eq!(crate::dsl::print(&parse(&text).unwrap()), text);

        // again: the new ones only, as rows of that table
        let again = REPLY.replace("Wrong password", "Locked account");
        let ops = draft_ops(&doc, &again).unwrap();
        assert!(ops.contains(r#""canvas":"tests","op":"in""#), "{ops}");
        apply(&mut doc, &ops);
        let labels: Vec<String> = scenarios(&doc).into_iter().map(|s| s.label).collect();
        assert_eq!(
            labels,
            ["Pay with a saved card", "Wrong password", "Locked account"]
        );
        assert!(draft_ops(&doc, &again)
            .unwrap_err()
            .contains("only repeats"));
        assert!(draft_prompt(&doc).contains("Scenarios already written (3)"));
    }

    #[test]
    fn statuses_and_coverage() {
        let doc = parse("lanes refund: Refunds {\n  lane a: Agent {\n    x: Refund\n  }\n}\ntable qa: QA {\n  test | expected | result\n  Refund in full | Money back | Passed\n  Refund twice | Refused | FAIL\n  Partial | Some back | blocked by the bank\n}\n").unwrap();
        let s = scenarios(&doc);
        assert_eq!(
            s.iter().map(|x| x.outcome).collect::<Vec<_>>(),
            [Outcome::Pass, Outcome::Fail, Outcome::Blocked]
        );
        // no covers column: nothing names the journey
        assert!(!covered(&doc, &parts(&doc)[0], &s));
        assert!(draft_ops(&doc, "no table here").is_err());
    }
}
