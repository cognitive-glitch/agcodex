# AGCodex TODO

Updated: 2025-08-21 02:15

This file tracks implementation progress against CLAUDE.md and PLANS.md. Keep entries short, actionable, and dated.

## Status Snapshot

- Overall: Phase 1 largely complete, Phase 2-4 plans ready
- Notable progress this week:
  - Operating modes scaffolding (Plan/Build/Review) added (core/src/modes.rs)
  - TUI supports --mode CLI arg; prompt suffix wired via ModeManager
  - High defaults: reasoning_effort=high, reasoning_summaries=detailed (core/src/config.rs)
  - Context Engine scaffolding: ast_compactor, semantic_index, retrieval, embeddings, cache
  - Code tools scaffolding: tree_sitter (primary), ripgrep, fd_find; ast_grep as optional
  - Embeddings capability helper: detect OpenAI embeddings availability via API key only
  - TUI transcript fixture updated; gating of reasoning status lines via env var to avoid drift
  - **Cargo workspace consolidation completed (2025-08-21)**: ~80 dependencies centralized in root Cargo.toml, all 22 crates updated to use workspace references
  - **anyhow → thiserror migration completed (2025-08-21)**: All compilation errors fixed, workspace compiles cleanly
  - **Comprehensive implementation plans created (2025-08-21)**: Detailed plans for TUI modes, tree-sitter integration, and rebranding
- Tests: Workspace compiles successfully with only harmless warnings

## Phase 1: Foundation & Rebranding

- [ ] Complete rebranding across crates (codex → agcodex) with crate/binary renames
      **Plan created**: 8,773 occurrences, automated script ready
- [x] Introduce OperatingMode scaffolding and restrictions (Plan/Build/Review) (2025-08-21)
- [x] CLI --mode parsing and prompt suffix injection (2025-08-21)
- [ ] TUI wiring for mode switching (Shift+Tab), visual indicators, restrictions enforcement
      **Plan created**: Detailed implementation plan with ModeIndicator widget
- [ ] Session persistence at ~/.agcodex/history with Zstd compression (new persistence crate)
- [x] Set high defaults for reasoning effort and summaries (2025-08-21)
- [x] Complete migration from anyhow to thiserror across codebase (2025-08-21)
      **Status**: Error types created for all crates, compilation successful
- [x] Establish native tool policy: no Comby; tree-sitter primary; ast-grep optional (2025-08-21)
- [x] Scaffolds for native fd-find and ripgrep integrations (2025-08-21)
- [x] Consolidate Cargo workspace dependencies (2025-08-21)

Notes:

- Reasoning status in /status output is gated by env var SHOW_REASONING_STATUS=1 to keep fixtures stable.

## Phase 2: AST Intelligence

- [ ] Tree-sitter integration for 50+ languages (Cargo dependencies and parsers)  
      **Plan created**: Comprehensive list of 50+ parsers, LanguageRegistry design
      Status: scaffolding only (no grammars wired)
- [ ] AST-RAG engine: hierarchical chunking, indexing, embeddings
      **Plan created**: ASTIndexer, SemanticChunker, VectorDatabase designs
- [ ] AI Distiller-style compaction with 90%+ compression
      **Plan created**: AiDistiller algorithm with 3 compression levels
- [ ] Location-aware embeddings (file:line:column)
      **Plan created**: SourceLocation type with precise metadata

## Phase 3: Core TUI Features

- [ ] Message Navigation (Ctrl+J with context restoration)
- [ ] History Browser (Ctrl+H with timeline)
- [ ] Smooth session switching UX (Ctrl+S / Ctrl+O)
- [ ] Multi-Agent orchestrator UI (Ctrl+A), worktree support
- [ ] Notification system (terminal bell/desktop)

## Phase 4: Enhancements

- [ ] Type system improvements (newtype/builder/typestate patterns)
- [ ] Configurable intelligence (light/medium/hard)
- [ ] GPT-5 prompt optimization (XML-like structured prompts)
- [ ] AST-based edit tools with precise location metadata

## Internal Tools & Policy

- [x] Do not use Comby
- [x] Prefer tree-sitter as primary structural engine
- [x] Offer ast-grep as optional internal tooling (scaffold present)
- [ ] Implement native fd-find and ripgrep integrations

## Embeddings

- [x] Policy check: ChatGPT login tokens do NOT grant embeddings API access
- [x] Helper to detect OpenAI embeddings availability via OPENAI_API_KEY or auth.json
- [ ] Wire embeddings provider(s) and configuration

## Testing & QA

- [x] codex-tui library tests pass (117/117)
- [ ] Investigate and fix failing workspace tests (linux-sandbox, mcp-server, etc.)
- [ ] Add new tests for modes (TUI Shift+Tab), session persistence, and context engine

## Immediate Next Steps (Short List)

1. ~~Complete anyhow → thiserror migration~~ ✅ Done (2025-08-21)
2. **Phase 2.1**: Enhance ModeManager with cycle functionality and visual properties
3. **Phase 2.2**: Create ModeIndicator widget and integrate Shift+Tab handler
4. **Phase 3.1**: Add tree-sitter language dependencies to workspace Cargo.toml
5. **Phase 3.2**: Create agcodex-ast crate with LanguageRegistry
6. **Phase 4.1**: Run rebranding script (codex → agcodex)
7. Add persistence crate with Zstd compression support

## Conventions

- Keep TODO concise; link to PLANS.md for details
- Use [x]/[ ] checkboxes; add date suffix when closing items
- Prefer thiserror over anyhow; avoid introducing new anyhow uses
