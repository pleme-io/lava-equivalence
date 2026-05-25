# lava-equivalence — scripts/

## `regen-pangea-goldens.rb`

Regenerates the committed `tests/goldens/*.json` files from real
pangea Ruby DSL output. Run when:

- A new pangea architecture is ported into lava
- Pangea's renderer changes the wire shape
- You suspect lava ↔ pangea drift

### Environment

```bash
brew install ruby           # or rbenv / asdf
gem install bundler:2.5.22  # match Gemfile.lock
```

### Run

```bash
cd /path/to/pangea-architectures
bundle install
bundle exec ruby /path/to/lava-equivalence/scripts/regen-pangea-goldens.rb
```

The script writes JSON into `lava-equivalence/tests/goldens/<arch>.json`.
The next `cargo test --release` in lava-equivalence diffs lava render
output against these pangea-generated goldens. Any divergence is the
real ↔ canonical pangea ↔ lava byte-equivalence gap to investigate.

### Why a Ruby script (not pure Rust)

Pangea is a Ruby DSL. The shortest path to "real pangea-generated
terraform.json" is to call the Ruby code directly. The goldens it
writes are pure data (JSON) — Rust-side consumption stays in
`lava-equivalence/tests/golden_round_trip.rs` unchanged.

