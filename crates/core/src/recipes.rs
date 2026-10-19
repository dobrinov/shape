//! Recipes: arrangements of ordinary blocks for things a software team keeps
//! coming back to (incidents, OKRs, a burndown…). They are not blocks of
//! their own: a recipe is a section written in the `shape` language. The
//! AI's prompts list them (one line each: when to use one, and its shape),
//! so a model reaches for the right shape when what it is told fits; the
//! editor inserts one on request. A team adds its own as `.shape` files in
//! a `recipes/` folder (the first line: `# recipe: Name — when to use it`).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    /// When to use it, for people and models.
    pub when: String,
    /// Its shape in a line, for prompts (few tokens).
    pub shape: String,
    /// The section to insert, in the `shape` language.
    pub source: String,
}

fn r(id: &str, name: &str, when: &str, shape: &str, source: &str) -> Recipe {
    Recipe {
        id: id.into(),
        name: name.into(),
        when: when.into(),
        shape: shape.into(),
        source: source.trim_start_matches('\n').into(),
    }
}

/// The built-in recipes.
pub fn builtins() -> Vec<Recipe> {
    vec![
        r("achievements", "Achievements", "wins, launches, milestones reached, praise: when and how we won",
          "section `achievements` with a timeline of wins (important ones starred) and a table `how-we-won` (win | what made it work | who)",
          r#"
group achievements: Achievements {
  timeline wins: Wins {
    2026-10-28: Open beta shipped on time [important, green]
  }
  table how-we-won: How we won {
    win | what made it work | who
    Open beta on time | Cut scope early; daily check-ins | team
  }
}
"#),
        r("mistakes", "Mistakes and lessons", "something went wrong that was not an outage: a slip, a wrong bet, a misunderstanding, and what we learned",
          "section `mistakes` with a table `lessons` (when | what went wrong | why | lesson) and a kanban `follow-ups` (to do, done)",
          r#"
group mistakes: Mistakes and lessons {
  table lessons: Mistakes {
    when | what went wrong | why | lesson
    2026-10-14 | The beta date slipped | Fraud reporting was found late | Review compliance needs at kick-off
  }
  kanban follow-ups: Follow-ups {
    column follow-ups.todo: To do {
      fu-kickoff: Add a compliance review to kick-offs
    }
    column follow-ups.done: Done {}
  }
}
"#),
        r("incidents", "Incidents", "outages, production bugs, security or data issues, and their post-mortems",
          "section `incidents` with a table `incident-log` (date | what happened | severity | impact | owner | status), a timeline of them, and a decision record per post-mortem (context, decision = the fix, consequences = follow-ups)",
          r#"
group incidents: Incidents {
  table incident-log: Incident log {
    date | what happened | severity | impact | owner | status
    2026-10-20 | Payments down for 2 hours | high | Checkout failed for everyone | ana | fixed
  }
  timeline incident-history: When they happened {
    2026-10-20: Payments down for 2 hours [important, red]
  }
  adr pm-payments-down: Post-mortem: payments down [accepted] {
    context: What happened, who noticed, how long it lasted
    decision: What we changed so it does not happen again
    consequences: Follow-ups and their owners
  }
}
"#),
        r("status", "Status report (traffic light)", "how things stand across areas: on track, at risk, off track",
          "section `status` with a table `status-report` (area | status | trend | note), each row coloured green, yellow or red by its status",
          r#"
group status: Status report {
  table status-report: Where we stand {
    area | status | trend | note
    Scope | on track | steady | Signed off [green]
    Schedule | at risk | worse | Beta moved a week [yellow]
    Budget | off track | worse | Over by 10% [red]
  }
}
"#),
        r("burndown", "Burndown", "work left in a sprint or release against the ideal line",
          "a line chart `burndown` with columns day | ideal | remaining (one row per day)",
          r#"
chart burndown: Sprint burndown [line, unit: points] {
  day | ideal | remaining
  Mon | 40 | 40
  Tue | 32 | 36
  Wed | 24 | 30
  Thu | 16 | 21
  Fri | 8 | 12
}
"#),
        r("okrs", "OKRs", "objectives and key results, and how far along they are",
          "section `okrs` with a mindmap `objectives` (objective → key results) and a horizontal bar chart `kr-progress` (key result | done, in %)",
          r#"
group okrs: OKRs {
  mindmap objectives: Make checkout effortless {
    Saved cards for returning shoppers
    Checkout under 30 seconds
    Card errors down by half
  }
  chart kr-progress: Key results [bar, horizontal, unit: %] {
    key result | done
    Saved cards | 60
    Under 30 s | 35
    Errors halved | 20
  }
}
"#),
        r("roadmap", "Roadmap", "what ships when over the next quarters",
          "a gantt `roadmap` (scale: month) with one task per theme and milestones for releases",
          r#"
gantt roadmap: Roadmap [start: 2026-11-02, scale: month] {
  Saved cards [6w]
  Apple Pay [4w, after: saved-cards]
  Launch [after: apple-pay, milestone]
}
"#),
        r("retro", "Retrospective", "what went well, what to improve, and what we will do about it",
          "section `retro` with sections `went-well` and `to-improve` of stickies, and a kanban `retro-actions` (to do, done)",
          r#"
group retro: Retrospective {
  group went-well: Went well {
    ww-pairing: Pairing on the parser [sticky, green]
  }
  group to-improve: To improve {
    ti-ci: Flaky CI on macOS [sticky, red]
  }
  kanban retro-actions: Actions {
    column retro-actions.todo: To do {
      ra-ci: Pin the macOS runner image
    }
    column retro-actions.done: Done {}
  }
}
"#),
        r("raci", "RACI", "who is responsible, accountable, consulted and informed for each piece of work",
          "a table `raci` (task | then one column per person or team, cells R, A, C or I)",
          r#"
table raci: Who does what {
  task | lead | PM | VP Eng
  Discovery | R | A | I
  Spec | A | R | C
  Release | R | C | A
}
"#),
        r("prioritisation", "Prioritisation", "deciding what to do first: effort against impact",
          "a scatter chart `prioritisation` (idea | effort | impact, 1–10)",
          r#"
chart prioritisation: Effort and impact [scatter] {
  idea | effort | impact
  Saved cards | 6 | 9
  Dark mode | 2 | 3
  Apple Pay | 5 | 7
}
"#),
        r("personas", "Personas", "who the users are: their goals and what gets in their way",
          "section `personas` of person boxes, each with a detail: goals · pains",
          r#"
group personas: Personas {
  persona-shopper: Returning shopper [person, detail: Wants to pay in one tap · Hates retyping the card]
  persona-admin: Shop admin [person, detail: Wants fewer support tickets · Fears fraud]
}
"#),
        r("journey", "User journey", "the steps a user goes through, and what the system does at each",
          "swimlanes `journey` with lanes for the user and the system, steps in order, arrows between them",
          r#"
lanes journey: Checkout journey {
  lane user: Shopper {
    j-browse: Picks a product
    j-pay: Pays with a saved card
  }
  lane system: System {
    j-charge: Charges the card
  }
}
j-browse -> j-pay -> j-charge
"#),
        r("user-flow", "Screens and flow", "an app's or a site's screens and how people move between them (wireframes)",
          "section `user-flow` of `screen` blocks (mobile or web; one component per line) with arrows from the button or link on one screen to the next screen",
          r#"
group user-flow: User flow {
  screen flow-start: Start [mobile] {
    header: App
    title: Welcome
    text: What the app does, in a line.
    button go: Get started [blue]
  }
  screen flow-home: Home [mobile] {
    header: Home
    search: Search
    list: First item | Second item | Third item
    tabbar: Home | Search | Me
  }
}
flow-start.go -> flow-home: taps
"#),
        r("tech-debt", "Tech debt register", "shortcuts and old code that slow us down, and what fixing them costs",
          "a table `tech-debt` (item | pain | effort | owner | status), rows coloured by pain",
          r#"
table tech-debt: Tech debt {
  item | pain | effort | owner | status
  Legacy cart service | high | large | sam | planned [red]
  Flaky payment tests | medium | small | ana | doing [yellow]
}
"#),
        r("release-checklist", "Release checklist", "what must be true before shipping a release",
          "a todo list `release-checklist`, one item per check (`[x]` when done), with owners and due dates",
          r#"
todo release-checklist: Release checklist {
  [x] Release notes written [owner: ana]
  [ ] Migrations run on staging [owner: sam]
  [ ] Rollback tested
    [ ] On staging
  [ ] Support told what changes
}
"#),
        r("stakeholders", "Stakeholder map", "who cares about the work and how much say they have",
          "a scatter chart `stakeholders` (who | interest | influence, 1–10)",
          r#"
chart stakeholders: Stakeholders [scatter] {
  who | interest | influence
  CFO | 4 | 9
  Support | 9 | 3
  Sales | 7 | 6
}
"#),
        r("ownership", "Team ownership", "which team or person owns which area",
          "a mindmap `ownership` (area → owner, as topics)",
          r#"
mindmap ownership: Who owns what {
  Payments
    ana
  Checkout
    sam
  Mobile
    rado
}
"#),
        r("metrics", "Metrics", "the numbers that tell whether it works",
          "section `metrics` with a line chart of the main metric over time and a bar chart comparing segments",
          r#"
group metrics: Metrics {
  chart conversion: Conversion [line, unit: %] {
    week | conversion
    W40 | 2.1
    W41 | 2.4
    W42 | 2.6
  }
  chart by-device: By device [bar, unit: %] {
    device | conversion
    Mobile | 1.9
    Desktop | 3.2
  }
}
"#),
    ]
}

/// A team's recipe from a `.shape` file: the first line says
/// `# recipe: Name — when to use it`; the rest is the section.
pub fn from_file(id: &str, text: &str) -> Option<Recipe> {
    let first = text.lines().next()?.trim();
    let head = first
        .strip_prefix('#')?
        .trim()
        .strip_prefix("recipe:")?
        .trim();
    let (name, when) = head
        .split_once(" — ")
        .or_else(|| head.split_once(" - "))
        .unwrap_or((head, ""));
    let source: String = text
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
        + "\n";
    Some(Recipe {
        id: id.into(),
        name: name.trim().into(),
        when: when.trim().into(),
        // a team's own are short: the model reads them whole
        shape: format!("this shape:\n```shape\n{source}```"),
        source,
    })
}

/// The recipes for prompts: a line each (and a team's own in full).
pub fn prompt_section(extra: &[Recipe]) -> String {
    let mut s = String::from("## Recipes\n\nShapes for things that keep coming up. When what you are told fits one, use its shape (ids and rows are examples: adapt them); when the board already has one, add to it instead of making another.\n\n");
    for r in builtins().iter().chain(extra) {
        s.push_str(&format!("- {} (for {}): {}\n", r.name, r.when, r.shape));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recipe_is_a_valid_board() {
        let all = builtins();
        assert!(all.len() >= 15);
        for r in &all {
            let doc = crate::dsl::parse(&r.source).unwrap_or_else(|e| panic!("{}: {:?}", r.id, e));
            assert!(!doc.nodes.is_empty() || !doc.groups.is_empty(), "{}", r.id);
            assert!(!r.when.is_empty() && !r.shape.is_empty(), "{}", r.id);
        }
        // their ids do not clash with each other
        let mut ids = std::collections::HashSet::new();
        for r in &all {
            let doc = crate::dsl::parse(&r.source).unwrap();
            for id in doc
                .nodes
                .iter()
                .map(|n| &n.id)
                .chain(doc.groups.iter().map(|g| &g.id))
            {
                assert!(ids.insert(id.clone()), "{id} is in two recipes");
            }
        }
        let own = from_file(
            "oncall",
            "# recipe: On-call — who is on call when\ntable oncall: On call {\n  week | who\n}\n",
        )
        .unwrap();
        assert_eq!(
            (own.name.as_str(), own.when.as_str()),
            ("On-call", "who is on call when")
        );
        assert!(own.source.starts_with("table oncall"));
        assert!(from_file("x", "no header\n").is_none());
        let p = prompt_section(&[own]);
        assert!(
            p.contains("- Incidents (for outages")
                && p.contains("- On-call (for who is on call when): this shape:"),
            "{p}"
        );
    }
}
