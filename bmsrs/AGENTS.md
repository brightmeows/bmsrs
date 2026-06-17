# bmsrs (root crate)

Re-export facade. No new logic.

## Module hierarchy

The `pub mod` tree mirrors the workspace directory structure:

```
bms::{tokenizer, control_flow, parser, processor}
bmson::{def, processor}
chart
player
```

When adding a new crate to the workspace, add a corresponding `pub mod`
with `pub use new_crate::*;` here. Placeholder crates are excluded — only
re-export crates with public API.
