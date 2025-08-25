#!/bin/bash
# Script to verify benchmarks compile and provide initial baseline

set -e

echo "================================================"
echo "AGCodex Benchmark Verification"
echo "================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Change to the codex-rs directory
cd "$(dirname "$0")/.."

echo -e "\n${YELLOW}Step 1: Checking Cargo.toml configuration${NC}"
if grep -q "benches" Cargo.toml; then
    echo -e "${GREEN}✓ Benches crate found in workspace${NC}"
else
    echo -e "${RED}✗ Benches crate not found in workspace${NC}"
    exit 1
fi

echo -e "\n${YELLOW}Step 2: Building benchmark dependencies${NC}"
cargo build --release -p agcodex-core -p agcodex-persistence -p agcodex-ast 2>/dev/null || {
    echo -e "${RED}✗ Failed to build dependencies${NC}"
    echo "Please ensure core, persistence, and ast crates build successfully"
    exit 1
}
echo -e "${GREEN}✓ Dependencies built successfully${NC}"

echo -e "\n${YELLOW}Step 3: Checking benchmark compilation${NC}"
BENCHMARKS=(
    "compression_bench"
    "search_bench"
    "agent_bench"
    "session_bench"
    "mode_bench"
)

for bench in "${BENCHMARKS[@]}"; do
    echo -n "  Checking $bench... "
    if cargo check --bench "$bench" -p agcodex-benchmarks 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
        echo "  Error: $bench failed to compile"
        echo "  Run 'cargo check --bench $bench -p agcodex-benchmarks' for details"
    fi
done

echo -e "\n${YELLOW}Step 4: Running quick baseline (optional)${NC}"
echo "To run a quick baseline test, execute:"
echo "  cargo bench --bench compression_bench -- --sample-size 10 --warm-up-time 1"
echo ""
echo "To run all benchmarks with full sampling:"
echo "  cargo bench"
echo ""
echo "To save a baseline for comparison:"
echo "  cargo bench -- --save-baseline initial"

echo -e "\n${GREEN}================================================${NC}"
echo -e "${GREEN}Benchmark verification complete!${NC}"
echo -e "${GREEN}================================================${NC}"

echo -e "\n${YELLOW}Performance Targets Summary:${NC}"
cat << EOF
┌─────────────────┬──────────┬────────────┬──────────┐
│ Component       │ Target   │ Acceptable │ Critical │
├─────────────────┼──────────┼────────────┼──────────┤
│ Mode Switch     │ <50ms    │ <100ms     │ >200ms   │
│ Symbol Search   │ <1ms     │ <5ms       │ >10ms    │
│ Agent Spawn     │ <100ms   │ <200ms     │ >500ms   │
│ Session Save    │ <500ms   │ <1s        │ >2s      │
│ Compression     │ >50MB/s  │ >25MB/s    │ <10MB/s  │
└─────────────────┴──────────┴────────────┴──────────┘
EOF

echo -e "\n${YELLOW}Next Steps:${NC}"
echo "1. Fix any compilation errors if present"
echo "2. Run benchmarks to establish baseline: cargo bench"
echo "3. Use results to identify optimization opportunities"
echo "4. Monitor performance in CI/CD pipeline"