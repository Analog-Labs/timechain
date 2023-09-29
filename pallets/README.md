# Pallets

```mermaid
---
title: Relationships
---
stateDiagram-v2
    Shards --> Tasks: updates
    Tasks --> Shards: uses
    Members --> Elections: updates
    Elections --> Members: uses
    Elections --> Shards: uses
    Members --> Shards: updates
    Shards --> Members: uses
```
