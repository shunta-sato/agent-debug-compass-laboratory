# Claim Boundaries

Observed variation is not a controlled operating-point sweep.

Example:

```text
Observed:
  CPU frequency varied from 600MHz to 1800MHz under default policy.

Allowed claim:
  Frequency variation was observed under target default dynamic policy.

Blocked claim:
  adc-lab verified behavior across all fixed CPU frequencies.

Next evidence needed:
  controlled operating point matrix with fixed frequency or governor-controlled conditions.
```

`adc-lab report operating-point` creates provisional operating-point and capability-cost artifacts. Production-quality claims require target characterization and controlled evidence.
