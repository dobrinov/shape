#!/bin/sh
# A stand-in AI harness that takes its time: for the test of the board
# waiting (locked, with what is going on) while the AI works.
cat > /dev/null
sleep 3
printf '```json\n{"note":"Grouped it.","ops":[{"op":"add_node","id":"done","label":"Thought it through"}]}\n```\n'
