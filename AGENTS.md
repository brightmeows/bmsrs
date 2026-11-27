# bmsrs

## Project scope

A Rust workspace for parsing, converting, and simulating playback of BMS and
BMSON charts. Two domain pipelines (`bms/`, `bmson/`) converge on `common/`:
`bmsrs-chart` is the intermediate representation, `bmsrs-player` queries it, and
`bmsrs-pipeline` holds the shared vocabulary. The root crate is a re-export
facade. `README.md` carries the crate table and the pipeline, and each
crate's crate-level doc states its role.

> **Release status**: `0.0.3` is released; the project is pre-1.0. All public
> interfaces may change at any time and semver compatibility does not apply.
> Cross-crate architectural changes and breaking API changes only require
> confirmation from the user.

`bms-lsp` lives in a separate repository:
<https://github.com/brightmeows/bms-lsp>. It pins this workspace's
`bmsrs-bms-analysis` and `bmsrs-bms-syntax` at their published versions.

External dependencies are managed through `[workspace.dependencies]`; do not
specify versions in individual crate `Cargo.toml` files. Internal
dev-dependencies are path-only (`{ path = "..." }`, no version): a version
requirement makes `cargo publish` resolve from the registry, and release-plz
orders releases without dev-dependencies, so packaging fails whenever the
dependency crate is unpublished.

### Facade crate

The repository root is the `bmsrs` facade crate. When adding a workspace crate,
add the corresponding `pub mod` and `pub use` in `src/lib.rs`.

## Commands

`pre-commit run --all-files` is the completion gate; the hooks and each failure's
fix live in `.pre-commit-config.yaml`. Workspace tests run with
`cargo test --workspace --quiet`.

## Conventions

### Commit format

Conventional Commits with a scope, matching the `release-plz.toml` changelog
groups. Titles and bodies are English, a repository-level override of the
Chinese default. Mark breaking changes with `!` and add a
`BREAKING CHANGE: <migration note>` footer; release-plz renders it into the
changelog entry.

### Worktree directory

Git worktrees live under `.worktrees/`.

### Lint conventions

The lint table lives in `[workspace.lints]` in the root `Cargo.toml`. `#[allow]`
is forbidden: use `#[expect(clippy::lint, reason = "...")]` and keep the reason,
because only its presence is checked.

### API design

Namespace a public operation on the type it belongs to: constructors, queries,
conversions, and lifecycle stages are associated functions. An operation stays
free in three cases: it has no domain type to hang on (`base36_digit_value`),
its natural owner cannot host it because of the dependency direction
(`Bms::parse` would make the model crate depend on the parser), or the language
requires it, as with a `#[proc_macro_derive]` entry point.

The rule buys discoverability: every operation appears in its type's rustdoc,
and a call site names what it acts on. It costs a divergence from Rust's
free-function idiom; rust-analyzer's guide takes the opposite default for
single-action APIs, so read this as this workspace's convention, not as a
correctness rule.

The facade may flatten an associated function into a free entry point when the
body is a single delegation to it and the receiver would be the only argument
(`bms::write` is `BmsWriter::write`). The condition is that no second
implementation exists to keep in sync.

A zero-sized namespace type (`BmsProcessor`, `BmsonProcessor`, `BmsWriter`)
groups operations that share a parameter set or a conversion process; it is not
a builder. Builder means a type with a terminal method that consumes it, and
`MessagesBuilder` is the only one in the workspace.

Diagnostic codes come from `bmsrs_pipeline::DiagnosticCode` only. A layer may
wrap it in its own warning type, but no layer may invent a second string
vocabulary; `DiagnosticCode::as_str` is the only place a code becomes a string.

### Interface shapes

Pick the construction shape by what varies, not by habit:

| What varies | Shape | Example |
|-------------|-------|---------|
| Required inputs only | constructor | `Chart::new`, `TimingTrack::new` |
| Optional overlays on a one-shot value | consuming `with_*` setter | `ChartData::new(..).with_ln_type_hint(..)`, `Diagnostic::new(..).with_span(..)` |
| Configuration reused across calls | options struct with public fields | `ParseOptions`, `DocumentOptions` |
| Mutually exclusive input kinds | closed sum type with constructors | `Input::text` / `Input::bytes` / `Input::file` |
| A stage the caller must not skip | consuming finalizer | `MessagesBuilder::finish` |
| Optional overlays that repeat as a set | preset `for_*` constructor | none yet; the trigger is a second call site setting the same seven `ChartData` overlays |

An options struct implements `Default` only when the default is specified
behavior (first branch, `CollectAll`), not an arbitrary guess; a caller with a
better default should have to say so.

A builder type is not the default shape. The pipeline is a pure data
transformation, so a separate builder adds a type, a fallible `build()`, and a
second spelling of the same arguments without removing anything a constructor
plus `with_*` already removes. Reassess only when construction gains staged
requirements or mutually exclusive choices, which is where typestate pays for
itself. `Input` is that reassessment: three mutually exclusive sources behind
one type with constructors, and still no terminal `build()`.

Every type named by a curated facade entry point must resolve from the same
module. `tests/facade_api.rs` names them, so dropping a re-export fails to
compile. `raw` is the escape hatch for everything else, which means `prelude`
imports from the curated modules and never from `raw`; it exports types and
traits only, so functions stay behind `bms::` and `bmson::`.

`pub` inside a private module is the house style: `clippy::redundant_pub_crate`
is denied, and the private module already caps visibility. The compiler's
`unreachable_pub` lint stays off because the two contradict each other. The
over-wide items already in the tree are not chased; the lint conflict is the
reason, and `pub(crate)` would fail the build.

Enums mirroring a closed external vocabulary (BMS channels, syntax kinds,
header commands) stay exhaustive, including `EventKind`: a new event kind means
the format gained a construct, and every renderer should be forced to handle it
rather than silently ignore it.

Deferred interface work, with the trigger that reopens it:

| Item | What deferring costs today | Trigger |
|------|---------------------------|---------|
| `Viewport` options struct | `Viewport::new` takes two `f64`, so a swapped pair compiles | a third display parameter arrives; `Viewport::new` already clamps both scalars, so a new field would grow a `new_with_*` ladder |
| A recoverable-warning channel for `bmson::parse_str` | a tool handling both formats branches on the return shape, since `bms::parse` returns `Outcome` and `bmson::parse_str` does not | BMSON parsing produces its first recoverable warning |
| One return shape for range queries (`Player::events_in_range` yields a slice, `notes_in_range` an iterator) | one loop must call `.iter()` on one query and not the other | a consumer ranges over both in the same loop and has to convert |
| `#[non_exhaustive]` on `DiagnosticCode`, `SourceFormat`, and `SymbolKind` | every added variant breaks a downstream exhaustive match; free before 1.0 | 1.0, where semver starts to matter |

### Format sources

The format documents are external and not vendored. `README.md`'s "Format scope
and sources" table is authoritative; cite the original, not an intermediate
note. The KaleidBeat manual wins when the secondary contract disagrees.

### Development references

`rust-meta-cognition` (<https://github.com/brightmeows/rust-meta-skills>) routes
Rust design and compilation problems.

### Comment style

Doc and implementation comments are English; BMS/bmson command names keep their
spelling and code identifiers stay in backticks. Public items use `///`, modules
`//!`. `no-cjk-in-comments` enforces full-line comments and `#[expect]` reasons;
inline trailing comments are review.

### Test conventions

Tests are named `<scenario>_<expectation>`, one scenario per test, with related
assertions allowed. Public API tests live in `tests/`, `pub(crate)` and private
tests in `#[cfg(test)]` modules inside `src/`. Tests exercise public types, not
internal wrappers.

### Boundaries

Deliberate limits and engine divergences are recorded in the owning layer:
`common/chart/AGENTS.md` for the IR and time model, `bms/processor/AGENTS.md`
and `bmson/processor/AGENTS.md` for conversion, and `bms/syntax/AGENTS.md` for
control flow. Check the relevant file before adding format support rather than
assuming a gap.

Where engines disagree, the default follows the specification and the
compatibility case gets an opt-in switch or a documented refusal. Do not "fix"
one side silently.

### Versioning

Versions, `CHANGELOG.md`, and git tags belong to release-plz.

## Gates

A PR opens once its checks pass and merges only after the user approves. One
logical change per commit. Ask before adding an external dependency or touching
CI, or changing `.pre-commit-config.yaml`. Never force-push `main`.
