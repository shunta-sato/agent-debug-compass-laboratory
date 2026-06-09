# Tool Qualification

`adc-lab` treats tools as first-class evidence sources.

Built-in read-only and bounded tools can be marked `builtin`. External or agent-created tools are not evidence sources until qualification evidence exists.

Agent-created tool path:

1. Missing tool is reported.
2. Agent proposes a manifest.
3. Agent implements a bounded adapter, probe, observer, or load tool.
4. Static safety review is recorded.
5. Dry-run is recorded.
6. Qualification report is created.
7. Bounded experiment uses the tool.
8. Evidence is accepted only when qualification status allows it.

MVP `adc-lab tool qualify` records manifest checks and keeps evidence rejected when dry-run or manual comparison evidence is missing.
