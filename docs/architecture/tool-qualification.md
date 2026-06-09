# Tool Qualification

`adc-lab` treats tools as first-class evidence sources.

PR3 qualification is conservative. It classifies discovered tools from
`lab.toolchain_inventory.v1`; it does not execute external tools, run load
generators, perform control operations, or compare probe output.

Built-in read-only observation/probe/report/health tools can be marked
`builtin` and accepted as evidence when they are available and require no
privilege. External, agent-created, control-capable, load, privileged, or
missing tools are not evidence sources until later qualification evidence
exists.

Inventory qualification command:

```sh
adc-lab tool qualify-inventory --inventory lab/runs/LAB-RUN-.../toolchain/toolchain_inventory.json
```

Outputs:

- `tools/<tool-id>.qualification.json` for each discovered tool.
- `tools/tool_qualification_summary.json` with accepted, rejected, and missing
  tool ids.
- an audit event with operation `tool.qualify_inventory`.

PR3 statuses:

- `builtin`: accepted only for available, non-privileged, built-in read-only
  tools.
- `needs_control_test`: control-capable or privileged tools that need approval,
  restore, and verification evidence before supporting claims.
- `external_unqualified`: available external tools without dry-run, output
  validation, version/hash, and comparison evidence.
- `agent_created_unqualified`: manifest-only agent-created tools.
- `refused`: missing tools, load tools before bounded-load safety evidence, or
  tools outside the PR3 allowlist.

Agent-created tool path:

1. Missing tool is reported.
2. Agent proposes a manifest.
3. Agent implements a bounded adapter, probe, observer, or load tool.
4. Static safety review is recorded.
5. Dry-run is recorded.
6. Qualification report is created.
7. Bounded experiment uses the tool.
8. Evidence is accepted only when qualification status allows it.

MVP `adc-lab tool qualify` records manifest checks and keeps evidence rejected
when dry-run, manual comparison, version/hash, and output validation evidence is
missing.
