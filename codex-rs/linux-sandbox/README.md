# agcodex-linux-sandbox

This crate is responsible for producing:

- a `agcodex-linux-sandbox` standalone executable for Linux that is bundled with the Node.js version of the AGCodex CLI
- a lib crate that exposes the business logic of the executable as `run_main()` so that
  - the `agcodex-exec` CLI can check if its arg0 is `agcodex-linux-sandbox` and, if so, execute as if it were `agcodex-linux-sandbox`
  - this should also be true of the `agcodex` multitool CLI
