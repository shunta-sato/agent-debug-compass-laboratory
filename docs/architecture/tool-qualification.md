# Tool Qualification

`adc-lab` treats tools as first-class evidence sources.

Tool qualification is conservative. It classifies discovered tools from
`lab.toolchain_inventory.v1`; it does not execute external tools, perform
control operations, or compare probe output.

Built-in read-only observation/probe/report/health tools can be marked
`builtin` and accepted as evidence when they are available and require no
privilege. Starting in PR5, the built-in `adc-lab-builtin-cpu-load` tool can be
marked `builtin` and accepted only for explicit Tier 1 bounded CPU load
`lab.load_plan.v1` / `lab.load_result.v1` evidence that includes duration,
worker, thermal, operator-abort, and safety-monitor fields.

External, agent-created, control-capable, privileged, missing, or non-allowlisted
load tools are not evidence sources until qualification evidence exists.

Inventory qualification command:

```sh
adc-lab tool qualify-inventory --inventory lab/runs/LAB-RUN-.../toolchain/toolchain_inventory.json
```

Outputs:

- `tools/<tool-id>.qualification.json` for each discovered tool.
- `tools/tool_qualification_summary.json` with accepted, rejected, and missing
  tool ids.
- an audit event with operation `tool.qualify_inventory`.

Statuses:

- `builtin`: accepted only for available, non-privileged, built-in read-only
  tools, plus the PR5 built-in bounded CPU load tool for bounded load result
  evidence.
- `needs_control_test`: control-capable or privileged tools that need approval,
  restore, and verification evidence before supporting claims.
- `external_unqualified`: available external tools without dry-run, output
  validation, version/hash, and comparison evidence.
- `agent_created_unqualified`: manifest-only agent-created tools.
- `refused`: missing tools or tools outside the current evidence allowlist.

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
