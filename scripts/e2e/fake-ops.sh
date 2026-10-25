#!/bin/sh
# A stand-in AI harness for the Ask AI test: replies with ops. A request with
# "twice" gets a reply that does not apply first (a wrong id), then a good one.
# A rethink gets a note and a section; a question an answer with links;
# drafting test scenarios a table of them (one more the second time); a
# rethink told what a newer Shape can do says so.
p=$(cat)
case "$p" in
  *"# Questions to plan the delivery of"*) printf '```json\n{"questions": [{"id": "start", "question": "When does the work start?", "hint": "a date"}, {"id": "team", "question": "Who is on the team?", "hint": "e.g. 2 web developers"}, {"id": "ga", "question": "Is there a fixed launch date?"}]}\n```\n' ;;
  *"# Plan the delivery of"*"2 Nov 2026"*) printf '```json\n{"note": "A plan from 2 Nov: build, beta and launch.", "ops": [{"op": "add_group", "id": "delivery", "label": "Delivery plan", "tags": ["delivery"]}, {"op": "merge", "dsl": "group delivery {\\n  gantt plan: Plan [start: 2026-11-02] {\\n    Widget\\n      Build [4w, platform: web]\\n      Beta [2w]\\n      Launch [milestone]\\n  }\\n  todo open: Open questions for the plan {\\n    [ ] Who is on the team?\\n  }\\n}"}]}\n```\n' ;;
  *"# A question about the board"*"Who owns the API"*) printf 'The **platform team** owns it ([[api]]); nothing on the board says who owns the database.\n' ;;
  *"# A question about the board"*) printf 'The API writes to Postgres.\n\n```shape\ntitle: Writes\nwriter: Orders API [link: #/api]\nstore: Postgres [link: #/db]\nwriter -> store: writes\n```\n' ;;
  *"# Draft test scenarios"*"Scenarios already written"*) printf '```shape\ntable t: Tests {\n  scenario | steps | expected | covers | status\n  Pay twice | 1. Pay\\n2. Pay again | Charged once | Checkout | to run\n}\n```\n' ;;
  *"# Draft test scenarios"*) printf '```shape\ntable t: Tests {\n  scenario | steps | expected | covers | status\n  Pay with a saved card | 1. Open the cart\\n2. Pay | Paid, receipt sent | Checkout | to run\n  Wrong password | 1. Sign in with a wrong password | Says it is wrong | Sign in | to run\n}\n```\n' ;;
  *"# Rethink the board"*"The answer was drawn as this small board"*) printf '```json\n{"note":"Took the answer in.","ops":[{"op":"update_node","id":"api","detail":"writes to Postgres"}]}\n```\n' ;;
  *"# Rethink the board"*"Adapt this canvas for senior management"*) printf '```json\n{"note":"Adapted for management.","ops":[{"op":"update_node","id":"api","detail":"Orders"}]}\n```\n' ;;
  *"# Rethink the board"*"Two plans now"*) printf '```json\n{"note":"Scope now has both plans.","ops":[{"op":"in","canvas":"notes","ops":[{"op":"update_node","id":"n1","label":"Two plans: free and team"},{"op":"resolve_comment","id":"c1","resolution":"Scope updated"}]}]}\n```\n' ;;
  *"# Rethink the board"*"## What Shape can do now"*) printf '```json\n{"note":"Put the storage in its own section, with what Shape can do now.","ops":[{"op":"add_group","id":"storage","label":"Storage","members":["db"]}]}\n```\n' ;;
  *"# Rethink the board"*) printf '```json\n{"note":"Put the storage in its own section.","ops":[{"op":"add_group","id":"storage","label":"Storage","members":["db"]}]}\n```\n' ;;
  *"could not be applied"*) printf '```json\n{"ops":[{"op":"update_node","id":"db","color":"red"}]}\n```\n' ;;
  *twice*) printf '```json\n{"ops":[{"op":"update_node","id":"dbb","color":"red"}]}\n```\n' ;;
  *) printf 'Here you go:\n```json\n{"ops":[{"op":"merge","dsl":"cache: Redis [cylinder]\\napi -> cache: reads"}]}\n```\n' ;;
esac
