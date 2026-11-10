# Dependency audit — 2026-11-09

Tool: cargo-deny 0.20.2, installed from its locked source package into
`target/devtools`. It is a development tool, not an application dependency or
global PATH change. Project Cargo.lock remains the application dependency lock.

Initial audit passed advisories, licenses and sources but failed the existing
`wildcards = "deny"` policy: four internal workspace path dependencies had no
version requirement. All four now specify version `0.1.0` alongside their
paths. No lint was disabled, no advisory ignored and no dependency downgraded.
Unused BSD-3-Clause/ISC/CC0 license allowances were removed to keep the policy
limited to encountered license expressions.

An explicit online fetch of the default RustSec advisory database succeeded:

```powershell
.\target\devtools\bin\cargo-deny.exe -L info --locked fetch db
.\target\devtools\bin\cargo-deny.exe --workspace --locked --offline check --hide-inclusion-graph
```

Observed database: `https://github.com/RustSec/advisory-db`, commit
`7a40350e1b8ba5b656d909f88222a8059a83d88d`, commit timestamp
`2026-11-08T16:03:42-07:00`, fetched 2026-11-09. The second command deliberately
checks this freshly-fetched snapshot without a second network fetch.

Result: **exit 0 — advisories ok, bans ok, licenses ok, sources ok**.
The workspace graph includes all workspace members; no target filter or dev
dependency exclusion was supplied. The fresh snapshot reported no matching
denied security advisory or license/source policy violation in that graph.
This is time-scoped advisory evidence, not a guarantee against unknown issues.

One warning remains: Syn 2.0.119 and Syn 3.0.9 are both required by transitive
procedural macro dependencies. The policy already permits duplicate versions
with a warning. Their dependency graph was reviewed; no risky forced unification
or policy relaxation was introduced. Required Rust/clippy warnings-as-errors
checks remain separate and pass without warnings.

GitHub CI is intentionally disabled during development at the user's request.
Run the commands above locally to refresh this evidence when dependencies or
advisories change. The previous workflow was removed before the initial push;
no GitHub Actions run is part of this verification evidence.

## Review dependencies, 2026-11-09

Added locked pulldown-cmark 0.13.6, quick-xml 0.42.0, csv 1.4.0 and
serde_json 1.0.154 (order/arbitrary precision preservation), plus their small
transitive dependencies. Re-ran the workspace offline audit against the same
fetched RustSec snapshot: exit 0, advisories/bans/licenses/sources all pass;
the existing Syn duplicate warning remains. No license allowance, advisory
ignore or source-policy exception was added. These are parsers, not runtimes
for executing source, HTML, XML external entities or links.

## Additional text-format parsers

Final dependencies: json5 1.3.7 and serde-saphyr 2.1.1, with serialization and
deserialization enabled and optional comment/property extensions disabled.
An initial serde_yaml_ng attempt was rejected by cargo-deny because its
unsafe-libyaml dependency triggers RUSTSEC-2024-0449 (unmaintained). Both were
removed from Cargo.lock; no advisory was ignored or severity reduced. The
replacement passes the existing advisory/bans/source gates.

serde-saphyr's encoding_rs adapter requires `(Apache-2.0 OR MIT) AND BSD-3-Clause`
for WHATWG encoding tables. Reviewed encoding_rs 0.8.44's LICENSE-WHATWG and
LICENSE-MIT, and added a BSD-3-Clause allowance scoped to that crate in
deny.toml. Global allowances remain unchanged. Distribution must retain the
required third-party notices; the application has not been packaged/published.

Final workspace offline check against the recorded RustSec snapshot: exit 0,
advisories/bans/licenses/sources pass, with only the existing Syn duplicate
warning. Parser depth/node/alias budgets remain enabled; no include/property
resolver is installed. Unknown YAML tags are rejected, and JSON5 input has a
string/comment-aware nesting limit. An ordered project-owned Serde value keeps
non-finite numbers/complex keys without libyaml. Project code forbids unsafe
Rust; no source, notebook, config directive or environment value is executed.

## Settings and file-details dependencies

The TUI now uses the existing tempfile crate at runtime for atomic language
preferences and time 0.3.55 with formatting (time-macros 0.2.32) for UTC file
timestamps. Re-ran the locked workspace offline audit against the recorded
snapshot: exit 0, advisories/bans/licenses/sources pass, only the existing Syn
duplicate warning. No policy allowance or advisory exception was added.

## Agent CLI / MCP / local IPC dependencies

Added locked official rmcp 3.9.0 (server/transport-io only), Tokio 1.54.1,
interprocess 2.4.8 with async local sockets, schemars 1.2.3, UUID 1.29.2 and
Windows-only windows-permissions 0.2.4 / widestring. The TUI does not link rmcp;
Agent IPC/runtime starts only with --agent. No remote HTTP transport, shell
execution tool or model/client runtime is enabled.

Read the 0BSD LICENSE files for interprocess' recvmsg 1.0.0 and doctest-file
1.1.1: unrestricted use/copy/modify/distribute, with warranty disclaimer. Added
allowances scoped to these two crates. No advisory was ignored and global
license/source policy remains unchanged.

Refetched RustSec on 2026-11-09: commit
`899077d9d471b9fd3dbf4b2faea8ef26e83692e8`, timestamp
`2026-11-09T11:48:10+01:00`. Locked offline workspace check: exit 0,
**advisories ok, bans ok, licenses ok, sources ok**. Reviewed duplicate-version
warnings now include bitflags, block-buffer, cpufeatures, crypto-common, digest,
sha2 and syn. SDK and existing dependencies require both generations; no
forced downgrade or weakened advisory policy was used. Native strict Clippy
is a separate warnings-as-errors gate.

## Risk mitigation dependencies, 2026-11-10

Only two new locked packages: winapi-util 0.1.11 (Windows stable handle identity)
and xattr 1.7.0 (Unix handle-based extended attributes). guinea-io also reuses
existing serde/serde_json, libc and windows-permissions for recovery/metadata.
No existing package version was upgraded and no policy exception was added.

Refetched RustSec on 2026-11-10, commit
`d9cb059f3217db87aeccd190b81eaffe8cc29cb9`, timestamp
`2026-11-09T17:13:08+01:00`. Locked workspace offline check: exit 0,
**advisories ok, bans ok, licenses ok, sources ok**. The same seven existing
duplicate-version groups remain warnings under the unchanged policy.
Project code still forbids unsafe Rust; target OS wrappers are dependencies.
