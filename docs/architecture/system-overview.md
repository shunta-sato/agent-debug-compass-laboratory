# System Overview

`adc-lab` separates experiment governance from shell execution.

```text
adc-lab CLI
  controller / agent side
      |
      | local fixed calls or SSH fixed adc-lab-target calls
      v
adc-lab-target
  non-root target runner
      |
      | sudo helper call for allowlisted control only
      v
adc-lab-priv-helper
  root-owned typed helper
```

The controller writes run artifacts and audit events. The target runner performs read-only inventory, observation, health checks, and bounded non-root load. SSH runner selection is fixed to `adc-lab-target` or an allowlisted development path ending in `/adc-lab-target`; shell fragments are refused before SSH execution. The privileged helper only accepts typed JSON plans or restore leases.

In this MVP, privileged apply/restore is local-target only. Remote privileged apply is deferred until the helper can run on the target through an explicit target-local transport with target identity binding.

ADC Flight Recorder remains production-oriented and lightweight. `adc-lab` is explicitly experimental, bounded, and auditable.
