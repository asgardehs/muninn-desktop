---
name: journal
description: Daily journal entry
extends: note
fields:
  date:
    type: date
    required: true
  mood:
    type: string
match:
  path_glob: "journal/*.md"
---
A daily journal entry type. Extends note with date and mood fields.
