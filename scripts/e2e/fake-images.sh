#!/bin/sh
# A stand-in AI harness for the images test: it says how many image files
# came along (in SHAPE_AI_IMAGES, and listed under "## Images" in the
# prompt), and whether they are really there, by adding a box saying so.
p=$(cat)
n=0
for f in $SHAPE_AI_IMAGES; do [ -s "$f" ] && n=$((n + 1)); done
case "$p" in
  *"## Images"*) listed=yes ;;
  *) listed=no ;;
esac
case "$p" in
  *"# A question about the board"*) printf 'Seen.\n\n```shape\ntitle: Seen\nseen: %s images, listed %s\n```\n' "$n" "$listed" ;;
  *) printf '```json\n{"ops":[{"op":"add_node","id":"seen","label":"%s images, listed %s"}]}\n```\n' "$n" "$listed" ;;
esac
