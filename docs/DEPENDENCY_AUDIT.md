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
