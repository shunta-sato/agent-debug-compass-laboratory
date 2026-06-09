# Linux Thermal Zone Adapter

Purpose: read Linux thermal zones for passive observation and load abort monitoring.

Allowed access:

- `/sys/class/thermal/thermal_zone*/temp`

Evidence boundary:

- Thermal readings are target-local observer data.
- Observer overhead and sampling cadence must be considered before making physical-footprint claims.
