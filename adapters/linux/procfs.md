# Linux procfs Adapter

Purpose: read-only Linux process and kernel counters for target inventory and passive observation.

Allowed access:

- `/proc/stat` for CPU tick counters.
- `/proc/meminfo` for memory totals and availability.

Evidence boundary:

- procfs readings are observation evidence when sampled through bounded `adc-lab observe`.
- procfs readings do not prove controlled operating points by themselves.
