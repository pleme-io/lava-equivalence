# Pangea parity — what a real oracle measured

## Why the goldens in `tests/goldens/` are not parity evidence

They were rendered by lava. Comparing lava against them proves **render
stability** — that lava keeps producing what it produced yesterday — and it
cannot, by construction, prove agreement with Pangea. A self-generated golden
goes green on the day lava is wrong, in exactly the way it goes green on the
day lava is right.

The oracle has to come from the other implementation. Pangea already commits
one next to each workspace: a rendered `<workspace>.tf.json`, produced by the
Ruby that magma actually applies. **No Ruby needs to run to use it** — it is a
file in git. That is the whole differential, and it costs nothing to keep.

## Where the real differential lives — and why not here

Every committed oracle belongs to a private environment: its rendered JSON
carries live cloud resource ids and operator CIDRs, which is why
`pangea-architectures` is private. **This repo is public.**

So the split is:

| layer | repo | holds |
|---|---|---|
| comparison machinery | `lava-equivalence` (public) | generic; no environment data |
| architecture structure | `lava-architectures` (public) | `.tlisp` with `:inputs`, zero literals |
| oracle + values | `pangea-architectures` (private) | the real `.tf.json` |

A parity architecture is the artifact most likely to be written by copying its
oracle, and the oracle is the thing that must not travel. Hence every
environment value in `aws-sg-ingress-rules.tlisp` is an `:input`.

## Measured 2026-08-22 — first real oracle

One architecture (`aws-sg-ingress-rules`) was derived from a real workspace and
rendered against that workspace's committed `.tf.json`. Result: **10 of 18 JSON
paths identical**, zero lava-only paths. Every resource-level field that both
sides emit — `type`, `protocol`, `cidr_blocks`, `security_group_id`,
`description` — matched exactly.

The 8 that diverged are three distinct findings, each confirmed in source
rather than inferred from the diff:

### 1. Every scalar renders as a JSON string

`from_port` / `to_port` render `"22"` where Pangea renders `22`. lava has no
numeric literal — bindings arrive through `set_str` and stay strings.

Terraform tolerates both for ports, so this breaks *parity* without breaking
*apply* — which is precisely why it needs a differential to find. A gap that
still works is one no operator ever reports.

### 2. `:result` does not become a terraform `output` block

`lava-core` supports outputs — `Architecture::render_terraform_json` emits an
`output` block from `self.outputs` (`lava-core/src/lib.rs:493`), and a unit
test pins it (`:628`). But **all 30 bundled architectures declare `:result`,
and zero goldens contain an `output` block.** The `.tlisp` evaluator never
populates `Architecture.outputs` from the `:result` form.

So this is a *wiring* gap between the Lisp surface and a core that already
works, not a missing capability — and it is invisible from either side alone:
lava-core's test passes, every architecture looks like it declares outputs, and
the rendered JSON silently has none.

### 3. Terraform output `description` is unrepresentable

`render_terraform_json` inserts exactly one key per output, `value`. Pangea
emits `value` and `description`. This one *is* a capability gap in lava-core,
and it is the honest floor: 2 of the 4 missing paths cannot be produced today
no matter how the architecture is written.

## The setup property worth knowing before extending this

`lava-equivalence` depends on **published** `lava-architectures` from crates.io,
not on the sibling checkout. A newly authored architecture is therefore not
under test here until it is released — a local `[patch.crates-io]` does not fix
it either, because `load_bundled` bakes `CARGO_MANIFEST_DIR` at compile time and
the registry copy wins. Render from inside `lava-architectures` when measuring
something unreleased.
