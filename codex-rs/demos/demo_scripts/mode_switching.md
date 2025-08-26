# AGCodex Mode Switching Demo Script

## Overview
This demo showcases AGCodex's unique three-mode system that adapts to different stages of development.

## Demo Duration: 3 minutes

---

## Part 1: Introduction (30 seconds)

```bash
# Launch AGCodex
$ cargo run --bin agcodex
```

**Narration:**
"Welcome to AGCodex, where AI coding assistance adapts to your workflow. Unlike other tools that give full access all the time, AGCodex provides three distinct operating modes designed for different development phases."

**Screen shows:**
```
╔═══════════════════════════════════════════════════════════════╗
║  AGCodex v2.0.0 - AI-Powered AST Intelligence                ║
║  Current Mode: 🔨 BUILD (Full Access)                        ║
║  Press Shift+Tab to switch modes                             ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## Part 2: PLAN Mode - Read-Only Analysis (60 seconds)

**Action:** Press `Shift+Tab` to switch to PLAN mode

**Screen shows:**
```
╔═══════════════════════════════════════════════════════════════╗
║  Mode: 📋 PLAN (Read-Only Analysis)                          ║
╚═══════════════════════════════════════════════════════════════╝
```

**User Input:**
```
> Analyze the architecture of this codebase and identify potential 
  performance bottlenecks in the data processing pipeline
```

**AGCodex Response:**
```
[PLAN MODE - Read-Only Analysis]

Analyzing codebase architecture...
✓ Parsed 1,247 files across 27 different languages
✓ Generated AST for 98,456 functions/methods
✓ Compressed context by 92% (1.2MB → 96KB)

Architecture Overview:
┌─────────────────┐
│   Frontend      │──REST API──▶┌──────────────┐
│   (React/TS)    │             │  API Gateway │
└─────────────────┘             └──────┬───────┘
                                       │
                               ┌───────▼────────┐
                               │  Service Layer │
                               │  (Node.js)     │
                               └───────┬────────┘
                                       │
                        ┌──────────────┼──────────────┐
                        ▼              ▼              ▼
                 ┌──────────┐  ┌──────────┐  ┌──────────┐
                 │ Auth Svc │  │ Data Svc │  │ Cache    │
                 └──────────┘  └──────────┘  │ (Redis)  │
                                              └──────────┘

Performance Bottlenecks Identified:
1. 🔴 Critical: data_processor.js:L234-L567
   - Nested loops with O(n³) complexity
   - Processing 10K records takes ~12s
   
2. ⚠️ Warning: api/handlers/batch.ts:L89
   - No pagination on batch endpoints
   - Memory spike with >1000 items
   
3. ⚠️ Warning: cache_manager.rs:L445
   - Cache invalidation causing cascading updates
   - 67% of cache writes trigger full rebuild

Recommendations:
• Refactor data_processor.js using streaming approach
• Implement cursor-based pagination
• Use partial cache invalidation strategy

[Note: In PLAN mode, I can analyze but cannot modify files]
```

**Narration:**
"In PLAN mode, AGCodex provides deep analysis without any write permissions. Perfect for code reviews, architecture exploration, and understanding unfamiliar codebases safely."

---

## Part 3: BUILD Mode - Full Development Power (60 seconds)

**Action:** Press `Shift+Tab` to switch to BUILD mode

**Screen shows:**
```
╔═══════════════════════════════════════════════════════════════╗
║  Mode: 🔨 BUILD (Full Access)                                ║
╚═══════════════════════════════════════════════════════════════╝
```

**User Input:**
```
> Fix the O(n³) complexity issue in data_processor.js and 
  implement the streaming approach
```

**AGCodex Response:**
```
[BUILD MODE - Full Access Enabled]

Analyzing data_processor.js with AST...
✓ Identified nested loop structure at lines 234-567
✓ Found 3 levels of array iterations
✓ Detecting data dependencies...

Implementing streaming solution:

📝 Editing: data_processor.js
────────────────────────────────────
- Lines 234-567: Replacing nested loops with stream pipeline
+ Added: Stream transformation pipeline
+ Added: Async iterator pattern
+ Added: Batch processing with configurable chunk size

// Before (O(n³) complexity):
- for (const user of users) {
-   for (const transaction of user.transactions) {
-     for (const item of transaction.items) {
-       processedData.push(transformItem(user, transaction, item));
-     }
-   }
- }

// After (O(n) with streaming):
+ const processedData = await pipeline(
+   users.stream(),
+   flatMapAsync(user => user.transactions),
+   flatMapAsync(transaction => transaction.items),
+   mapAsync(item => transformItem(item)),
+   batch(100), // Process in chunks of 100
+   collect()
+ );

✓ File updated successfully
✓ Running performance benchmark...

Benchmark Results:
├─ Before: 10,000 records in 12.3s (813 records/sec)
└─ After:  10,000 records in 0.8s (12,500 records/sec)
           15.4x performance improvement! 🚀

✓ All tests passing (247/247)
✓ No type errors detected
✓ Memory usage reduced by 78%
```

**Narration:**
"BUILD mode provides full access to create, modify, and refactor code. AGCodex understands the AST structure, making intelligent transformations while maintaining code correctness."

---

## Part 4: REVIEW Mode - Quality Assurance (60 seconds)

**Action:** Press `Shift+Tab` to switch to REVIEW mode

**Screen shows:**
```
╔═══════════════════════════════════════════════════════════════╗
║  Mode: 🔍 REVIEW (Quality Focus - Limited to 10KB edits)     ║
╚═══════════════════════════════════════════════════════════════╝
```

**User Input:**
```
> Review the streaming implementation for edge cases and add 
  appropriate error handling
```

**AGCodex Response:**
```
[REVIEW MODE - Quality & Safety Focus]

Analyzing recent changes for quality issues...
✓ AST validation complete
✓ Type safety verified
✓ Performance regression tests passed

Quality Review Findings:

🔍 Edge Cases Identified:
1. Empty input array handling
2. Null/undefined values in stream
3. Memory pressure with large batches
4. Network interruption during streaming
5. Partial batch processing on error

📝 Adding defensive code (7.2KB edit - within 10KB limit):

+ // Edge case: Empty input
+ if (!users || users.length === 0) {
+   return { processedData: [], stats: getEmptyStats() };
+ }

+ // Edge case: Null safety
+ const safeTransform = withNullCheck(transformItem);

+ // Edge case: Memory management
+ const SAFE_BATCH_SIZE = Math.min(
+   getAvailableMemory() / AVG_ITEM_SIZE,
+   100
+ );

+ // Edge case: Error recovery
+ const processedData = await pipeline(
+   users.stream(),
+   flatMapAsync(user => user?.transactions ?? []),
+   flatMapAsync(transaction => transaction?.items ?? []),
+   mapAsync(item => safeTransform(item)),
+   batch(SAFE_BATCH_SIZE),
+   retryOnError(3, exponentialBackoff),
+   collect()
+ ).catch(error => {
+   logger.error('Stream processing failed', error);
+   return fallbackProcessor(users);
+ });

Test Coverage Report:
├─ Line Coverage: 94% → 98% ✓
├─ Branch Coverage: 87% → 95% ✓
├─ Edge Cases: 5/5 covered ✓
└─ Error Paths: 3/3 tested ✓

Code Quality Metrics:
• Cyclomatic Complexity: 4 (excellent)
• Cognitive Complexity: 6 (good)
• Maintainability Index: 92 (excellent)
```

**Narration:**
"REVIEW mode focuses on quality, limiting edits to 10KB to prevent large-scale changes during review. Perfect for adding tests, fixing bugs, and ensuring code quality."

---

## Part 5: Mode Switching Flow (30 seconds)

**Quick demonstration of rapid mode switching:**

```
Press Shift+Tab → 📋 PLAN  → "What needs refactoring?"
Press Shift+Tab → 🔨 BUILD → "Implement the refactoring"  
Press Shift+Tab → 🔍 REVIEW → "Verify and add tests"
Press Shift+Tab → 📋 PLAN  → "Analyze the improvements"
```

**Final Screen:**
```
╔═══════════════════════════════════════════════════════════════╗
║  AGCodex Mode System Summary                                 ║
╠═══════════════════════════════════════════════════════════════╣
║  📋 PLAN   │ Read-only analysis & exploration                ║
║  🔨 BUILD  │ Full development capabilities                  ║
║  🔍 REVIEW │ Quality assurance & testing                   ║
╠═══════════════════════════════════════════════════════════════╣
║  Switch modes instantly with Shift+Tab                       ║
║  Each mode optimized for its development phase               ║
║  Context preserved across mode transitions                   ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## Key Takeaways

1. **Adaptive Workflow**: Modes match your development phase
2. **Safety First**: PLAN mode for exploration without risk
3. **Power When Needed**: BUILD mode for heavy lifting
4. **Quality Gates**: REVIEW mode ensures code quality
5. **Instant Switching**: Shift+Tab for seamless transitions
6. **Context Preservation**: Your work follows you between modes

## Try It Yourself

```bash
# Install AGCodex
$ cargo install agcodex

# Start with PLAN mode
$ agcodex --mode plan

# Or jump straight to BUILD
$ agcodex --mode build
```