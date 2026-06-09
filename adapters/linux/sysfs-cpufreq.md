# Linux sysfs cpufreq Adapter

Purpose: discover and, through the privileged helper only, control CPU frequency governor policy.

Read-only surfaces:

- `/sys/devices/system/cpu/cpufreq/policy*/scaling_cur_freq`
- `/sys/devices/system/cpu/cpufreq/policy*/scaling_governor`

Privileged surfaces:

- `scaling_governor` writes for allowlisted `linux.cpufreq.set_governor`.

Safety boundary:

- Plans never include arbitrary sysfs paths.
- Helper discovers policy directories from the fixed cpufreq root.
- Restore leases capture per-policy governor and frequency bounds.
