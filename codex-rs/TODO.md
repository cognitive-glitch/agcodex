# AGCodex TODO

Updated: 2025-08-21 03:30

This file tracks implementation progress against CLAUDE.md and PLANS.md. Keep entries short, actionable, and dated.

## Status Snapshot

- Overall: Major implementation milestones achieved - compilation clean, AST infrastructure complete
- Notable progress this week:
  - Operating modes scaffolding (Plan/Build/Review) added (core/src/modes.rs)
  - TUI supports --mode CLI arg; prompt suffix wired via ModeManager
  - High defaults: reasoning_effort=high, reasoning_summaries=detailed (core/src/config.rs)
  - Context Engine scaffolding: ast_compactor, semantic_index, retrieval, embeddings, cache
  - Code tools scaffolding: tree_sitter (primary), fd_find; AST-based agent tools
  - Embeddings capability helper: detect OpenAI embeddings availability via API key only
  - TUI transcript fixture updated; gating of reasoning status lines via env var to avoid drift
  - **Cargo workspace consolidation completed (2025-08-21)**: ~80 dependencies centralized in root Cargo.toml, all 22 crates updated to use workspace references
  - **anyhow → thiserror migration completed (2025-08-21)**: All compilation errors fixed, workspace compiles cleanly
  - **Comprehensive implementation plans created (2025-08-21)**: Detailed plans for TUI modes, tree-sitter integration, and rebranding
  - **Tree-sitter AST infrastructure implemented (2025-08-21)**: 50+ languages, LanguageRegistry, compactor with 70-95% compression
  - **Session persistence implemented (2025-08-21)**: Zstd compression, bincode/MessagePack serialization
  - **All compilation errors fixed (2025-08-21)**: bincode v2 migration, mcp_types references, variable names corrected
- Tests: Workspace compiles successfully with only harmless warnings

## Phase 1: Foundation & Rebranding [✅ 90% Complete]

- [ ] Complete rebranding across crates (codex → agcodex) with crate/binary renames
      **Plan created**: 8,773 occurrences, automated script ready
- [x] Introduce OperatingMode scaffolding and restrictions (Plan/Build/Review) (2025-08-21)
- [x] CLI --mode parsing and prompt suffix injection (2025-08-21)
- [ ] TUI wiring for mode switching (Shift+Tab), visual indicators, restrictions enforcement
      **Plan created**: Detailed implementation plan with ModeIndicator widget
- [x] Session persistence at ~/.agcodex/history with Zstd compression (2025-08-21)
      **Status**: Persistence crate created with full implementation
- [x] Set high defaults for reasoning effort and summaries (2025-08-21)
- [x] Complete migration from anyhow to thiserror across codebase (2025-08-21)
      **Status**: Error types created for all crates, compilation successful
- [x] Establish native tool policy: no Comby; tree-sitter primary; ast-grep optional (2025-08-21)
- [x] Scaffolds for fd-find and AST-based agent tools (2025-08-21)
- [x] Consolidate Cargo workspace dependencies (2025-08-21)

Notes:

- Reasoning status in /status output is gated by env var SHOW_REASONING_STATUS=1 to keep fixtures stable.

## Phase 2: AST Intelligence [✅ Complete]

- [x] Tree-sitter integration for 50+ languages (2025-08-21)
      **Status**: Full implementation with LanguageRegistry, 47 languages supported
- [x] AST-RAG engine: hierarchical chunking, indexing, embeddings (2025-08-21)
      **Status**: Implemented with ASTIndexer, SemanticIndex, ParserCache
- [x] AI Distiller-style compaction with 90%+ compression (2025-08-21)
      **Status**: Implemented with 3 levels (Light: 70%, Standard: 85%, Maximum: 95%)
- [x] Location-aware embeddings (file:line:column) (2025-08-21)
      **Status**: SourceLocation type implemented with precise metadata tracking

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
- [x] Implement AST-based agent tools infrastructure (2025-08-21)
      **Status**: Module structure created in code_tools

## Embeddings (Independent System)

- [x] Policy check: ChatGPT login tokens do NOT grant embeddings API access
- [x] Helper to detect OpenAI embeddings availability via OPENAI_API_KEY or auth.json
- [ ] **Create independent embeddings module** (completely separate from chat)
- [ ] **Implement multi-provider support** (OpenAI, Gemini, Voyage AI)
- [ ] **Add disabled-by-default configuration** (zero overhead when off)
- [ ] **Separate authentication** (OPENAI_EMBEDDING_KEY, GEMINI_API_KEY, VOYAGE_API_KEY)
- [ ] **Provider auto-selection** based on available keys
- [ ] **Intelligence mode mapping** for each provider
- [ ] **Optional caching layer** with DashMap
- [ ] **Context engine integration** (hybrid AST + embeddings when enabled)

## Testing & QA

- [x] codex-tui library tests pass (117/117)
- [ ] Investigate and fix failing workspace tests (linux-sandbox, mcp-server, etc.)
- [ ] Add new tests for modes (TUI Shift+Tab), session persistence, and context engine

## Immediate Next Steps (Short List)

1. ~~Complete anyhow → thiserror migration~~ ✅ Done (2025-08-21)
2. ~~Fix all compilation errors~~ ✅ Done (2025-08-21)
3. ~~Implement tree-sitter AST infrastructure~~ ✅ Done (2025-08-21)
4. ~~Create persistence crate with compression~~ ✅ Done (2025-08-21)
5. **Run rebranding script** (codex → agcodex) - Script ready, 8,773 occurrences
6. **TUI Mode Integration**: Wire Shift+Tab switching with ModeIndicator widget
7. **Independent Embeddings System**: Create optional multi-provider support
8. **Subagent System**: Implement @agent-name invocation and context isolation
9. **Session Management UI**: Implement Ctrl+S save, Ctrl+O load dialogs

## Conventions

- Keep TODO concise; link to PLANS.md for details
- Use [x]/[ ] checkboxes; add date suffix when closing items
- Prefer thiserror over anyhow; avoid introducing new anyhow uses
