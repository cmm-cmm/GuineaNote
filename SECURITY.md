# Security Policy

## Supported versions

Guinea Note is in preview. Only the latest release receives fixes.

| Version | Supported |
| --- | --- |
| 0.1.x (latest) | Yes |
| Older | No |

## Reporting a vulnerability

Please **do not open a public issue** for a vulnerability. Report it
privately through GitHub:
[Report a vulnerability](https://github.com/cmm-cmm/GuineaNote/security/advisories/new)
(Security tab → *Report a vulnerability*).

Include the version (`guinea --version`), the OS and terminal, the steps to
reproduce and the impact you expect. You should receive an answer within
7 days. Once a fix is released, the advisory is published with credit unless
you prefer to stay anonymous.

## Scope

Areas where a bug can affect security:

- **File safety:** saves that lose or alter bytes, writes outside the chosen
  path, following symlinks, or overwriting a file changed by another program
  (`crates/guinea-io`, [docs/FILE_SAFETY.md](docs/FILE_SAFETY.md)).
- **Crash recovery drafts:** the private local store of unsaved text and its
  permissions.
- **Agent access** (opt-in `--agent`): the local IPC authentication, the
  workspace and secret-path policy (`.env` and similar files are blocked
  unless `--agent-secrets` is given), and read-only leases
  (`crates/guinea-service`, [docs/AGENT_USAGE.md](docs/AGENT_USAGE.md)).
- **Terminal output:** control sequences in a file that escape the editor
  and reach the host terminal, and OSC 52 clipboard writes.
- **Release binaries:** archives that do not match `SHA256SUMS`.

Out of scope: problems that need an already compromised local account, and
denial of service from opening extremely large files (limits and prompts are
documented).
