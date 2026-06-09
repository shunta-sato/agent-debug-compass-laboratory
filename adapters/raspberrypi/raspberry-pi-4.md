# Raspberry Pi 4 Adapter Notes

Initial target class for `adc-lab`.

Expected Linux surfaces:

- procfs CPU and memory counters.
- sysfs thermal zones.
- cpufreq policy directories when the kernel exposes them.

MVP limitation:

- This repository bootstrap does not include live Pi4 characterization evidence.
- Target-specific budgets remain provisional until `embedded-target-characterization` is run.
