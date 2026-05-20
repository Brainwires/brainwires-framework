# Tombstone packages

In v0.11.0 the Deno port renamed six packages to match Rust's singular crate
names, and dissolved `@brainwires/tools` into `@brainwires/tool-runtime` +
`@brainwires/tool-builtins`. This directory holds the **final** publishes of the
old names — each is a 2-file package (`deno.json` + `mod.ts`) that re-exports
the new package(s) with a `@deprecated` banner.

## Publish procedure

These should be published from a separate `release/0.10.2-tombstones` branch
that contains **only** these seven packages at the workspace root — _not_ from
`main`, so the new packages (which share filesystem space with the tombstones'
parent dirs in `packages/`) don't shadow them.

```bash
git checkout -b release/0.10.2-tombstones
# Move each tombstones/<old>/ into packages/<old>/
# Delete everything else under packages/
# For each:
for pkg in providers permissions agents mcp resilience training tools; do
  (cd packages/$pkg && deno publish)
done
git tag v0.10.2-tombstones
# Never touch this branch again.
```

After publish, the JSR registry has both:

- `@brainwires/<old>@0.10.2` — deprecation-tagged re-export
- `@brainwires/<new>@0.11.0` — the real new package

Consumers pinned to `^0.10.x` of an old name will keep working; the next time
they run `deno cache --reload` they'll see the `@deprecated` banner and the
deprecation field set on the JSR package.

## Mapping

| Old name                  | Re-exports                                                                                                                          |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `@brainwires/providers`   | `@brainwires/provider` + `@brainwires/provider-speech`                                                                              |
| `@brainwires/permissions` | `@brainwires/permission` (anomaly moved to `@brainwires/telemetry/anomaly`)                                                         |
| `@brainwires/agents`      | `@brainwires/agent` + `@brainwires/inference` + `@brainwires/mdap` + `@brainwires/seal` + `@brainwires/skills` + `@brainwires/eval` |
| `@brainwires/mcp`         | `@brainwires/mcp-client` (server moved to `@brainwires/mcp-server`)                                                                 |
| `@brainwires/resilience`  | `@brainwires/call-policy`                                                                                                           |
| `@brainwires/training`    | `@brainwires/finetune`                                                                                                              |
| `@brainwires/tools`       | `@brainwires/tool-runtime` + `@brainwires/tool-builtins`                                                                            |
