//! What newer versions of Shape can do with a board, for Rethink.
//!
//! A board records the version that last rethought it (`rethought: 0.75.0`).
//! The next Rethink with a newer Shape gets the [`NOTES`] since then: each is
//! about a built-in perspective (what its summary now reads, and how a board
//! gives it that), and applies only to a board that shows that perspective,
//! from a built-in set or with the built-in id. A board of custom
//! perspectives, or none, is left as its people built it.
use crate::model::Document;

/// This version of Shape.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What a version can do with a board, for one built-in perspective.
pub struct Note {
    pub version: &'static str,
    /// The built-in perspective it serves.
    pub perspective: &'static str,
    /// For the person, in a few words.
    pub short: &'static str,
    /// For the model: what to do with the board.
    pub text: &'static str,
}

/// Oldest first.
pub const NOTES: &[Note] = &[
    Note {
        version: "0.70.0",
        perspective: "marketing",
        short: "launch checklists and public dates for Marketing",
        text: "Marketing's summary reads the readiness checklists (to-do lists, with owners and due dates), the dates people outside see (a timeline; `important` on the ones to announce) and what is not decided yet. Put launch readiness in a to-do list and public dates on a timeline.",
    },
    Note {
        version: "0.70.0",
        perspective: "legal",
        short: "obligations with due dates for Legal",
        text: "Legal's summary reads the obligations: contractual, regulatory and compliance duties in a section named Obligations (or tagged `legal`), each with `due:` when it has a date, and the decisions waiting as decision records.",
    },
    Note {
        version: "0.70.0",
        perspective: "user",
        short: "the journey and what users said for Users",
        text: "The Users summary reads the journey (swimlanes, or screens in order) and what users said: quotes and feedback as stickies in a section named What users told us (or tagged `user`).",
    },
    Note {
        version: "0.70.0",
        perspective: "design",
        short: "screens and their flow for Design",
        text: "Design's summary reads the wireframe screens and the arrows between them (the flow), and design decisions as decision records.",
    },
    Note {
        version: "0.71.0",
        perspective: "delivery",
        short: "risks per piece of work for Delivery",
        text: "Delivery reads the risks of each piece of work: a gantt task with risks or blockers of its own can open a canvas holding them (`canvas: ID` on the task, a risk matrix inside), and its bar then says how many risks it holds.",
    },
    Note {
        version: "0.74.0",
        perspective: "delivery",
        short: "one roadmap across projects for Delivery",
        text: "When a board holds several projects (each with its own plan, risks, blockers or details), Delivery shows them on one roadmap, a row per project, with each project's details in its own canvas: give each project a canvas, opened by a box on this board, and move its plan, risks and details in. A single project stays flat. On a gantt chart, a milestone that marks a release of a task can sit on that task's row (`row: TASK-ID`).",
    },
    Note {
        version: "0.74.0",
        perspective: "qa",
        short: "test scenarios for QA",
        text: "The QA summary reads how it works (sequences, swimlanes, screens) and the test scenarios: a table with the columns scenario | steps | expected | covers | status. If the board has flows or screens and no scenarios yet, add a canvas `tests` (\"Test scenarios\"), opened by a box, with such a table: the main scenarios a tester would run (steps numbered and separated by `\\n`, the expected result, `covers` naming the flow or screen, status `to run`).",
    },
    Note {
        version: "0.76.0",
        perspective: "delivery",
        short: "sub-projects and their stages on the roadmap for Delivery",
        text: "A gantt task can be made of parts, written indented under it: a sub-project with its stages, which the roadmap unfolds from the project's row. When a project's plan holds several sub-projects (products, workstreams), group each one's tasks under it, with the stages the team uses, and fold the ones not being delivered now. Rename tasks and milestones named by a number (*Phase 2 beta*) after what they deliver and whose they are (*Mobile app beta*), and give what a stage waits for (an outside approval, a spike, another team) a task of its own with `after:`.",
    },
    Note {
        version: "0.80.0",
        perspective: "delivery",
        short: "platforms marked on gantt tasks for Delivery",
        text: "A gantt task can say which platform it is for (`platform: web`, `ios`, `android` or any word), shown as a chip by its name. Where a task names its platform in its title (*Implement (iOS)*, *Spike · Android*), move it to `platform:` and keep the title short (*Implement*); where work is done for several platforms, give each platform a task of its own.",
    },
];

/// Whether version `a` is newer than `b` (`0.10.0` > `0.9.9`).
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parts(a) > parts(b)
}

/// The notes for `root` (the whole board) since it was last rethought:
/// newer than its `rethought:` (all of them when it has none) and up to this
/// version, for the built-in perspectives it shows.
pub fn since(root: &Document) -> Vec<&'static Note> {
    let ids: Vec<String> = crate::perspectives::all(root)
        .into_iter()
        .map(|p| p.id)
        .collect();
    let last = root.settings.rethought.as_deref();
    NOTES
        .iter()
        .filter(|n| last.is_none_or(|l| newer(n.version, l)) && !newer(n.version, VERSION))
        .filter(|n| ids.iter().any(|i| i == n.perspective))
        .collect()
}

/// The notes as a section of the Rethink prompt ("" when there are none).
pub fn prompt_section(root: &Document) -> String {
    let notes = since(root);
    if notes.is_empty() {
        return String::new();
    }
    let mut s = format!(
        "## What Shape can do now\n\nThis board was last rethought {}; Shape is now at {VERSION}, and its perspectives can show more of a board. Bring the board up to it where it fits what is on it, from what the board already says (never invent facts):\n\n",
        match &root.settings.rethought {
            Some(v) => format!("with Shape {v}"),
            None => "with an earlier Shape".to_string(),
        }
    );
    for n in notes {
        s.push_str(&format!("- {}\n", n.text));
    }
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_since_the_last_rethink_for_the_boards_perspectives() {
        let all = |src: &str| -> Vec<&str> {
            since(&crate::dsl::parse(src).unwrap())
                .iter()
                .map(|n| n.short)
                .collect()
        };
        // a project team, never rethought: everything for its perspectives
        let p = all("perspectives: project\n");
        assert!(
            p.contains(&"test scenarios for QA")
                && p.contains(&"risks per piece of work for Delivery")
        );
        assert!(
            !p.contains(&"the journey and what users said for Users"),
            "no Users on a project board"
        );
        // rethought with 0.71: only what came after
        let p = all("perspectives: project\nrethought: 0.71.0\n");
        assert!(!p.contains(&"risks per piece of work for Delivery"));
        assert!(p.contains(&"one roadmap across projects for Delivery"));
        // rethought with this version: nothing new
        assert!(all(&format!("perspectives: project\nrethought: {VERSION}\n")).is_empty());
        // a built-in id the board has itself counts; custom ones and none do not
        assert_eq!(
            all("perspective qa: Testing [tags: qa]\nperspective ops: Ops [tags: ops]\n"),
            ["test scenarios for QA"]
        );
        assert!(all("perspective ops: Ops [tags: ops]\n").is_empty());
        assert!(all("a: Box\n").is_empty());
        // the setting round-trips
        let src = "title: X\nrethought: 0.74.0\n\na: Box\n";
        assert_eq!(crate::dsl::print(&crate::dsl::parse(src).unwrap()), src);
        assert!(newer("0.10.0", "0.9.9") && !newer("0.74.0", "0.74.0"));
    }
}
