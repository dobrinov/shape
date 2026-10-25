//! Planning the delivery of a board that has no plan yet.
//!
//! [`questions_prompt`] asks a model what it needs to know to plan how the
//! work gets delivered (what the board does not say: the team, the start,
//! the dates that are fixed, the milestones, the platforms…), and
//! [`questions`] reads its reply. The person answers what they can;
//! [`plan_prompt`] then asks for the plan as ops: a section with a gantt
//! chart (sub-projects, stages, milestones, dependencies), the delivery
//! risks, and the open questions for what was not answered. The Delivery
//! summary (its traffic lights and burndown) follows from that plan.
use crate::model::Document;
use serde::{Deserialize, Serialize};

/// A question about the delivery, for the person to answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub question: String,
    /// What an answer looks like (`a date, or "when the team is free"`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hint: String,
}

/// Whether the board (any canvas) has a plan: a gantt chart with a task.
pub fn has_plan(root: &Document) -> bool {
    crate::dashboard::docs(root).iter().any(|(_, d)| {
        d.groups.iter().any(|g| {
            g.gantt
                && d.nodes
                    .iter()
                    .any(|n| n.group.as_deref() == Some(g.id.as_str()))
        })
    })
}

fn heading(root: &Document, what: &str, today: Option<i64>) -> String {
    let title = root.title.as_deref().unwrap_or("Untitled board");
    let mut s = format!("# {what} \"{title}\"\n\n");
    if let Some(t) = today {
        s.push_str(&format!("Today is {}.\n\n", crate::gantt::format_date(t)));
    }
    s.push_str("You are the delivery lead on this project. The board below says what is being built and why: its scope, screens, decisions, risks and anything already known about the team and the dates. It is written in the `shape` language (`id: Label [attrs]` is a box, `a -> b: label` an arrow, `group id: Label { … }` a section; blocks hold their items one per line), the top board first, then each canvas inside it.\n\n");
    if let Some(d) = &root.description {
        s.push_str(&format!("About it: {d}\n\n"));
    }
    s.push_str(&crate::prompt::board_by_canvas(root, today));
    s.push('\n');
    s
}

/// A prompt asking a model what it needs to know to plan the delivery.
pub fn questions_prompt(root: &Document, today: Option<i64>) -> String {
    let mut s = heading(root, "Questions to plan the delivery of", today);
    s.push_str("## What to do\n\nThere is no delivery plan yet. Before drafting one, ask the people what you need to know that the board does not say. Ask 5 to 10 questions, the most useful first, each about one thing, in plain words a product manager answers in a line: who is on the team and what they work on, when the work starts, the dates that are fixed (a launch, a contract, an event), the milestones that matter (a beta, general availability), the sub-projects or workstreams and the stages each goes through, the platforms, what it waits for outside the team (an approval, another team, a supplier), and what worries them. Do not ask what the board already says.\n\n## How to reply\n\nOne ```json code block, no other text:\n\n```json\n{\"questions\": [\n  {\"id\": \"start\", \"question\": \"When does the work start?\", \"hint\": \"a date, or when the team is free\"},\n  {\"id\": \"team\", \"question\": \"Who is on the team, and what does each work on?\", \"hint\": \"e.g. 2 web, 1 mobile, 1 designer\"}\n]}\n```\n");
    s
}

/// The questions in a model's reply to [`questions_prompt`].
pub fn questions(reply: &str) -> Result<Vec<Question>, String> {
    let body = crate::ops::extract_json(reply).ok_or("no ```json block with the questions")?;
    #[derive(Deserialize)]
    struct Reply {
        questions: Vec<Question>,
    }
    let r: Reply = serde_json::from_str(body).map_err(|e| format!("the questions do not read: {e}"))?;
    let qs: Vec<Question> = r
        .questions
        .into_iter()
        .filter(|q| !q.question.trim().is_empty())
        .take(12)
        .collect();
    if qs.is_empty() {
        return Err("no questions".into());
    }
    Ok(qs)
}

/// A prompt asking a model for the delivery plan, from the board and the
/// answers to its questions (an empty answer: not known yet). The reply
/// is a note and ops, as for Rethink.
pub fn plan_prompt(root: &Document, answers: &[(String, String)], today: Option<i64>) -> String {
    let mut s = heading(root, "Plan the delivery of", today);
    s.push_str("## What the people said\n\n");
    for (q, a) in answers {
        let a = a.trim();
        s.push_str(&format!(
            "- {}\n  {}\n",
            q.trim(),
            if a.is_empty() { "(not known yet)" } else { a }
        ));
    }
    s.push_str("\n## What to do\n\nDraft the delivery plan from the board and these answers, on the top board in a section `delivery` (\"Delivery plan\") tagged `delivery`:\n\n- A `gantt` chart of the work: a group per sub-project or workstream, with the stages it goes through (the ones the people named; else the ones the board implies), each with its length, owner and platform when known, `after:` for what it waits for, and a milestone for each date that matters (a beta, general availability, a fixed date). Use dates that were given; where none was, start from the start date and the lengths. A stage nobody can size yet has no length.\n- A `risks` matrix of what could stop the delivery (from the board and the answers), each with its probability, impact and owner.\n- A `todo` list \"Open questions for the plan\" with what was not answered and what you assumed, so it can be settled later.\n- Keep the plan's names short and plain (*Beta*, *Implement* with `platform: ios`), as below.\n\nThe Delivery summary reads the plan: its milestones get traffic lights and the work before the next one a burndown.\n\n");
    s.push_str(crate::prompt::PLANS);
    s.push_str("## How to reply\n\nOne ```json code block with a short `note` (what the plan says, and what you assumed) and the `ops`; merging the blocks in the `shape` language is cheapest:\n\n```json\n{\"note\": \"…\", \"ops\": [\n  {\"op\": \"add_group\", \"id\": \"delivery\", \"label\": \"Delivery plan\", \"tags\": [\"delivery\"]},\n  {\"op\": \"merge\", \"dsl\": \"group delivery {\\n  gantt plan: Plan [start: 2026-11-02] {\\n    Web app\\n      Build [4w, owner: ana, platform: web]\\n      Beta [2w]\\n      General availability [2027-01-11, milestone]\\n  }\\n}\"}\n]}\n```\n\n");
    s.push_str(crate::prompt::OPS_REFERENCE);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_then_a_plan() {
        let doc = crate::dsl::parse("title: Chat widget\n\nscope: Answers from the help pages\n").unwrap();
        assert!(!has_plan(&doc));
        let q = questions_prompt(&doc, crate::gantt::parse_date("2026-10-25"));
        assert!(q.contains("Today is 2026-10-25.") && q.contains("Answers from the help pages") && q.contains("```json"));
        let qs = questions("```json\n{\"questions\": [{\"id\": \"start\", \"question\": \"When does the work start?\", \"hint\": \"a date\"}, {\"id\": \"x\", \"question\": \" \"}]}\n```").unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].hint, "a date");
        assert!(questions("no json").is_err());
        let p = plan_prompt(&doc, &[("When does the work start?".into(), "2 Nov".into()), ("Who is on the team?".into(), "".into())], None);
        assert!(p.contains("- When does the work start?\n  2 Nov") && p.contains("(not known yet)") && p.contains("Open questions for the plan"));
        let planned = crate::dsl::parse("gantt plan: Plan {\n  Build [1w]\n}\n").unwrap();
        assert!(has_plan(&planned));
    }
}
