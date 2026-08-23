<!-- qbit-toolkit:codex-ai-tooling:start -->
## Qbit AI tooling

This managed block describes repository-owned AI tooling installed by `qbit-toolkit`. Existing project instructions outside this block remain authoritative for architecture, build/test commands, coding conventions, and contribution rules.

### Mandatory task classification

Before selecting an AI capability, classify the request as ordinary repository work, semantic code work, architecture-impact analysis, external-library documentation, or real incident analysis.

### Deterministic tool routing

- Use built-in file/command/Git tools for ordinary repository work.
- Use Serena only when semantic symbol/reference/diagnostic behavior is materially useful. Start with a read-only semantic call.
- Use Graphify only for explicit broad architecture-impact work and only through the scoped `.ai/scripts/graphify-*` wrappers. Always validate material graph conclusions against source and, where useful, Serena.
- Use Context7 only for narrow external version-specific documentation after resolving the dependency/version from project metadata.
- Use Sentry only for a concrete incident and only through the configured read-only tools.
- Browser/Playwright tooling is intentionally absent unless the project establishes a separate justified capability.

### Execution timing and reuse

- Local tooling is container-owned; do not install Serena, Graphify, language servers, or Rust components globally on the host.
- Serena starts lazily through project-scoped MCP configuration; first use may build the pinned image.
- Graphify requires an explicit repository-relative scope. The build wrapper reuses an unchanged scope fingerprint; the update wrapper forces a rebuild.
- Full Doctor is for installation, maintenance, tooling/version changes, or real inconsistency, not every prompt.
- Bootstrap and Doctor must not install target application dependencies or run target-root package-manager/Cargo operations.
- Follow `.ai/policies/tool-boundaries.md` for trust, mutation, network, and evidence boundaries.
<!-- qbit-toolkit:codex-ai-tooling:end -->

## AI context lifecycle

- Before the first substantive repository analysis, planning, or implementation in a Codex session, automatically run `powershell -NoProfile -ExecutionPolicy Bypass -File .ai/context/context.ps1 start`. Do not ask the developer and do not wait for a context-loading instruction.
- Read `.ai-bridge/context-runtime.md` before substantive planning or implementation. Do not rerun `start` repeatedly in the same session unless the context cache may have changed materially.
- Central AI context is coordination evidence, never implementation authority. Current source, tests, schemas/migrations, explicit contracts, and committed canonical decisions outrank stored context according to their claim type.
- Preserve pre-existing uncommitted user changes; do not stage, format, rewrite, reset, clean, or otherwise disturb unrelated work.
- After a substantive validated milestone that materially changes durable continuity state, and before the final handoff, automatically write `.ai-bridge/context-checkpoint.json` and run `powershell -NoProfile -ExecutionPolicy Bypass -File .ai/context/context.ps1 checkpoint`. No developer reminder is required.
- Do not create checkpoints for ordinary read-only questions, every chat message, or work that produced no durable continuity change.
- The checkpoint JSON must use `schemaVersion: 1`, repository `qbit-toolbox`, and include `scope`, controlled `status`, `objective`, `confirmedFindings[]`, `decisions[]`, `rejectedApproaches[]`, `validation[]`, `openQuestions[]`, and `nextAction`.
- Promote durable technical decisions into their canonical owning repository/ADR/contract; the context checkpoint records continuity, not technical authority.
- Never store secrets, credentials, cookies, tokens, private keys, `.env` values, customer secrets, or raw chat transcripts in AI context.
- If context start/checkpoint fails because of missing context source, authentication, dirty/diverged context state, or a concurrent conflict, do not perform destructive recovery; report the condition and preserve data.
- `.ai-bridge/` and `.ai/context/cache/` are transient/ignored runtime locations. Serena and Graphify remain derived evidence tools, not project memory.
