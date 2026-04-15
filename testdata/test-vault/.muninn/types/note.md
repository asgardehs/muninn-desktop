---
name: note
description: A basic note
fields:
  title:
    type: string
    required: true
  tags:
    type: list
    items:
      type: string
  status:
    type: enum
    values:
      - active
      - done
      - archived
---
The default note type. All notes should have a title.
