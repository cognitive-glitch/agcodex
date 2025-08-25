# agcodex-core

This crate implements the business logic for AGCodex. It is designed to be used by the various AGCodex UIs written in Rust.

## Dependencies

Note that `agcodex-core` makes some assumptions about certain helper utilities being available in the environment. Currently, this

### macOS

Expects `/usr/bin/sandbox-exec` to be present.

### Linux

Expects the binary containing `agcodex-core` to run the equivalent of `agcodex debug landlock` when `arg0` is `agcodex-linux-sandbox`. See the `agcodex-arg0` crate for details.

### All Platforms

Expects the binary containing `agcodex-core` to simulate the virtual `apply_patch` CLI when `arg1` is `--agcodex-run-as-apply-patch`. See the `agcodex-arg0` crate for details.
