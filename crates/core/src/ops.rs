//! Operations: the one way to change a document. The editor, the CLI, the
//! library API and LLM responses all express edits as a list of ops:
//!
//! ```json
//! {"ops": [
//!   {"op": "add_node", "id": "cache", "label": "Redis", "shape": "cylinder", "group": "backend"},
//!   {"op": "add_edge", "from": "api", "to": "cache", "label": "reads"},
//!   {"op": "resolve_comment", "id": "c1", "resolution": "Added a cache in front of the DB"}
//! ]}
//! ```
//!
//! A batch is atomic: if one op fails, nothing is applied. In update ops an
//! empty string clears an optional field (`"color": ""`).
use serde::{Deserialize, Serialize};

use crate::model::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    AddNode {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shape: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
        /// The asset the box shows (see `add_asset`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<String>,
        /// The canvas the box opens (see `add_canvas`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        canvas: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        w: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        h: Option<f64>,
        /// Mindmap: the topic to branch from.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        /// Position among the parent's branches (0 = first; default last).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        /// Start a mindmap here: right, left, both or down.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mindmap: Option<String>,
        /// Gantt task: start date (`YYYY-MM-DD`; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<String>,
        /// Gantt task: duration in days (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        days: Option<f64>,
        /// Gantt task: the tasks it starts after.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<Vec<String>>,
        /// Gantt task: a milestone (a point in time, not a bar).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        milestone: Option<bool>,
        /// Gantt milestone: the task whose row it sits on ("" gives it its own row).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        row: Option<String>,
        /// Gantt task: the task of its chart it is part of (a phase of a
        /// sub-project; "" puts it back at the top).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        under: Option<String>,
        /// Timeline event: when, as people write it (`2026-10-28`, `Oct 2026`,
        /// `7–14 May 2026`; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        date: Option<String>,
        /// Timeline event: one that matters most (drawn as a star).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        important: Option<bool>,
        /// A wireframe screen's component: what it is drawn as (`button`,
        /// `input`, `list`…; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        widget: Option<String>,
        /// A to-do item's depth under the item above it (0: top level).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<u8>,
        /// Gantt task: percent done, 0–100 (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        done: Option<f64>,
        /// Gantt task: who does it ("" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner: Option<String>,
        /// Gantt task: the platform it is for (web, ios, android, any word; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        platform: Option<String>,
        /// Forecast launch: what it adds a month once ramped up.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adds: Option<f64>,
        /// Forecast launch: months to ramp up (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ramp: Option<f64>,
        /// Risk: probability, 1 to 5 (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        probability: Option<u8>,
        /// Risk: impact, 1 to 5 (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        impact: Option<u8>,
        /// Chart row: one value per series (null for none).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        values: Option<Vec<Option<f64>>>,
        /// Table row: its cells after the first (the label).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cells: Option<Vec<String>>,
        /// Database table column: type, pk, unique, null, fk, default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        column: Option<Column>,
    },
    UpdateNode {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shape: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
        /// The asset the box shows ("" for none).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<String>,
        /// The canvas the box opens ("" for none).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        canvas: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        w: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        h: Option<f64>,
        /// Mindmap: move the topic (with its branch) under another one;
        /// "" detaches it into a mindmap of its own.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        /// Position among its siblings (with or without a new parent).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        /// Mindmap layout of a centre (right, left, both, down); "" ends the map.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mindmap: Option<String>,
        /// Fold or unfold the branches under a topic.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        collapsed: Option<bool>,
        /// Gantt task: start date (`YYYY-MM-DD`; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<String>,
        /// Gantt task: duration in days (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        days: Option<f64>,
        /// Gantt task: the tasks it starts after.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<Vec<String>>,
        /// Gantt task: a milestone (a point in time, not a bar).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        milestone: Option<bool>,
        /// Gantt milestone: the task whose row it sits on ("" gives it its own row).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        row: Option<String>,
        /// Gantt task: the task of its chart it is part of (a phase of a
        /// sub-project; "" puts it back at the top).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        under: Option<String>,
        /// Timeline event: when, as people write it (`2026-10-28`, `Oct 2026`,
        /// `7–14 May 2026`; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        date: Option<String>,
        /// Timeline event: one that matters most (drawn as a star).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        important: Option<bool>,
        /// A wireframe screen's component: what it is drawn as (`button`,
        /// `input`, `list`…; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        widget: Option<String>,
        /// A to-do item's depth under the item above it (0: top level).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<u8>,
        /// Gantt task: percent done, 0–100 (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        done: Option<f64>,
        /// Gantt task: who does it ("" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner: Option<String>,
        /// Gantt task: the platform it is for (web, ios, android, any word; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        platform: Option<String>,
        /// Forecast launch: what it adds a month once ramped up.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adds: Option<f64>,
        /// Forecast launch: months to ramp up (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ramp: Option<f64>,
        /// Risk: probability, 1 to 5 (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        probability: Option<u8>,
        /// Risk: impact, 1 to 5 (0 clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        impact: Option<u8>,
        /// Chart row: one value per series (null for none).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        values: Option<Vec<Option<f64>>>,
        /// Table row: its cells after the first (the label).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cells: Option<Vec<String>>,
        /// Database table column: type, pk, unique, null, fk, default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        column: Option<Column>,
    },
    /// Add topics from an indented outline (or a Markdown list) under
    /// `parent`; without `parent` the first line becomes a new mindmap.
    AddOutline {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        outline: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
        /// A new mindmap: the section it goes into.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group: Option<String>,
    },
    /// A new, empty canvas inside this board; a box opens it with
    /// `canvas: ID`.
    AddCanvas {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        title: String,
    },
    /// Retitle a canvas.
    UpdateCanvas {
        id: String,
        title: String,
    },
    /// Add or change a perspective (`remove: true` takes it off the board;
    /// a built-in one comes back). Fields left out stay as they are.
    Perspective {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        show: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<bool>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        remove: bool,
    },
    /// Set up the presentation (made when there is none): its title, the
    /// transition, the instructions, the slides in order (the whole list:
    /// `{"target": "risks", "notes": "…", "transition": "zoom"}`, `*` for
    /// the whole board). `remove: true` takes it off. Fields left out stay.
    Presentation {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transition: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slides: Option<Vec<crate::model::Slide>>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        remove: bool,
    },
    /// Ops for a canvas inside this board: `canvas` is its path of ids
    /// (`plan`, or `plan/details` for one inside it).
    In {
        canvas: String,
        #[serde(default)]
        ops: Vec<Op>,
    },
    /// Keep an image in the board: `data` is a `data:image/…;base64,…` URI.
    /// Boxes show it with `image: ID`.
    AddAsset {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        data: String,
    },
    /// Rename an id everywhere it is referenced (edges, comments, members).
    Rename {
        id: String,
        to: String,
    },
    /// Remove a node or group (with its edges) or an edge or a comment.
    /// A mindmap topic goes with its whole branch.
    Remove {
        id: String,
        /// For groups: keep the members (moved to the parent group). Default true.
        /// For mindmap topics: keep the branches (moved up a level). Default false.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        keep_members: Option<bool>,
    },
    /// Pin a node, swimlanes or gantt chart at a position (top-left corner).
    /// For a mindmap topic the position is relative to its centre's top-left.
    Move {
        id: String,
        x: f64,
        y: f64,
    },
    /// Move pinned nodes by an offset (a node, or every node in a group).
    Nudge {
        id: String,
        dx: f64,
        dy: f64,
    },
    /// Let the layout place a node (or swimlanes, gantt chart) again; without
    /// `id`, everything.
    Unpin {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    AddEdge {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        from: String,
        to: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arrow: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    UpdateEdge {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arrow: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        /// A message of a sequence diagram: its place among the messages
        /// (0 = first); another arrow: among all arrows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    AddGroup {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        /// The perspectives it belongs to (`tech` for a technical section).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        /// Nodes (or groups) to move into the new group.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        members: Vec<String>,
        /// Swimlanes: the groups inside are lanes.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        lanes: bool,
        /// A kanban board: its groups are columns of cards (implies `lanes`).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        kanban: bool,
        /// A sequence diagram: its nodes are participants, arrows between
        /// them messages in order.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        sequence: bool,
        /// Gantt chart: the nodes inside are tasks.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        gantt: bool,
        /// Gantt chart: its first day (`YYYY-MM-DD`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<String>,
        /// Risk matrix: the nodes inside are risks.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        risks: bool,
        /// Risk matrix: 3, 4 or 5 levels (5 by default).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        levels: Option<u8>,
        /// Chart: bar, line, area, pie, donut or scatter.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chart: Option<String>,
        /// Chart: series stacked.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stacked: Option<bool>,
        /// Bar chart: bars across.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        horizontal: Option<bool>,
        /// Chart: the values' unit ("" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        /// Chart: the series' names.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        series: Option<Vec<String>>,
        /// Chart: what the rows are (the header's first cell; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        /// A table (its nodes are rows).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        table: Option<bool>,
        /// A timeline (its nodes are events, each with a `date`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeline: Option<bool>,
        /// A cycle (its nodes are steps, each leading to the next).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cycle: Option<bool>,
        /// A to-do list (its nodes are items; `done: 100` ticks one).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        todo: Option<bool>,
        /// A wireframe screen drawn as this device (mobile, tablet, web;
        /// its nodes are components); "" makes it a section again.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        screen: Option<String>,
        /// A database table (its nodes are columns).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entity: Option<bool>,
        /// A forecast (its nodes are launches).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        forecast: Option<bool>,
        /// Forecast: the number today (a month's revenue).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        now: Option<f64>,
        /// A database schema: a section of tables from a schema file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        schema: Option<bool>,
        /// Schema: the file its tables come from, relative to the board ("" clears).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        /// Schema: only the tables matching these globs ("" clears).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        only: Option<String>,
        /// A decision record with this status (proposed, accepted, rejected,
        /// superseded, deprecated); its nodes are its fields.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adr: Option<String>,
        /// Table: the columns' names (the header row).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<Vec<String>>,
    },
    UpdateGroup {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        /// Replaces its tags ([] clears them).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lanes: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kanban: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sequence: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        gantt: Option<bool>,
        /// Gantt chart: its first day (`YYYY-MM-DD`; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<String>,
        /// Gantt chart: `day`, `week`, `month` or `quarter` ("" resets it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scale: Option<String>,
        /// Risk matrix: 3, 4 or 5 levels (0 resets it to 5).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        levels: Option<u8>,
        /// Chart: bar, line, area, pie, donut or scatter.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chart: Option<String>,
        /// Chart: series stacked.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stacked: Option<bool>,
        /// Bar chart: bars across.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        horizontal: Option<bool>,
        /// Chart: the values' unit ("" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        /// Chart: the series' names.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        series: Option<Vec<String>>,
        /// Chart: what the rows are (the header's first cell; "" clears it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<String>,
        /// A table (its nodes are rows).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        table: Option<bool>,
        /// A timeline (its nodes are events, each with a `date`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeline: Option<bool>,
        /// A cycle (its nodes are steps, each leading to the next).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cycle: Option<bool>,
        /// A to-do list (its nodes are items; `done: 100` ticks one).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        todo: Option<bool>,
        /// A wireframe screen drawn as this device (mobile, tablet, web;
        /// its nodes are components); "" makes it a section again.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        screen: Option<String>,
        /// A database table (its nodes are columns).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entity: Option<bool>,
        /// A forecast (its nodes are launches).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        forecast: Option<bool>,
        /// Forecast: the number today (a month's revenue).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        now: Option<f64>,
        /// A database schema: a section of tables from a schema file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        schema: Option<bool>,
        /// Schema: the file its tables come from, relative to the board ("" clears).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        /// Schema: only the tables matching these globs ("" clears).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        only: Option<String>,
        /// A decision record with this status (proposed, accepted, rejected,
        /// superseded, deprecated); its nodes are its fields.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adr: Option<String>,
        /// Table: the columns' names (the header row).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<Vec<String>>,
    },
    AddComment {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<String>,
        /// The other elements it is about too (`on` is where its pin goes).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        also: Option<Vec<String>>,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
    },
    Reply {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
    },
    EditComment {
        id: String,
        text: String,
    },
    ResolveComment {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resolution: Option<String>,
    },
    ReopenComment {
        id: String,
    },
    /// Document settings: title, description, direction, layout, routing, theme.
    Set {
        key: String,
        value: String,
    },
    /// Statements in the Shape language, added to the board as if they
    /// were written at its end: new elements are created, and a box, group
    /// or arrow written again by its id gets the attributes given (`api
    /// [red]`). The cheapest way to add a lot at once.
    Merge {
        dsl: String,
    },
    /// Add a theme, or replace the one with its id: its text in the language
    /// (`theme brand [base: light] {\n  background: #faf9f6\n}`).
    Theme {
        source: String,
    },
}

/// What a batch did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Applied {
    /// Ids created by the batch, in order (useful when ids were generated).
    pub created: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpError {
    /// 0-based index of the failing op.
    pub index: usize,
    pub message: String,
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "op #{}: {}", self.index + 1, self.message)
    }
}

/// Apply a batch atomically.
pub fn apply(doc: &mut Document, ops: &[Op]) -> Result<Applied, OpError> {
    let mut work = doc.clone();
    let mut applied = Applied::default();
    for (index, op) in ops.iter().enumerate() {
        apply_one(&mut work, op, &mut applied.created).map_err(|message| OpError {
            index,
            message: suggest(&work, message),
        })?;
        applied.count += 1;
    }
    check_parts(&mut work).map_err(|message| OpError {
        index: ops.len().saturating_sub(1),
        message,
    })?;
    *doc = work;
    Ok(applied)
}

/// Gantt tasks are parts (`under`) of tasks in their own chart only: a task
/// that leaves its chart takes no part with it (they move up), and a task
/// cannot be part of itself.
fn check_parts(doc: &mut Document) -> Result<(), String> {
    let mut lift = Vec::new();
    for n in &doc.nodes {
        let Some(u) = n.under.as_deref() else { continue };
        match doc.node(u) {
            Some(p) if p.group == n.group && n.group.is_some() => {}
            Some(_) => lift.push((n.id.clone(), None)),
            None => return Err(format!("no task `{u}` for `{}` to be part of", n.id)),
        }
    }
    for (id, to) in lift {
        doc.node_mut(&id).unwrap().under = to;
    }
    for n in &doc.nodes {
        let mut cur = n.under.as_deref();
        let mut steps = 0;
        while let Some(u) = cur {
            if u == n.id || steps > doc.nodes.len() {
                return Err(format!("`{}` cannot be part of a task that is part of it", n.id));
            }
            cur = doc.node(u).and_then(|p| p.under.as_deref());
            steps += 1;
        }
    }
    for c in &mut doc.canvases {
        check_parts(&mut c.doc)?;
    }
    Ok(())
}

/// "no node `apii`" → "no node `apii` (did you mean `api`?)": the closest
/// id on the board, when one is close.
fn suggest(doc: &Document, message: String) -> String {
    let Some(start) = message
        .find("no ")
        .and_then(|i| message[i..].find('`').map(|j| i + j + 1))
    else {
        return message;
    };
    let Some(len) = message[start..].find('`') else {
        return message;
    };
    let wrong = &message[start..start + len];
    let ids = doc
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .chain(doc.groups.iter().map(|g| g.id.as_str()))
        .chain(doc.edges.iter().map(|e| e.id.as_str()))
        .chain(doc.comments.iter().map(|c| c.id.as_str()));
    let best = ids
        .map(|id| (distance(wrong, id), id))
        .filter(|(d, id)| *d <= (wrong.len().max(id.len()) / 3).max(1))
        .min();
    match best {
        Some((_, id)) => format!("{message} (did you mean `{id}`?)"),
        None => message,
    }
}

/// Edit distance between two ids.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// A chart's start: None leaves it, Some(None) clears it.
fn chart_start(v: &Option<String>) -> Result<Option<Option<String>>, String> {
    opt(v)
        .map(|s| {
            s.map(|s| {
                crate::gantt::parse_date(&s)
                    .map(crate::gantt::format_date)
                    .ok_or_else(|| format!("`{s}` is not a date (YYYY-MM-DD)"))
            })
            .transpose()
        })
        .transpose()
}

/// Timeline fields of a node: "" clears the date.
fn widget_kind(w: &str) -> Result<crate::model::Widget, String> {
    crate::model::Widget::parse(w).ok_or_else(|| {
        format!(
            "`{w}` is not a screen component (use {})",
            crate::model::Widget::names()
        )
    })
}

fn device(d: &str) -> Result<crate::model::Device, String> {
    crate::model::Device::parse(d).ok_or_else(|| {
        format!(
            "unknown device `{d}` (use {})",
            crate::model::Device::names()
        )
    })
}

fn event(n: &mut Node, date: &Option<String>, important: &Option<bool>) -> Result<(), String> {
    if let Some(d) = opt(date) {
        if let Some(d) = &d {
            if crate::timeline::parse(d).is_none() {
                return Err(crate::timeline::date_error(d));
            }
        }
        n.date = d;
    }
    if let Some(i) = important {
        n.important = *i;
    }
    Ok(())
}

/// Gantt fields of a node: "" clears the start, 0 the duration, [] `after`.
fn task(
    n: &mut Node,
    start: &Option<String>,
    days: &Option<f64>,
    after: &Option<Vec<String>>,
    milestone: &Option<bool>,
    done: &Option<f64>,
    owner: &Option<String>,
) -> Result<(), String> {
    if let Some(s) = opt(start) {
        n.start = s
            .map(|s| {
                crate::gantt::parse_date(&s)
                    .map(crate::gantt::format_date)
                    .ok_or_else(|| format!("`{s}` is not a date (YYYY-MM-DD)"))
            })
            .transpose()?;
    }
    if let Some(d) = days {
        n.days = Some(*d).filter(|d| *d > 0.0);
    }
    if let Some(a) = after {
        n.after = a.iter().filter(|s| !s.is_empty()).cloned().collect();
    }
    if let Some(m) = milestone {
        n.milestone = *m;
    }
    if let Some(d) = done {
        if !(0.0..=100.0).contains(d) {
            return Err(format!("done is a percentage (0 to 100), not {d}"));
        }
        n.done = Some(*d).filter(|d| *d > 0.0);
    }
    if let Some(o) = opt(owner) {
        n.owner = o;
    }
    Ok(())
}

/// A chart's fields, from an add_group or update_group ("" chart: not a
/// chart any more).
fn chart_fields(
    g: &mut Group,
    chart: &Option<String>,
    stacked: &Option<bool>,
    horizontal: &Option<bool>,
    unit: &Option<String>,
    series: &Option<Vec<String>>,
    axis: &Option<String>,
) -> Result<(), String> {
    if let Some(c) = opt(chart) {
        g.chart = c
            .map(|c| {
                ChartKind::parse(&c)
                    .ok_or_else(|| format!("unknown chart type `{c}` (use {})", ChartKind::names()))
            })
            .transpose()?;
    }
    if let Some(v) = stacked {
        g.stacked = *v;
    }
    if let Some(v) = horizontal {
        g.horizontal = *v;
    }
    if let Some(u) = opt(unit) {
        g.unit = u;
    }
    if let Some(sr) = series {
        g.series = sr.clone();
    }
    if let Some(a) = opt(axis) {
        g.axis = a;
    }
    Ok(())
}

/// A risk matrix's levels: 3 to 5 (0 or none: the default, 5).
fn matrix_levels(v: &Option<u8>) -> Result<Option<u8>, String> {
    match v {
        None | Some(0) => Ok(None),
        Some(n @ 3..=5) => Ok(Some(*n)),
        Some(n) => Err(format!("a risk matrix has 3, 4 or 5 levels, not {n}")),
    }
}

/// A risk's probability and impact: 1 to 5, 0 clears.
fn risk(n: &mut Node, probability: &Option<u8>, impact: &Option<u8>) -> Result<(), String> {
    for (v, slot, what) in [
        (probability, &mut n.probability, "probability"),
        (impact, &mut n.impact, "impact"),
    ] {
        if let Some(v) = v {
            if *v > 5 {
                return Err(format!("{what} is a level from 1 to 5, not {v}"));
            }
            *slot = Some(*v).filter(|v| *v > 0);
        }
    }
    Ok(())
}

fn opt(v: &Option<String>) -> Option<Option<String>> {
    v.as_ref()
        .map(|s| if s.is_empty() { None } else { Some(s.clone()) })
}

fn color(c: &str) -> Result<String, String> {
    let c = c.trim().to_ascii_lowercase();
    if valid_color(&c) {
        Ok(c)
    } else {
        Err(format!(
            "unknown colour `{c}` (use {} or #rrggbb)",
            COLORS.join(", ")
        ))
    }
}

fn map_layout(s: &str) -> Result<MapLayout, String> {
    MapLayout::parse(s)
        .ok_or_else(|| format!("unknown mindmap layout `{s}` (use {})", MapLayout::names()))
}

fn shape(s: &str) -> Result<Shape, String> {
    Shape::parse(s).ok_or_else(|| format!("unknown shape `{s}` (use {})", Shape::names()))
}

/// Move arrow `id` to place `k`: among its sequence diagram's messages, or
/// among all arrows.
fn move_edge(doc: &mut Document, id: &str, k: usize) {
    let Some(from) = doc.edges.iter().position(|e| e.id == id) else {
        return;
    };
    let seq = doc
        .edge(id)
        .and_then(|e| crate::sequence::sequence_of(doc, e))
        .map(|g| g.id.clone());
    let peers: Vec<usize> = (0..doc.edges.len())
        .filter(|&i| {
            i != from
                && match &seq {
                    Some(g) => {
                        crate::sequence::sequence_of(doc, &doc.edges[i]).is_some_and(|s| s.id == *g)
                    }
                    None => true,
                }
        })
        .collect();
    let e = doc.edges.remove(from);
    // positions shift by one past the removed arrow
    let at = match peers.get(k) {
        Some(&p) => p - usize::from(p > from),
        None => peers
            .last()
            .map_or(from, |&p| p - usize::from(p > from) + 1),
    };
    doc.edges.insert(at.min(doc.edges.len()), e);
}

/// A decision record's status; "" is none (not a record).
fn adr_status(s: &Option<String>) -> Result<Option<String>, String> {
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(v) if crate::adr::valid_status(v) => Ok(Some(v.to_string())),
        Some(v) => Err(crate::adr::status_error(v)),
    }
}

/// A `canvas` value: a canvas of this board ("" or none: no canvas).
fn need_canvas(doc: &Document, canvas: &Option<String>) -> Result<Option<String>, String> {
    match canvas.as_deref().filter(|c| !c.is_empty()) {
        None => Ok(None),
        Some(c) if doc.canvas(c).is_some() => Ok(Some(c.to_string())),
        Some(c) => Err(format!("no canvas `{c}` (add it with add_canvas)")),
    }
}

/// An `image` value: an asset the board has ("" or none: no image).
fn need_asset(doc: &Document, image: &Option<String>) -> Result<Option<String>, String> {
    match image.as_deref().filter(|i| !i.is_empty()) {
        None => Ok(None),
        Some(i) if doc.asset(i).is_some() => Ok(Some(i.to_string())),
        Some(i) => Err(format!("no asset `{i}` (add it with add_asset)")),
    }
}

fn need_container(doc: &Document, g: &str) -> Result<(), String> {
    if doc.group(g).is_none() {
        return Err(format!("no group `{g}`"));
    }
    Ok(())
}

/// A forecast launch's numbers.
fn launch(n: &mut Node, adds: &Option<f64>, ramp: &Option<f64>) {
    if let Some(v) = adds {
        n.adds = Some(*v);
    }
    if let Some(r) = ramp {
        n.ramp = Some(*r).filter(|r| *r > 0.0);
    }
}

fn new_id(doc: &Document, wanted: Option<&str>, label: &str) -> Result<String, String> {
    match wanted.filter(|s| !s.is_empty()) {
        Some(id) => {
            let id = if crate::dsl::is_ident(id) {
                id.to_string()
            } else {
                slug(id)
            };
            if doc.id_taken(&id) {
                Err(format!("id `{id}` is already used"))
            } else {
                Ok(id)
            }
        }
        None => Ok(doc.fresh_id(&slug(if label.is_empty() { "node" } else { label }))),
    }
}

/// One op. Renaming a chart renames its rows too (`chart.q1` → `revenue.q1`).
fn apply_one(doc: &mut Document, op: &Op, created: &mut Vec<String>) -> Result<(), String> {
    let rows: Vec<String> = match op {
        Op::Rename { id, .. } if doc.group(id).is_some_and(|g| g.plotted()) => doc
            .nodes
            .iter()
            .filter(|n| {
                n.group.as_deref() == Some(id.as_str()) && n.id.starts_with(&format!("{id}."))
            })
            .map(|n| n.id.clone())
            .collect(),
        _ => Vec::new(),
    };
    let shown = |doc: &Document| -> Vec<String> {
        doc.nodes.iter().filter_map(|n| n.image.clone()).collect()
    };
    let opened = |doc: &Document| -> Vec<String> {
        doc.nodes.iter().filter_map(|n| n.canvas.clone()).collect()
    };
    let (before, links) = if matches!(op, Op::Remove { .. }) {
        (shown(doc), opened(doc))
    } else {
        (Vec::new(), Vec::new())
    };
    apply_op(doc, op, created)?;
    // an image no box shows, or a canvas no box opens, after a removal goes with it
    if !before.is_empty() {
        let now = shown(doc);
        doc.assets
            .retain(|a| !before.contains(&a.id) || now.contains(&a.id));
    }
    if !links.is_empty() {
        let now = opened(doc);
        doc.canvases
            .retain(|c| !links.contains(&c.id) || now.contains(&c.id));
    }
    if let Op::Rename { id, to } = op {
        for r in rows {
            let new = format!("{to}{}", &r[id.len()..]);
            if !doc.id_taken(&new) {
                apply_op(doc, &Op::Rename { id: r, to: new }, created)?;
            }
        }
    }
    Ok(())
}

fn apply_op(doc: &mut Document, op: &Op, created: &mut Vec<String>) -> Result<(), String> {
    match op {
        Op::AddNode {
            id,
            label,
            shape: sh,
            color: c,
            detail,
            group,
            link,
            image,
            canvas,
            tags,
            x,
            y,
            w,
            h,
            parent,
            index,
            mindmap,
            start,
            days,
            after,
            milestone,
            row,
            under,
            platform,
            date,
            important,
            widget,
            level,
            done,
            owner,
            adds,
            ramp,
            probability,
            impact,
            values,
            cells,
            column,
        } => {
            let id = new_id(doc, id.as_deref(), label)?;
            let parent = parent.as_deref().filter(|p| !p.is_empty());
            if let Some(p) = parent {
                if doc.node(p).is_none() {
                    return Err(format!("no node `{p}` to branch from"));
                }
            }
            if let Some(g) = group.as_deref().filter(|g| !g.is_empty()) {
                need_container(doc, g)?;
            }
            let mut n = Node::new(&id);
            n.label = if label == &id {
                String::new()
            } else {
                label.clone()
            };
            if let Some(s) = sh {
                n.shape = shape(s)?;
            }
            if let Some(c) = c.as_deref().filter(|c| !c.is_empty()) {
                n.color = Some(color(c)?);
            }
            n.detail = detail.clone().filter(|d| !d.is_empty());
            n.group = group.clone().filter(|g| !g.is_empty());
            n.link = link.clone().filter(|l| !l.is_empty());
            n.image = need_asset(doc, image)?;
            n.canvas = need_canvas(doc, canvas)?;
            n.tags = tags.clone();
            (n.x, n.y, n.w, n.h) = (*x, *y, *w, *h);
            if let Some(m) = mindmap.as_deref().filter(|m| !m.is_empty()) {
                n.mindmap = Some(map_layout(m)?);
            }
            task(&mut n, start, days, after, milestone, done, owner)?;
            if let Some(r) = opt(row) {
                n.row = r;
            }
            if let Some(u) = opt(under) {
                n.under = u;
            }
            if let Some(p) = platform {
                n.platform = crate::gantt::platform_id(p);
            }
            launch(&mut n, adds, ramp);
            event(&mut n, date, important)?;
            if let Some(w) = widget.as_deref().filter(|w| !w.is_empty()) {
                n.widget = Some(widget_kind(w)?);
            }
            if let Some(l) = level {
                n.level = (*l).min(8);
            }
            // a screen's components: `login.submit`, and drawn as something
            if let Some(scr) = n
                .group
                .as_deref()
                .and_then(|g| doc.group(g))
                .filter(|g| g.screen.is_some())
            {
                let prefix = format!("{}.", scr.id);
                if !n.id.starts_with(&prefix) {
                    let wanted = format!("{prefix}{}", n.id);
                    n.id = if doc.id_taken(&wanted) {
                        doc.fresh_id(&wanted)
                    } else {
                        wanted
                    };
                }
                n.widget.get_or_insert(crate::model::Widget::Text);
            }
            risk(&mut n, probability, impact)?;
            if let Some(v) = values {
                n.values = v.clone();
            }
            if let Some(c) = cells {
                n.cells = c.clone();
            }
            if let Some(c) = column {
                n.column = Some(c.clone()).filter(|c| *c != Column::default());
            }
            doc.nodes.push(n);
            if let Some(p) = parent {
                doc.set_parent(&id, Some(p), *index)?;
            } else if index.is_some() {
                // among the other members of its group (rows, tasks)
                doc.move_among_siblings(&id, *index);
            }
            created.push(id);
        }
        Op::UpdateNode {
            id,
            label,
            shape: sh,
            color: c,
            detail,
            group,
            link,
            image,
            canvas,
            tags,
            w,
            h,
            parent,
            index,
            mindmap,
            collapsed,
            start,
            days,
            after,
            milestone,
            row,
            under,
            platform,
            date,
            important,
            widget,
            level,
            done,
            owner,
            adds,
            ramp,
            probability,
            impact,
            values,
            cells,
            column,
        } => {
            if let Some(Some(g)) = opt(group) {
                need_container(doc, &g)?;
            }
            match (opt(parent), index) {
                (Some(p), _) => doc.set_parent(id, p.as_deref(), *index)?,
                (None, Some(i)) => {
                    if doc.node(id).is_none() {
                        return Err(format!("no node `{id}`"));
                    }
                    doc.move_among_siblings(id, Some(*i))
                }
                (None, None) => {}
            }
            if let Some(m) = opt(mindmap) {
                let m = m.map(|m| map_layout(&m)).transpose()?;
                let n = doc.node_mut(id).ok_or_else(|| format!("no node `{id}`"))?;
                if n.parent.is_some() && m.is_some() {
                    return Err(format!(
                        "`{id}` is a branch; only a map's centre has a layout"
                    ));
                }
                n.mindmap = m;
            }
            let n = doc.node_mut(id).ok_or_else(|| format!("no node `{id}`"))?;
            if let Some(l) = label {
                n.label = if l == id { String::new() } else { l.clone() };
            }
            if let Some(s) = sh {
                n.shape = shape(s)?;
            }
            if let Some(c) = opt(c) {
                n.color = c.map(|c| color(&c)).transpose()?;
            }
            if let Some(d) = opt(detail) {
                n.detail = d;
            }
            if let Some(g) = opt(group) {
                n.group = g;
            }
            if image.is_some() {
                let i = need_asset(doc, image)?;
                doc.node_mut(id).unwrap().image = i;
            }
            if canvas.is_some() {
                let c = need_canvas(doc, canvas)?;
                doc.node_mut(id).unwrap().canvas = c;
            }
            let n = doc.node_mut(id).unwrap();
            if let Some(l) = opt(link) {
                n.link = l;
            }
            if let Some(t) = tags {
                n.tags = t.clone();
            }
            if let Some(w) = w {
                n.w = Some(*w).filter(|v| *v > 0.0);
            }
            if let Some(h) = h {
                n.h = Some(*h).filter(|v| *v > 0.0);
            }
            if let Some(c) = collapsed {
                n.collapsed = *c;
            }
            task(n, start, days, after, milestone, done, owner)?;
            if let Some(r) = opt(row) {
                n.row = r;
            }
            if let Some(u) = opt(under) {
                n.under = u;
            }
            if let Some(p) = platform {
                n.platform = crate::gantt::platform_id(p);
            }
            launch(n, adds, ramp);
            event(n, date, important)?;
            if let Some(l) = level {
                n.level = (*l).min(8);
            }
            if let Some(w) = widget {
                n.widget = if w.is_empty() {
                    None
                } else {
                    Some(widget_kind(w)?)
                };
            }
            risk(n, probability, impact)?;
            if let Some(v) = values {
                n.values = v.clone();
            }
            if let Some(c) = cells {
                n.cells = c.clone();
            }
            if let Some(c) = column {
                n.column = Some(c.clone()).filter(|c| *c != Column::default());
            }
        }
        Op::AddOutline {
            parent,
            outline,
            index,
            x,
            y,
            group,
        } => {
            if let Some(g) = group.as_deref().filter(|g| !g.is_empty()) {
                need_container(doc, g)?;
            }
            let lines = crate::mindmap::parse_reply(outline);
            let at = match (x, y) {
                (Some(x), Some(y)) => Some((*x, *y)),
                _ => None,
            };
            let ids = crate::mindmap::insert_outline(
                doc,
                parent.as_deref().filter(|p| !p.is_empty()),
                &lines,
                *index,
                at,
            )?;
            // a new map goes into the section as a whole (topics follow their centre)
            if let (None, Some(g), Some(c)) = (
                parent.as_deref().filter(|p| !p.is_empty()),
                group.as_deref().filter(|g| !g.is_empty()),
                ids.first(),
            ) {
                if let Some(n) = doc.node_mut(c) {
                    n.group = Some(g.to_string());
                }
            }
            created.extend(ids);
        }
        Op::AddCanvas { id, title } => {
            let base = id
                .as_deref()
                .filter(|i| !i.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| slug(if title.is_empty() { "canvas" } else { title }));
            if id.as_deref().is_some_and(|i| !i.is_empty())
                && (!crate::dsl::is_ident(&base)
                    || doc.id_taken(&base)
                    || doc.canvas(&base).is_some())
            {
                return Err(format!("id `{base}` is already used or not an id"));
            }
            let mut cid = base.clone();
            let mut k = 2;
            while doc.id_taken(&cid) || doc.canvas(&cid).is_some() {
                cid = format!("{base}-{k}");
                k += 1;
            }
            doc.canvases.push(crate::model::Canvas {
                id: cid.clone(),
                doc: Document {
                    title: Some(title.clone()).filter(|t| !t.is_empty()),
                    ..Default::default()
                },
            });
            created.push(cid);
        }
        Op::Perspective {
            id,
            label,
            tags,
            show,
            details,
            remove,
        } => {
            if !crate::dsl::is_ident(id) {
                return Err(format!("`{id}` is not an id"));
            }
            if *remove {
                doc.perspectives.retain(|p| p.id != *id);
                return Ok(());
            }
            if let Some(bad) = show
                .iter()
                .flatten()
                .find(|k| !crate::perspectives::KINDS.contains(&k.as_str()))
            {
                return Err(format!(
                    "a perspective cannot show `{bad}` (use {})",
                    crate::perspectives::KINDS.join(", ")
                ));
            }
            // a built-in one is copied onto the board the first time it changes
            let base = crate::perspectives::all(doc)
                .into_iter()
                .find(|p| p.id == *id)
                .unwrap_or_else(|| crate::model::Perspective {
                    id: id.clone(),
                    label: id.clone(),
                    ..Default::default()
                });
            let mut p = doc
                .perspectives
                .iter()
                .find(|p| p.id == *id)
                .cloned()
                .unwrap_or(base);
            if let Some(l) = label.as_deref().filter(|l| !l.trim().is_empty()) {
                p.label = l.trim().to_string();
            }
            if let Some(t) = tags {
                p.tags = t
                    .iter()
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
            }
            if let Some(s) = show {
                p.show = s.clone();
            }
            if let Some(d) = details {
                p.details = *d;
            }
            match doc.perspectives.iter_mut().find(|x| x.id == *id) {
                Some(x) => *x = p,
                None => doc.perspectives.push(p),
            }
        }
        Op::Presentation {
            title,
            transition,
            instructions,
            slides,
            remove,
        } => {
            if *remove {
                doc.presentation = None;
                return Ok(());
            }
            let mut p = doc
                .presentation
                .clone()
                .unwrap_or_else(|| crate::model::Presentation {
                    title: doc.title.clone().unwrap_or_default(),
                    ..Default::default()
                });
            if let Some(t) = title {
                p.title = t.trim().to_string();
            }
            if let Some(t) = transition {
                p.transition = crate::model::Transition::parse(t).ok_or_else(|| {
                    format!(
                        "unknown transition `{t}` (use {})",
                        crate::model::Transition::names()
                    )
                })?;
            }
            if let Some(i) = instructions {
                p.instructions = Some(i.trim().to_string()).filter(|i| !i.is_empty());
            }
            if let Some(list) = slides {
                for s in list {
                    if s.target != "*"
                        && doc.node(&s.target).is_none()
                        && doc.group(&s.target).is_none()
                    {
                        return Err(format!("no `{}` on the board for a slide", s.target));
                    }
                }
                p.slides = list
                    .iter()
                    .cloned()
                    .map(|mut s| {
                        s.title = s.title.filter(|t| !t.trim().is_empty());
                        s.notes = s.notes.filter(|t| !t.trim().is_empty());
                        s
                    })
                    .collect();
            }
            doc.presentation = Some(p);
        }
        Op::In { canvas, ops } => {
            let path: Vec<String> = canvas
                .split('/')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(String::from)
                .collect();
            let inner = doc
                .at_mut(&path)
                .filter(|_| !path.is_empty())
                .ok_or_else(|| format!("no canvas `{canvas}`"))?;
            for (i, op) in ops.iter().enumerate() {
                apply_one(inner, op, created).map_err(|m| {
                    format!("in canvas `{canvas}`, op #{}: {}", i + 1, suggest(inner, m))
                })?;
            }
        }
        Op::UpdateCanvas { id, title } => {
            let c = doc
                .canvas_mut(id)
                .ok_or_else(|| format!("no canvas `{id}`"))?;
            c.doc.title = Some(title.clone()).filter(|t| !t.is_empty());
        }
        Op::AddAsset { id, data } => {
            let data = data.trim();
            if crate::model::Asset::media_type(data).is_none() {
                return Err("an asset is an image as a data URI: `data:image/png;base64,…`".into());
            }
            let id = match id.as_deref().filter(|i| !i.is_empty()) {
                Some(i) if !crate::dsl::is_ident(i) => return Err(format!("`{i}` is not an id")),
                Some(i) if doc.asset(i).is_some() => {
                    return Err(format!("asset `{i}` is already used"))
                }
                Some(i) => i.to_string(),
                None => doc.fresh_asset_id(),
            };
            doc.assets.push(crate::model::Asset {
                id: id.clone(),
                data: data.to_string(),
            });
            created.push(id);
        }
        Op::Rename { id, to } => {
            if id == to {
                return Ok(());
            }
            // a screen's component keeps the screen's prefix (`login.submit`)
            let prefix = doc
                .node(id)
                .and_then(|n| crate::screen::screen_of(doc, n))
                .map(|g| format!("{}.", g.id));
            if let Some(p) = prefix.filter(|p| !to.starts_with(p.as_str())) {
                let to = format!("{p}{to}");
                return apply_op(doc, &Op::Rename { id: id.clone(), to }, created);
            }
            if !crate::dsl::is_ident(to) {
                return Err(format!(
                    "`{to}` is not a valid id (letters, digits, _, - and .)"
                ));
            }
            if doc.id_taken(to) {
                return Err(format!("id `{to}` is already used"));
            }
            if let Some(n) = doc.node_mut(id) {
                n.id = to.clone();
                // gantt tasks that start after it
                for n in &mut doc.nodes {
                    for a in &mut n.after {
                        if a == id {
                            *a = to.clone();
                        }
                    }
                    if n.row.as_deref() == Some(id) {
                        n.row = Some(to.clone());
                    }
                    if n.under.as_deref() == Some(id) {
                        n.under = Some(to.clone());
                    }
                }
                for n in &mut doc.nodes {
                    if n.parent.as_deref() == Some(id) {
                        n.parent = Some(to.clone());
                    }
                }
            } else if let Some(g) = doc.group_mut(id) {
                g.id = to.clone();
                for n in &mut doc.nodes {
                    if n.group.as_deref() == Some(id) {
                        n.group = Some(to.clone());
                    }
                }
                for g in &mut doc.groups {
                    if g.parent.as_deref() == Some(id) {
                        g.parent = Some(to.clone());
                    }
                }
            } else if let Some(e) = doc.edge_mut(id) {
                e.id = to.clone();
            } else if let Some(c) = doc.comment(id).map(|c| c.id.clone()) {
                if doc.comment(to).is_some() {
                    return Err(format!("comment `{to}` already exists"));
                }
                doc.comment_mut(&c).unwrap().id = to.clone();
                return Ok(());
            } else {
                return Err(format!("nothing has the id `{id}`"));
            }
            // edges keep their auto ids in step with their endpoints
            let mut renamed: Vec<(String, String)> = Vec::new();
            for i in 0..doc.edges.len() {
                let e = &mut doc.edges[i];
                let mut touched = false;
                if e.from == *id {
                    e.from = to.clone();
                    touched = true;
                }
                if e.to == *id {
                    e.to = to.clone();
                    touched = true;
                }
                if touched && e.id.contains("->") {
                    let old = e.id.clone();
                    e.id = String::new();
                    let (f, t) = (doc.edges[i].from.clone(), doc.edges[i].to.clone());
                    let fresh = doc.fresh_edge_id(&f, &t);
                    doc.edges[i].id = fresh.clone();
                    renamed.push((old, fresh));
                }
            }
            for c in &mut doc.comments {
                for t in c.on.iter_mut().chain(c.also.iter_mut()) {
                    if t == id {
                        *t = to.clone();
                    } else if let Some((_, new)) = renamed.iter().find(|(old, _)| old == t) {
                        *t = new.clone();
                    }
                }
            }
        }
        Op::Remove { id, keep_members }
            if doc.element(id).is_none() && doc.themes.iter().any(|t| t.id == *id) =>
        {
            // a theme: boards using it go back to the default
            let _ = keep_members;
            doc.themes.retain(|t| t.id != *id);
            if doc.settings.palette.as_deref() == Some(id.as_str()) {
                doc.settings.palette = None;
            }
        }
        Op::Remove { id, keep_members } => {
            if doc
                .node(id)
                .is_some_and(|n| n.parent.is_some() || !doc.children(&n.id).is_empty())
            {
                let branch = doc.subtree(id);
                if keep_members.unwrap_or(false) {
                    let (up, at) = {
                        let n = doc.node(id).unwrap();
                        let at = n
                            .parent
                            .as_ref()
                            .and_then(|p| doc.children(p).iter().position(|c| c.id == *id));
                        (n.parent.clone(), at)
                    };
                    let kids: Vec<String> = doc.children(id).iter().map(|c| c.id.clone()).collect();
                    let (x, y) = doc.node(id).map(|n| (n.x, n.y)).unwrap();
                    for (k, kid) in kids.iter().enumerate() {
                        doc.set_parent(kid, up.as_deref(), at.map(|a| a + k))?;
                        if up.is_none() && k == 0 {
                            let n = doc.node_mut(kid).unwrap();
                            (n.x, n.y) = (x, y);
                        }
                    }
                    apply_one(
                        doc,
                        &Op::Remove {
                            id: id.clone(),
                            keep_members: None,
                        },
                        created,
                    )?;
                    return Ok(());
                }
                for nid in branch.iter().rev() {
                    doc.node_mut(nid).unwrap().parent = None;
                    doc.nodes.retain(|n| n.id != *nid);
                    let gone: Vec<String> = doc
                        .edges
                        .iter()
                        .filter(|e| e.from == *nid || e.to == *nid)
                        .map(|e| e.id.clone())
                        .collect();
                    doc.edges.retain(|e| e.from != *nid && e.to != *nid);
                    detach_comments(doc, nid, &gone);
                }
            } else if crate::gantt::has_parts(doc, id) {
                // a gantt group: its parts move up (keep_members), or go with it
                let up = doc.node(id).and_then(|n| n.under.clone());
                let parts: Vec<String> = doc
                    .nodes
                    .iter()
                    .filter(|n| n.under.as_deref() == Some(id.as_str()))
                    .map(|n| n.id.clone())
                    .collect();
                for p in parts {
                    if keep_members.unwrap_or(false) {
                        doc.node_mut(&p).unwrap().under = up.clone();
                    } else {
                        apply_one(
                            doc,
                            &Op::Remove {
                                id: p,
                                keep_members: None,
                            },
                            created,
                        )?;
                    }
                }
                doc.node_mut(id).unwrap().under = None;
                return apply_one(
                    doc,
                    &Op::Remove {
                        id: id.clone(),
                        keep_members: None,
                    },
                    created,
                );
            } else if doc.node(id).is_some() {
                doc.nodes.retain(|n| n.id != *id);
                let gone: Vec<String> = doc
                    .edges
                    .iter()
                    .filter(|e| e.from == *id || e.to == *id)
                    .map(|e| e.id.clone())
                    .collect();
                doc.edges.retain(|e| e.from != *id && e.to != *id);
                detach_comments(doc, id, &gone);
            } else if let Some(g) = doc.group(id).cloned() {
                // a section's boxes stay; a block's items (a chart's tasks, a
                // table's rows…) mean nothing without it, so they go with it
                let block = g.gantt
                    || g.risks
                    || g.chart.is_some()
                    || g.table
                    || g.timeline
                    || g.todo
                    || g.cycle
                    || g.entity
                    || g.adr.is_some()
                    || g.screen.is_some()
                    || g.forecast;
                let keep = keep_members.unwrap_or(!block);
                if keep {
                    for n in &mut doc.nodes {
                        if n.group.as_deref() == Some(id) {
                            n.group = g.parent.clone();
                        }
                    }
                    for c in &mut doc.groups {
                        if c.parent.as_deref() == Some(id) {
                            c.parent = g.parent.clone();
                        }
                    }
                } else {
                    let mut doomed_groups = vec![id.clone()];
                    let mut changed = true;
                    while changed {
                        changed = false;
                        for c in &doc.groups {
                            if c.parent.as_ref().is_some_and(|p| doomed_groups.contains(p))
                                && !doomed_groups.contains(&c.id)
                            {
                                doomed_groups.push(c.id.clone());
                                changed = true;
                            }
                        }
                    }
                    let doomed_nodes: Vec<String> = doc
                        .nodes
                        .iter()
                        .filter(|n| n.group.as_ref().is_some_and(|g| doomed_groups.contains(g)))
                        .map(|n| n.id.clone())
                        .collect();
                    for nid in doomed_nodes {
                        if doc.node(&nid).is_none() {
                            continue; // went with its map
                        }
                        apply_one(
                            doc,
                            &Op::Remove {
                                id: nid,
                                keep_members: None,
                            },
                            created,
                        )?;
                    }
                    for gid in doomed_groups.iter().filter(|g| *g != id) {
                        let gone: Vec<String> = doc
                            .edges
                            .iter()
                            .filter(|e| e.from == *gid || e.to == *gid)
                            .map(|e| e.id.clone())
                            .collect();
                        doc.edges.retain(|e| e.from != *gid && e.to != *gid);
                        detach_comments(doc, gid, &gone);
                    }
                    doc.groups
                        .retain(|c| c.id == *id || !doomed_groups.contains(&c.id));
                }
                doc.groups.retain(|c| c.id != *id);
                let gone: Vec<String> = doc
                    .edges
                    .iter()
                    .filter(|e| e.from == *id || e.to == *id)
                    .map(|e| e.id.clone())
                    .collect();
                doc.edges.retain(|e| e.from != *id && e.to != *id);
                detach_comments(doc, id, &gone);
            } else if doc.edge(id).is_some() {
                doc.edges.retain(|e| e.id != *id);
                detach_comments(doc, id, &[]);
            } else if doc.comment(id).is_some() {
                doc.comments.retain(|c| c.id != *id);
            } else if doc.canvas(id).is_some() {
                // a canvas goes with the boxes that open it
                doc.canvases.retain(|c| c.id != *id);
                let links: Vec<String> = doc
                    .nodes
                    .iter()
                    .filter(|n| n.canvas.as_deref() == Some(id.as_str()))
                    .map(|n| n.id.clone())
                    .collect();
                for l in links {
                    apply_op(
                        doc,
                        &Op::Remove {
                            id: l,
                            keep_members: None,
                        },
                        created,
                    )?;
                }
            } else if doc.asset(id).is_some() {
                doc.assets.retain(|a| a.id != *id);
                for n in &mut doc.nodes {
                    if n.image.as_deref() == Some(id.as_str()) {
                        n.image = None;
                    }
                }
            } else {
                return Err(format!("nothing has the id `{id}`"));
            }
        }
        Op::Move { id, x, y } => {
            // a mindmap topic's x/y are relative to its map's centre
            if let Some(n) = doc.node_mut(id) {
                n.x = Some(x.round());
                n.y = Some(y.round());
            } else if doc.group(id).is_some() {
                // a section or block moves as a whole; a lane moves with its swimlanes
                let lane = doc
                    .group(id)
                    .and_then(|g| g.parent.as_deref())
                    .and_then(|p| doc.group(p))
                    .is_some_and(|p| p.lanes);
                if lane {
                    return Err(format!("`{id}` is a lane: move its swimlanes"));
                }
                let g = doc.group_mut(id).unwrap();
                g.x = Some(x.round());
                g.y = Some(y.round());
            } else {
                return Err(format!("no node or group `{id}`"));
            }
        }
        Op::Nudge { id, dx, dy } => {
            let targets: Vec<String> = if doc.node(id).is_some() {
                vec![id.clone()]
            } else if doc.group(id).is_some() {
                doc.nodes
                    .iter()
                    .filter(|n| {
                        n.group
                            .as_ref()
                            .is_some_and(|g| doc.group_chain(g).contains(id))
                    })
                    .map(|n| n.id.clone())
                    .collect()
            } else {
                return Err(format!("no node or group `{id}`"));
            };
            // pinned swimlanes and gantt charts in it move too
            let inside: Vec<String> = doc
                .groups
                .iter()
                .filter(|g| doc.group_chain(&g.id).contains(id))
                .map(|g| g.id.clone())
                .collect();
            for g in &mut doc.groups {
                if inside.contains(&g.id) {
                    if let (Some(x), Some(y)) = (g.x, g.y) {
                        g.x = Some((x + dx).round());
                        g.y = Some((y + dy).round());
                    }
                }
            }
            for t in targets {
                let n = doc.node_mut(&t).unwrap();
                if let (Some(x), Some(y)) = (n.x, n.y) {
                    n.x = Some((x + dx).round());
                    n.y = Some((y + dy).round());
                }
            }
        }
        Op::Unpin { id } => {
            for n in &mut doc.nodes {
                if id.as_ref().is_none_or(|id| *id == n.id) {
                    n.x = None;
                    n.y = None;
                }
            }
            for g in &mut doc.groups {
                if id.as_ref().is_none_or(|id| *id == g.id) {
                    g.x = None;
                    g.y = None;
                }
            }
        }
        Op::AddEdge {
            id,
            from,
            to,
            label,
            arrow,
            line,
            color: c,
        } => {
            for end in [from, to] {
                if doc.node(end).is_none() && doc.group(end).is_none() {
                    return Err(format!("no node or group `{end}` (add it first)"));
                }
            }
            let id = match id.as_deref().filter(|s| !s.is_empty()) {
                Some(id) if doc.id_taken(id) => return Err(format!("id `{id}` is already used")),
                Some(id) => id.to_string(),
                None => doc.fresh_edge_id(from, to),
            };
            let mut e = Edge {
                id: id.clone(),
                from: from.clone(),
                to: to.clone(),
                label: label.clone().filter(|l| !l.is_empty()),
                ..Default::default()
            };
            if let Some(a) = arrow {
                e.arrow = Arrow::parse(a)
                    .ok_or_else(|| format!("unknown arrow `{a}` (use {})", Arrow::names()))?;
            }
            if let Some(l) = line {
                e.line = Line::parse(l)
                    .ok_or_else(|| format!("unknown line `{l}` (use {})", Line::names()))?;
            }
            if let Some(c) = c.as_deref().filter(|c| !c.is_empty()) {
                e.color = Some(color(c)?);
            }
            doc.edges.push(e);
            created.push(id);
        }
        Op::UpdateEdge {
            id,
            from,
            to,
            label,
            arrow,
            line,
            color: c,
            index,
        } => {
            for end in [from, to].into_iter().flatten() {
                if doc.node(end).is_none() && doc.group(end).is_none() {
                    return Err(format!("no node or group `{end}`"));
                }
            }
            let e = doc.edge_mut(id).ok_or_else(|| format!("no edge `{id}`"))?;
            // an automatic id (`from->to`) follows the edge's new ends
            let auto = *id == format!("{}->{}", e.from, e.to);
            if let Some(f) = from {
                e.from = f.clone();
            }
            if let Some(t) = to {
                e.to = t.clone();
            }
            let (f, t) = (e.from.clone(), e.to.clone());
            let mut cur = id.clone();
            if auto && *id != format!("{f}->{t}") {
                let fresh = doc.fresh_edge_id(&f, &t);
                for c in &mut doc.comments {
                    for t in c.on.iter_mut().chain(c.also.iter_mut()) {
                        if t == id {
                            *t = fresh.clone();
                        }
                    }
                }
                doc.edge_mut(id).unwrap().id = fresh.clone();
                cur = fresh;
            }
            let e = doc.edge_mut(&cur).unwrap();
            if let Some(l) = opt(label) {
                e.label = l;
            }
            if let Some(a) = arrow {
                e.arrow = Arrow::parse(a)
                    .ok_or_else(|| format!("unknown arrow `{a}` (use {})", Arrow::names()))?;
            }
            if let Some(l) = line {
                e.line = Line::parse(l)
                    .ok_or_else(|| format!("unknown line `{l}` (use {})", Line::names()))?;
            }
            if let Some(c) = opt(c) {
                e.color = c.map(|c| color(&c)).transpose()?;
            }
            if let Some(k) = index {
                move_edge(doc, &cur, *k);
            }
        }
        Op::AddGroup {
            id,
            label,
            color: c,
            tags,
            parent,
            members,
            lanes,
            kanban,
            sequence,
            gantt,
            start,
            risks,
            levels,
            chart,
            stacked,
            horizontal,
            unit,
            series,
            axis,
            table,
            timeline,
            cycle,
            todo,
            screen,
            columns,
            entity,
            forecast,
            now,
            schema,
            file,
            only,
            adr,
        } => {
            let id = new_id(
                doc,
                id.as_deref(),
                if label.is_empty() { "group" } else { label },
            )?;
            if let Some(p) = parent.as_deref().filter(|p| !p.is_empty()) {
                need_container(doc, p)?;
            }
            let mut g = Group {
                id: id.clone(),
                label: if *label == id {
                    String::new()
                } else {
                    label.clone()
                },
                parent: parent.clone().filter(|p| !p.is_empty()),
                lanes: *lanes || *kanban,
                kanban: *kanban,
                sequence: *sequence,
                gantt: *gantt,
                start: chart_start(start)?.flatten(),
                risks: *risks,
                levels: matrix_levels(levels)?,
                ..Default::default()
            };
            chart_fields(&mut g, chart, stacked, horizontal, unit, series, axis)?;
            g.table = table.unwrap_or(false) || columns.is_some();
            g.timeline = timeline.unwrap_or(false);
            g.cycle = cycle.unwrap_or(false);
            g.todo = todo.unwrap_or(false);
            g.screen = match screen.as_deref().filter(|d| !d.is_empty()) {
                Some(d) => Some(device(d)?),
                None => None,
            };
            g.entity = entity.unwrap_or(false);
            g.forecast = forecast.unwrap_or(false);
            g.now = *now;
            g.schema = schema.unwrap_or(false);
            g.file = file.clone().filter(|f| !f.is_empty());
            g.only = only.clone().filter(|o| !o.is_empty());
            g.adr = adr_status(adr)?;
            g.columns = columns.clone().unwrap_or_default();
            if let Some(c) = c.as_deref().filter(|c| !c.is_empty()) {
                g.color = Some(color(c)?);
            }
            g.tags = tags.clone().unwrap_or_default();
            doc.groups.push(g);
            for m in members {
                if let Some(n) = doc.node_mut(m) {
                    n.group = Some(id.clone());
                } else if doc.group(m).is_some() && m != &id {
                    doc.group_mut(m).unwrap().parent = Some(id.clone());
                } else {
                    return Err(format!("no node or group `{m}` to put in `{id}`"));
                }
            }
            created.push(id);
        }
        Op::UpdateGroup {
            id,
            label,
            color: c,
            tags,
            parent,
            lanes,
            kanban,
            sequence,
            gantt,
            start,
            scale,
            levels,
            chart,
            stacked,
            horizontal,
            unit,
            series,
            axis,
            table,
            timeline,
            cycle,
            todo,
            screen,
            columns,
            entity,
            forecast,
            now,
            schema,
            file,
            only,
            adr,
        } => {
            if let Some(Some(p)) = opt(parent) {
                need_container(doc, &p)?;
                if doc.group_chain(&p).contains(id) {
                    return Err(format!("`{p}` is inside `{id}`"));
                }
            }
            let g = doc
                .group_mut(id)
                .ok_or_else(|| format!("no group `{id}`"))?;
            if let Some(l) = label {
                g.label = l.clone();
            }
            if let Some(c) = opt(c) {
                g.color = c.map(|c| color(&c)).transpose()?;
            }
            if let Some(t) = tags {
                g.tags = t.clone();
            }
            if let Some(p) = opt(parent) {
                g.parent = p;
            }
            if let Some(l) = lanes {
                g.lanes = *l;
            }
            if let Some(k) = kanban {
                g.kanban = *k;
                g.lanes |= *k;
            }
            if let Some(s) = sequence {
                g.sequence = *s;
            }
            if let Some(v) = gantt {
                g.gantt = *v;
            }
            if let Some(s) = chart_start(start)? {
                g.start = s;
            }
            if levels.is_some() {
                g.levels = matrix_levels(levels)?;
            }
            chart_fields(g, chart, stacked, horizontal, unit, series, axis)?;
            if let Some(t) = table {
                g.table = *t;
            }
            if let Some(t) = timeline {
                g.timeline = *t;
            }
            if let Some(c) = cycle {
                g.cycle = *c;
            }
            if let Some(t) = todo {
                g.todo = *t;
            }
            if let Some(d) = screen {
                g.screen = if d.is_empty() { None } else { Some(device(d)?) };
            }
            if let Some(c) = columns {
                g.columns = c.clone();
            }
            if let Some(e) = entity {
                g.entity = *e;
            }
            if let Some(v) = forecast {
                g.forecast = *v;
            }
            if let Some(v) = now {
                g.now = Some(*v);
            }
            if let Some(v) = schema {
                g.schema = *v;
            }
            if let Some(f) = file {
                g.file = Some(f.clone()).filter(|f| !f.is_empty());
            }
            if let Some(o) = only {
                g.only = Some(o.clone()).filter(|o| !o.is_empty());
            }
            if adr.is_some() {
                g.adr = adr_status(adr)?;
            }
            if let Some(v) = scale {
                g.scale = if v.is_empty() {
                    None
                } else {
                    Some(Scale::parse(v).ok_or_else(|| {
                        format!("unknown scale `{v}` (day, week, month, quarter)")
                    })?)
                };
            }
        }
        Op::AddComment {
            id,
            on,
            also,
            text,
            author,
            x,
            y,
        } => {
            let on = on.clone().filter(|s| !s.is_empty());
            let mut more: Vec<String> = Vec::new();
            for a in also.iter().flatten().filter(|a| !a.is_empty()) {
                if on.as_deref() != Some(a.as_str()) && !more.contains(a) {
                    more.push(a.clone());
                }
            }
            if on.is_none() && !more.is_empty() {
                return Err("a comment on several elements names the first in `on`".into());
            }
            for t in on.iter().chain(&more) {
                if doc.element(t).is_none() {
                    return Err(format!("nothing has the id `{t}` to comment on"));
                }
            }
            let id = match id.as_deref().filter(|s| !s.is_empty()) {
                Some(id) if doc.comment(id).is_some() => {
                    return Err(format!("comment `{id}` already exists"))
                }
                Some(id) => id.to_string(),
                None => doc.fresh_comment_id(),
            };
            doc.comments.push(Comment {
                id: id.clone(),
                on,
                also: more,
                text: text.clone(),
                author: author.clone().filter(|s| !s.is_empty()),
                x: *x,
                y: *y,
                ..Default::default()
            });
            created.push(id);
        }
        Op::Reply { id, text, author } => {
            let c = doc
                .comment_mut(id)
                .ok_or_else(|| format!("no comment `{id}`"))?;
            c.replies.push(Reply {
                author: author.clone().filter(|s| !s.is_empty()),
                text: text.clone(),
            });
        }
        Op::EditComment { id, text } => {
            doc.comment_mut(id)
                .ok_or_else(|| format!("no comment `{id}`"))?
                .text = text.clone();
        }
        Op::ResolveComment { id, resolution } => {
            let c = doc
                .comment_mut(id)
                .ok_or_else(|| format!("no comment `{id}`"))?;
            c.status = Status::Resolved;
            if let Some(r) = opt(resolution) {
                c.resolution = r;
            }
        }
        Op::ReopenComment { id } => {
            let c = doc
                .comment_mut(id)
                .ok_or_else(|| format!("no comment `{id}`"))?;
            c.status = Status::Open;
        }
        Op::Theme { source } => {
            let parsed = crate::dsl::parse(source).map_err(|e| {
                e.iter()
                    .map(|d| format!("line {}: {}", d.line, d.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            if parsed.themes.is_empty() {
                return Err("no `theme id { … }` in `source`".into());
            }
            for t in parsed.themes {
                match doc.themes.iter_mut().find(|x| x.id == t.id) {
                    Some(x) => *x = t,
                    None => doc.themes.push(t),
                }
            }
        }
        Op::Merge { dsl } => {
            let base = crate::dsl::print(doc);
            // where the merged statements start in the text that is read
            let offset = base.lines().count() + 1;
            let merged = crate::dsl::parse(&format!("{base}\n{dsl}\n")).map_err(|errs| {
                errs.iter()
                    .map(|e| match e.line {
                        l if l > offset => format!("line {}: {}", l - offset, e.message),
                        _ => e.message.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            for n in &merged.nodes {
                if doc.node(&n.id).is_none() {
                    created.push(n.id.clone());
                }
            }
            for g in &merged.groups {
                if doc.group(&g.id).is_none() {
                    created.push(g.id.clone());
                }
            }
            for e in &merged.edges {
                if doc.edge(&e.id).is_none() {
                    created.push(e.id.clone());
                }
            }
            for c in &merged.comments {
                if doc.comment(&c.id).is_none() {
                    created.push(c.id.clone());
                }
            }
            *doc = merged;
        }
        Op::Set { key, value } => {
            let v = value.trim();
            let s = &mut doc.settings;
            match key.as_str() {
                "title" => doc.title = Some(v.to_string()).filter(|v| !v.is_empty()),
                "description" => doc.description = Some(v.to_string()).filter(|v| !v.is_empty()),
                "direction" => {
                    s.direction = Direction::parse(v).ok_or_else(|| {
                        format!("unknown direction `{v}` (use {})", Direction::names())
                    })?
                }
                "layout" => {
                    s.layout = LayoutKind::parse(v).ok_or_else(|| {
                        format!("unknown layout `{v}` (use {})", LayoutKind::names())
                    })?
                }
                "routing" => {
                    s.routing = Routing::parse(v).ok_or_else(|| {
                        format!("unknown routing `{v}` (use {})", Routing::names())
                    })?
                }
                "type" | "perspectives" => {
                    s.kind = BoardKind::parse(v).ok_or_else(|| {
                        format!("unknown set of perspectives `{v}` (use {})", BoardKind::names())
                    })?
                }
                "theme" => match Theme::parse(v) {
                    Some(t) => {
                        s.theme = t;
                        s.palette = None;
                    }
                    // a theme on the board, a preset or a file
                    None if !v.is_empty() => s.palette = Some(v.to_string()),
                    None => s.palette = None,
                },
                "rethought" => s.rethought = Some(v.to_string()).filter(|v| !v.is_empty()),
                k => {
                    return Err(format!(
                        "unknown setting `{k}` (use {})",
                        crate::dsl::SETTINGS.join(", ")
                    ))
                }
            }
        }
    }
    Ok(())
}

/// Comments on a removed element stay, as canvas comments, so feedback is
/// never lost silently.
fn detach_comments(doc: &mut Document, id: &str, edges: &[String]) {
    let gone = |t: &str| t == id || edges.iter().any(|e| e == t);
    for c in &mut doc.comments {
        // about several: it stays on the others
        c.also.retain(|t| !gone(t));
        if c.on.as_deref().is_some_and(gone) && !c.also.is_empty() {
            c.on = Some(c.also.remove(0));
            continue;
        }
        if c.on
            .as_deref()
            .is_some_and(|on| on == id || edges.iter().any(|e| e == on))
        {
            let what = c.on.take().unwrap();
            c.text = format!("[on removed `{what}`] {}", c.text);
        }
    }
}

/// Read ops from JSON: `{"ops": [...]}`, `[...]` or a single op. Accepts an
/// LLM reply too: the first ```json fenced block (or bare JSON) is used.
pub fn parse_ops(text: &str) -> Result<Vec<Op>, String> {
    let body = extract_json(text).ok_or("no JSON found (expected {\"ops\": [...]})")?;
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("invalid JSON: {e}"))?;
    let list = match v {
        serde_json::Value::Array(a) => a,
        // a batch (`{"ops": [...]}`), not an op with ops of its own (`in`)
        serde_json::Value::Object(mut o) if o.contains_key("ops") && !o.contains_key("op") => {
            match o.remove("ops") {
                Some(serde_json::Value::Array(a)) => a,
                _ => return Err("`ops` must be an array".into()),
            }
        }
        o @ serde_json::Value::Object(_) => vec![o],
        _ => return Err("expected an object or an array of ops".into()),
    };
    list.into_iter()
        .enumerate()
        .map(|(i, v)| serde_json::from_value::<Op>(v).map_err(|e| format!("op #{}: {e}", i + 1)))
        .collect()
}

/// The JSON payload inside a chat reply: a fenced block or the outermost braces.
pub fn extract_json(text: &str) -> Option<&str> {
    for fence in ["```json", "```JSON", "```"] {
        if let Some(start) = text.find(fence) {
            let rest = &text[start + fence.len()..];
            if let Some(end) = rest.find("```") {
                let body = rest[..end].trim();
                if body.starts_with('{') || body.starts_with('[') {
                    return Some(body);
                }
            }
        }
    }
    let t = text.trim();
    if t.starts_with('{') || t.starts_with('[') {
        return Some(t);
    }
    let start = t.find(['{', '['])?;
    let end = t.rfind(['}', ']'])?;
    (end > start).then(|| &t[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl;

    fn doc() -> Document {
        dsl::parse("group b: Backend {\n api\n db\n}\nweb -> api -> db\ncomment on api: split?\ncomment on api->db: index?").unwrap()
    }

    #[test]
    fn removing_a_section_with_its_contents_leaves_nothing_behind() {
        let mut d = dsl::parse("group s: Section {\n  mindmap m: Map {\n    A\n      B\n  }\n  lanes o: Order {\n    lane c: Customer {\n      x: Place\n    }\n  }\n}\nkeep: Keep\nkeep -> c\nkeep -> b\ncomment on c: Hm").unwrap();
        apply(
            &mut d,
            &parse_ops(r#"{"op":"remove","id":"s","keep_members":false}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(
            dsl::print(&d).trim_end(),
            "keep: Keep\n\ncomment c1: [on removed `c`] Hm",
            "{}",
            dsl::print(&d)
        );
    }

    #[test]
    fn add_connect_and_resolve() {
        let mut d = doc();
        let ops = parse_ops(
            r#"Here you go:
```json
{"ops": [
  {"op": "add_node", "label": "Redis cache", "shape": "cylinder", "group": "b"},
  {"op": "add_edge", "from": "api", "to": "redis-cache", "label": "reads"},
  {"op": "resolve_comment", "id": "c1", "resolution": "kept as one service"}
]}
```"#,
        )
        .unwrap();
        let r = apply(&mut d, &ops).unwrap();
        assert_eq!(r.created, vec!["redis-cache", "api->redis-cache"]);
        assert_eq!(d.node("redis-cache").unwrap().group.as_deref(), Some("b"));
        assert_eq!(d.comment("c1").unwrap().status, Status::Resolved);
        assert!(d.check().is_empty(), "{:?}", d.check());
    }

    #[test]
    fn batches_are_atomic() {
        let mut d = doc();
        let before = d.clone();
        let ops =
            parse_ops(r#"[{"op":"add_node","id":"x"},{"op":"add_edge","from":"x","to":"nope"}]"#)
                .unwrap();
        let err = apply(&mut d, &ops).unwrap_err();
        assert_eq!(err.index, 1);
        assert_eq!(d, before);
    }

    #[test]
    fn rename_updates_references() {
        let mut d = doc();
        apply(
            &mut d,
            &[Op::Rename {
                id: "api".into(),
                to: "orders".into(),
            }],
        )
        .unwrap();
        assert!(d.edge("web->orders").is_some() && d.edge("orders->db").is_some());
        assert_eq!(d.comment("c1").unwrap().on.as_deref(), Some("orders"));
        assert_eq!(d.comment("c2").unwrap().on.as_deref(), Some("orders->db"));
        assert!(d.check().is_empty(), "{:?}", d.check());
    }

    #[test]
    fn remove_keeps_feedback() {
        let mut d = doc();
        apply(
            &mut d,
            &[Op::Remove {
                id: "api".into(),
                keep_members: None,
            }],
        )
        .unwrap();
        assert_eq!(d.edges.len(), 0);
        assert!(d
            .comments
            .iter()
            .all(|c| c.on.is_none() && c.text.starts_with("[on removed")));
        let mut d = doc();
        apply(
            &mut d,
            &[Op::Remove {
                id: "b".into(),
                keep_members: Some(false),
            }],
        )
        .unwrap();
        assert!(d.node("api").is_none() && d.group("b").is_none() && d.node("web").is_some());
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(parse_ops(r#"{"op":"add_node","colour":"red"}"#).is_err());
    }

    #[test]
    fn reversing_an_edge_keeps_its_id_automatic() {
        let mut d = crate::dsl::parse("a: A\nb: B\na -> b\ncomment on a->b: why?").unwrap();
        apply(
            &mut d,
            &parse_ops(r#"{"op":"update_edge","id":"a->b","from":"b","to":"a"}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(d.edges[0].id, "b->a");
        assert_eq!(d.comments[0].on.as_deref(), Some("b->a"));
        assert!(crate::dsl::print(&d).contains("b -> a\n"));
    }

    #[test]
    fn gantt_ops() {
        let mut d =
            crate::dsl::parse("gantt g: Plan [start: 2026-10-12] {\n  Design [5d]\n}").unwrap();
        apply(
            &mut d,
            &parse_ops(
                r#"[{"op":"add_node","label":"Build","group":"g","days":10,"after":["design"]},
              {"op":"update_group","id":"g","start":"2026-10-19"},
              {"op":"rename","id":"design","to":"ux"}]"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            crate::dsl::print(&d),
            "gantt g: Plan [start: 2026-10-19] {\n  Design [id: ux, 5d]\n  Build [10d, after: ux]\n}\n"
        );
        assert!(apply(
            &mut d,
            &parse_ops(r#"{"op":"update_node","id":"build","start":"2026-13-01"}"#).unwrap()
        )
        .is_err());
    }

    #[test]
    fn mindmap_ops() {
        let mut d =
            dsl::parse("mindmap m: Map {\n  A\n    a1\n  B\n}\ncomment on a1: why?").unwrap();
        let ops = parse_ops(
            r#"[
          {"op":"add_node","label":"First","parent":"m","index":0},
          {"op":"add_outline","parent":"b","outline":"- B\n  - b1\n    - deep\n  - b2"},
          {"op":"update_node","id":"a","collapsed":true},
          {"op":"rename","id":"b","to":"bee"}
        ]"#,
        )
        .unwrap();
        let r = apply(&mut d, &ops).unwrap();
        assert_eq!(r.created, vec!["first", "b1", "deep", "b2"]);
        let kids: Vec<&str> = d.children("m").iter().map(|n| n.id.as_str()).collect();
        assert_eq!(kids, vec!["first", "a", "bee"]);
        assert_eq!(d.node("b1").unwrap().parent.as_deref(), Some("bee"));
        // reparent with position; cycles are refused
        apply(
            &mut d,
            &parse_ops(r#"{"op":"update_node","id":"bee","parent":"first","index":0}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(d.node("bee").unwrap().parent.as_deref(), Some("first"));
        assert!(apply(
            &mut d,
            &parse_ops(r#"{"op":"update_node","id":"first","parent":"deep"}"#).unwrap()
        )
        .is_err());
        // a topic placed by hand (relative to its centre)
        apply(
            &mut d,
            &parse_ops(r#"{"op":"move","id":"a1","x":1,"y":2}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(d.node("a1").unwrap().x, Some(1.0));
        // removing a topic takes its branch; feedback stays
        apply(&mut d, &parse_ops(r#"{"op":"remove","id":"a"}"#).unwrap()).unwrap();
        assert!(d.node("a1").is_none());
        assert!(d.comments[0].on.is_none());
        // keep_members lifts the branches up a level, in place
        apply(
            &mut d,
            &parse_ops(r#"{"op":"remove","id":"bee","keep_members":true}"#).unwrap(),
        )
        .unwrap();
        let kids: Vec<&str> = d.children("first").iter().map(|n| n.id.as_str()).collect();
        assert_eq!(kids, vec!["b1", "b2"]);
        assert!(d.check().is_empty(), "{:?}", d.check());
        // a new map from an outline
        let r = apply(
            &mut d,
            &parse_ops(r#"{"op":"add_outline","outline":"Ideas\n  One\n  Two","x":500,"y":0}"#)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(r.created[0], "ideas");
        assert_eq!(d.node("ideas").unwrap().mindmap, Some(MapLayout::Right));
        assert_eq!(d.node("ideas").unwrap().x, Some(500.0));
    }

    #[test]
    fn images_are_assets_shown_by_boxes() {
        const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let mut d = crate::dsl::parse("title: T\nnote: Hi\n").unwrap();
        let r = apply(
            &mut d,
            &parse_ops(&format!(
                r#"{{"ops":[{{"op":"add_asset","data":"{PNG}"}},{{"op":"add_node","id":"shot","label":"Login","image":"img1","w":120,"h":80}}]}}"#
            ))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(r.created, ["img1", "shot"]);
        let text = crate::dsl::print(&d);
        assert!(
            text.contains("shot: Login [image: img1, w: 120, h: 80]\n"),
            "{text}"
        );
        assert!(text.ends_with(&format!("\nasset img1: {PNG}\n")), "{text}");
        assert_eq!(crate::dsl::parse(&text).unwrap(), d);
        // what a model reads leaves the data out
        let lean = crate::dsl::print_lean(&d);
        assert!(
            !lean.contains("base64")
                && lean.contains("# images, not shown: img1 (image/png, 1 KB)"),
            "{lean}"
        );
        assert!(!crate::prompt::brief(&d).contains("base64"));
        // a reply rewritten without the asset keeps the board's
        let mut reply = crate::dsl::parse("shot: Login screen [image: img1]\n").unwrap();
        reply.adopt_hidden(&d);
        assert_eq!(reply.assets, d.assets);
        // an unknown asset is refused; not an image either
        assert!(apply(
            &mut d,
            &parse_ops(r#"{"op":"update_node","id":"note","image":"img9"}"#).unwrap()
        )
        .is_err());
        assert!(apply(
            &mut d,
            &parse_ops(r#"{"op":"add_asset","data":"data:text/html;base64,PGI+"}"#).unwrap()
        )
        .is_err());
        assert!(crate::dsl::parse("asset x: hello\n").is_err());
        // the box drawn with its picture
        let svg = crate::render::render(&d, &Default::default()).svg;
        assert!(svg.contains("<image ") && svg.contains(PNG));
        // removing the last box that shows it removes the image
        apply(
            &mut d,
            &parse_ops(r#"{"op":"remove","id":"shot"}"#).unwrap(),
        )
        .unwrap();
        assert!(d.assets.is_empty());
        // patching a file keeps the asset line at the end
        let src = format!("title: T\nshot: Login [image: img1]\n\nasset img1: {PNG}\n");
        let old = crate::dsl::parse(&src).unwrap();
        let mut new = old.clone();
        apply(
            &mut new,
            &parse_ops(r#"{"op":"add_node","id":"b","label":"B"}"#).unwrap(),
        )
        .unwrap();
        let out = crate::patch::patch(&src, &old, &new);
        assert!(
            out.ends_with(&format!("asset img1: {PNG}\n")) && out.contains("b: B"),
            "{out}"
        );
    }

    #[test]
    fn add_outline_keeps_ids_and_colours_and_goes_into_a_section() {
        let mut d = crate::dsl::parse("group s: Plan {\n  a: A\n}\n").unwrap();
        let r = apply(
            &mut d,
            &parse_ops(
                r#"{"op":"add_outline","group":"s","outline":"Launch [id: launch, red]\n  Who books the venue and when it happens [collapsed]\n  Budget [id: a, bogus: 1, green]"}"#,
            )
            .unwrap(),
        )
        .unwrap();
        // a free id is used; a taken one gets a fresh id; bad attrs are dropped one by one
        assert_eq!(r.created, ["launch", "who-books-the-venue", "budget"]);
        assert_eq!(d.node("launch").unwrap().color.as_deref(), Some("red"));
        assert_eq!(d.node("launch").unwrap().group.as_deref(), Some("s"));
        assert!(d.node("who-books-the-venue").unwrap().collapsed);
        assert_eq!(d.node("budget").unwrap().color.as_deref(), Some("green"));
        assert!(d.check().is_empty(), "{:?}", d.check());
        let text = crate::dsl::print(&d);
        assert_eq!(crate::dsl::parse(&text).unwrap(), d);
        // the section must exist
        assert!(apply(
            &mut d,
            &parse_ops(r#"{"op":"add_outline","group":"nope","outline":"X"}"#).unwrap()
        )
        .is_err());
    }

    #[test]
    fn merge_adds_and_changes_with_statements() {
        let mut d = crate::dsl::parse("group back: Backend {\n  api: Orders API\n}\n").unwrap();
        let r = apply(
            &mut d,
            &parse_ops(r#"{"ops":[{"op":"merge","dsl":"group back {\n  db: Postgres [cylinder]\n}\napi [red]\napi -> db: SQL"}]}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(r.created, ["db", "api->db"]);
        assert_eq!(d.node("db").unwrap().group.as_deref(), Some("back"));
        assert_eq!(d.node("api").unwrap().color.as_deref(), Some("red"));
        // errors point at the merged lines
        let e = apply(
            &mut d,
            &parse_ops(r#"{"ops":[{"op":"merge","dsl":"x\ny [shape: blob]"}]}"#).unwrap(),
        )
        .unwrap_err();
        assert!(
            e.message.starts_with("line 2: unknown shape"),
            "{}",
            e.message
        );
    }

    #[test]
    fn unknown_ids_get_a_suggestion() {
        let mut d = crate::dsl::parse("orders-api: Orders\ndb").unwrap();
        let e = apply(
            &mut d,
            &parse_ops(r#"{"op":"update_node","id":"order-api","color":"red"}"#).unwrap(),
        )
        .unwrap_err();
        assert!(
            e.message.ends_with("(did you mean `orders-api`?)"),
            "{}",
            e.message
        );
    }

    #[test]
    fn a_comment_can_be_about_several_elements() {
        let src = "api: Orders API\ndb: Database\nweb: Web app\n\ncomment c1 on api, db: Which one owns the orders?\n";
        let mut d = crate::dsl::parse(src).unwrap();
        assert_eq!(crate::dsl::print(&d), src);
        let c = d.comment("c1").unwrap();
        assert_eq!((c.on.as_deref(), c.also.clone()), (Some("api"), vec!["db".to_string()]));
        let ops = |j: &str| parse_ops(j).unwrap();
        apply(&mut d, &ops(r#"{"op":"add_comment","on":"web","also":["api","web"],"text":"Same?"}"#)).unwrap();
        assert_eq!(d.comment("c2").unwrap().also, ["api"]);
        assert!(apply(&mut d, &ops(r#"{"op":"add_comment","on":"web","also":["nope"],"text":"x"}"#)).is_err());
        // a rename follows it; a removed one leaves it on the others
        apply(&mut d, &ops(r#"{"op":"rename","id":"db","to":"store"}"#)).unwrap();
        assert_eq!(d.comment("c1").unwrap().also, ["store"]);
        apply(&mut d, &ops(r#"{"op":"remove","id":"api"}"#)).unwrap();
        let c1 = d.comment("c1").unwrap();
        assert_eq!((c1.on.as_deref(), c1.also.len()), (Some("store"), 0));
        assert_eq!(d.comment("c2").unwrap().also.len(), 0);
        assert!(crate::dsl::print(&d).contains("comment c1 on store: Which one"));
    }
}
