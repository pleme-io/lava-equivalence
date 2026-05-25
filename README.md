# lava-equivalence

Byte-equivalence harness for the lava suite. For each bundled
architecture: render via lava-architectures → diff against a typed
golden fixture committed under `tests/goldens/`.

## Today

Goldens are seeded from lava's own renderer on first run. They act
as a regression net — any change to the renderer that perturbs
terraform.json output breaks the matching golden test.

## Tomorrow

When pangea-side fixtures are generated from real Ruby +
`tofu plan -json`, they drop into `tests/goldens/*.json` as
replacements. The harness shape stays identical; it just becomes
the byte-equivalence proof between pangea (Ruby+tofu) and
lava (tlisp+magma).

## Typed surface

- `Fixture` — architecture name + input bindings + golden path.
- `render_lava(&Fixture)` — load + eval + render terraform.json.
- `assert_terraform_json_equivalent(&actual, &expected)` —
  semantic JSON diff with sorted-key normalization; surfaces typed
  `EquivalenceMismatch { pointer, actual, expected }` on diff.
- `EquivalenceError` — Io / Eval / Render / Mismatch.

## Running

```bash
cargo test --release
```

5 unit + 4 round-trip tests; matrix-style aggregation over every
bundled architecture in `lava-architectures::BUNDLED_ARCHITECTURES`.
