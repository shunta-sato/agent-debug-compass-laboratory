# Documentation Index

This index separates normative project contracts from supporting reference
material. If documents conflict, the normative set below wins.

## Normative

- [Safety model](architecture/safety-model.md): risk tiers, blocked operations,
  and control boundaries.
- [Evidence model](evidence-model.md): artifact envelope, store trust boundary,
  schema posture, and no-claim-without-audit rules.
- [Rules](rules.md): rule predicate vocabulary, claim decisions, and catalog
  ownership.
- [CLI reference](reference/cli.md): public command examples, output paths, and
  artifact-kind mapping.

## Reference

- [Pressure probes](reference/pressure-probes.md): pressure and composite probe
  semantics.
- [Pi5 controller to Pi4 target](getting-started/pi5-controller-pi4-target.md):
  release-binary and target setup workflow.
- [Install release binaries](getting-started/install-release-binaries.md):
  checksum-first install flow.
- [Privilege model Option A](architecture/privilege-model-option-a.md):
  fixed-helper privileged boundary.
- [Audit and reproducibility](architecture/audit-and-reproducibility.md):
  audit correlation and release identity.
- [Tool qualification](architecture/tool-qualification.md): evidence-source
  qualification gates.
- [Resource harness](testing/resource-harness.md): bounded resource smoke
  procedure.
- [Target runtime NFR matrix](nfr/adc-lab-target-runtime.md): embedded runtime
  assumptions and no-measurement-no-claim matrix.

## Architecture Notes

Architecture notes are reference-only unless listed in the normative set.
They preserve design context and must not override the current safety,
evidence, rules, or CLI contracts.
