# Rules

v2 reports are produced by Rust rule tables under `crates/adc-lab-core/src/rules`.
Rules bind stable claim IDs to predicate vocabulary, evidence kinds, status
decisions, and next-evidence guidance.

The claim catalog is the source for blocked-claim identity and scan terms.
`constraints check` should use catalog-backed terms instead of duplicating
free-form blocked-claim text.

Rule predicates remain typed and small: `Present`, `PressureEffect`,
`CompositeMeasured`, `LoadDurationAtLeastSeconds`, `NetworkBoundedTransfer`,
`ObservationSamplesAtLeast`, boolean composition, and catalog-registered
`Custom` predicates. The evidence predicates inspect artifact status and
payload evidence; artifact presence alone is not a measured claim boundary.
When three custom predicates share the same shape, promote that shape into core
predicate vocabulary.

`report operating-contract` writes `lab.artifact.v2` with
`kind = report.operating_contract`. With `--include-run`, all run directories
are opened by the v2 evidence store and evaluated together; no v1 run-set or
multi-run compatibility artifact is emitted.

`decide suitability` writes a v2 suitability artifact to the requested output
path. v1 decision sidecars are no longer part of the public CLI contract.
