# agcodex-protocol

This crate defines the "types" for the protocol used by AGCodex CLI, which includes both "internal types" for communication between `agcodex-core` and `agcodex-tui`, as well as "external types" used with `agcodex mcp`.

This crate should have minimal dependencies.

Ideally, we should avoid "material business logic" in this crate, as we can always introduce `Ext`-style traits to add functionality to types in other crates.
