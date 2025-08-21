# AGCodex CLI (Rust Implementation)

We provide AGCodex CLI as a standalone, native executable to ensure a zero-dependency install.

## Installing AGCodex

Today, the easiest way to install AGCodex is via `npm`, though we plan to publish AGCodex to other package managers soon.

```shell
npm i -g @openai/agcodex@native
agcodex
```

You can also download a platform-specific release directly from our [GitHub Releases](https://github.com/openai/agcodex/releases).

## What's new in the Rust CLI

While we are [working to close the gap between the TypeScript and Rust implementations of AGCodex CLI](https://github.com/openai/agcodex/issues/1262), note that the Rust CLI has a number of features that the TypeScript CLI does not!

### Config

AGCodex supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json`. See [`config.md`](./config.md) for details.

### Model Context Protocol Support

AGCodex CLI functions as an MCP client that can connect to MCP servers on startup. See the [`mcp_servers`](./config.md#mcp_servers) section in the configuration documentation for details.

It is still experimental, but you can also launch AGCodex as an MCP _server_ by running `agcodex mcp`. Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector agcodex mcp
```

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](./config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS.

### `agcodex exec` to run AGCodex programmatially/non-interactively

To run AGCodex non-interactively, run `agcodex exec PROMPT` (you can also pass the prompt via `stdin`) and AGCodex will work on your task until it decides that it is done and exits. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.

### Use `@` for file search

Typing `@` triggers a fuzzy-filename search over the workspace root. Use up/down to select among the results and Tab or Enter to replace the `@` with the selected path. You can use Esc to cancel the search.

### `--cd`/`-C` flag

Sometimes it is not convenient to `cd` to the directory you want AGCodex to use as the "working root" before running AGCodex. Fortunately, `agcodex` supports a `--cd` option so you can specify whatever folder you want. You can confirm that AGCodex is honoring `--cd` by double-checking the **workdir** it reports in the TUI at the start of a new session.

### Shell completions

Generate shell completion scripts via:

```shell
agcodex completion bash
agcodex completion zsh
agcodex completion fish
```

### Experimenting with the AGCodex Sandbox

To test to see what happens when a command is run under the sandbox provided by AGCodex, we provide the following subcommands in AGCodex CLI:

```
# macOS
agcodex debug seatbelt [--full-auto] [COMMAND]...

# Linux
agcodex debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run AGCodex with the default, read-only sandbox
agcodex --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
agcodex --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
agcodex --sandbox danger-full-access
```

The same setting can be persisted in `~/.agcodex/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for AGCodex. Ultimately, we hope this to be a library crate that is generally useful for building other Rust/native applications that use AGCodex.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.
