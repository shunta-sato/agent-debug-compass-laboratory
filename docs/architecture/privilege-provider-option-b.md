# Privilege Provider: Option B

Option B is the future root-owned systemd/Unix-socket privilege provider.

PR10 does not implement or enable it. PR10 only introduces
`lab.privilege_provider_status.v1` so a lab run can state which privilege
provider is active and which future provider is planned but disabled.

## Current PR10 State

Active provider:

```text
option_a_sudo_helper
```

Planned disabled provider:

```text
option_b_systemd_unix_socket
```

Planned endpoint identifier:

```text
/run/adc-lab/privileged.sock
```

This endpoint is not created by PR10. It is a design identifier only.

## Non-Goals In PR10

- no systemd service unit
- no systemd socket unit
- no Unix-domain socket listener
- no target-local root daemon
- no remote privileged apply
- no NOPASSWD sudoers change
- no new cpufreq write behavior
- no new privileged operation allowlist

## Future Option B Requirements

Before Option B can become active, a later PR must define and verify:

- bounded typed request/response protocol
- provider binary ownership and permissions
- systemd unit hardening profile
- socket permissions and group policy
- request authentication or local authorization boundary
- plan digest and approval binding at the provider boundary
- restore lease creation and verification at the provider boundary
- helper/provider version and digest in audit
- crash/restart behavior
- downgrade/fallback to Option A
- target-local physical-footprint evidence for any daemon or listener

## Safety Boundary

Option B must not become an Agent shell.

The provider must refuse:

- arbitrary shell commands
- arbitrary executable paths
- arbitrary sysfs paths
- arbitrary file writes
- unapproved or untyped operations
- operations without audit and restore policy

The only active PR10 privileged transport remains the fixed Option A sudo helper.

## Provider Status Command

PR10 adds:

```sh
adc-lab privilege provider-status --target local --json
```

The command writes:

```text
privilege/privilege_provider_status.json
```

and appends:

```text
operation=privilege.provider_status
```

to `audit.jsonl`.

The report is Tier 0 read-only evidence. It reports privilege posture only; it
does not install, start, or contact a privileged provider.
