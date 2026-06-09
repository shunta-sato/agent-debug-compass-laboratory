# AGENTS.md - adc-lab repository instructions

This repository does not vendor `.agents/` playbooks. Do not add generated
agent indexes that reference ignored `.agents/skills/...` paths.

## Project Rules

- Keep `adc-lab` as a safety-gated experiment laboratory, not an arbitrary
  shell wrapper.
- Preserve the North Star: no Agent root shell, no uncontrolled experiment, no
  unapproved hard-to-restore operation, no unqualified tool evidence, no claim
  without audit.
- Keep privileged operations typed, bounded, approved, audited, and restorable.
- Use the smallest safe change that satisfies the request.

## Verification

Use `COMMANDS.md` as the canonical command registry.

Default final gate:

```bash
make verify
```

If a command cannot be run, state why and provide a reproducible procedure.

## Planning

For complex or multi-step work, update the active ExecPlan under `plans/`.
Keep progress, decisions, surprises, handoff, and verification evidence current.

## Final Response Format

Return, in this order:

1. Change Brief (what/why, scope, assumptions, risks)
2. What changed (files + intent)
3. Verification (commands + results; or what could not be run)
4. Follow-ups (optional)
