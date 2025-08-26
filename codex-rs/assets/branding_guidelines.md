# AGCodex Brand Guidelines

## Brand Identity

**AGCodex** represents the convergence of AI intelligence and code expertise, embodying precision, efficiency, and developer empowerment.

### Brand Values
- **Intelligence**: AST-powered understanding that goes beyond surface-level code analysis
- **Efficiency**: TUI-native design for maximum productivity
- **Privacy**: Local-first architecture respecting developer data sovereignty
- **Flexibility**: Adaptive modes (Plan/Build/Review) for different workflows
- **Performance**: 90% compression and sub-millisecond response times

## Visual Identity

### Color Palette

#### Primary Colors
```yaml
Ocean Blue:
  HEX: #0D7EA2
  RGB: 13, 126, 162
  Terminal: Cyan (36)
  Usage: Primary brand color, headers, active states

Deep Carbon:
  HEX: #1A1A1A
  RGB: 26, 26, 26
  Terminal: Black/Gray (0/8)
  Usage: Backgrounds, code blocks, terminal emulation

Bright Accent:
  HEX: #00FFB3
  RGB: 0, 255, 179
  Terminal: Bright Green (10)
  Usage: Success states, highlights, CTAs
```

#### Secondary Colors
```yaml
Plan Purple:
  HEX: #8B5CF6
  RGB: 139, 92, 246
  Terminal: Magenta (35)
  Usage: Plan mode indicator

Build Orange:
  HEX: #F97316
  RGB: 249, 115, 22
  Terminal: Yellow (33)
  Usage: Build mode indicator

Review Green:
  HEX: #10B981
  RGB: 16, 185, 129
  Terminal: Green (32)
  Usage: Review mode indicator
```

#### Semantic Colors
```yaml
Error Red:     #EF4444 (Terminal: 31)
Warning Amber: #F59E0B (Terminal: 33)
Info Blue:     #3B82F6 (Terminal: 34)
Success Green: #22C55E (Terminal: 32)
```

### Typography

#### Terminal Fonts (Recommended)
- **Primary**: JetBrains Mono, Fira Code, Cascadia Code
- **Fallback**: SF Mono, Monaco, Consolas
- **Features**: Ligatures enabled for better code readability

#### Documentation Fonts
- **Headers**: Inter, System UI
- **Body**: Inter, System UI
- **Code**: JetBrains Mono

### Logo Usage

#### Clear Space
Maintain minimum clear space equal to the height of the 'A' character around the logo.

#### Minimum Size
- Digital: 32px height minimum
- Terminal: 3 lines minimum for ASCII art version

#### Variations
1. **Full Logo**: Complete AGCodex branding with tagline
2. **Compact**: "AGCodex" text only
3. **Icon**: [A] or ◆A◆ symbol representation
4. **ASCII**: Terminal-friendly ASCII art versions

### Visual Elements

#### Box Drawing Characters
```
Standard: ┌─┬─┐ │ │ │ ├─┼─┤ └─┴─┘
Double:   ╔═╦═╗ ║ ║ ║ ╠═╬═╣ ╚═╩═╝
Mixed:    ╒═╤═╕ │ │ │ ╞═╪═╡ ╘═╧═╛
```

#### Mode Indicators
```
📋 PLAN   - Analysis and exploration
🔨 BUILD  - Active development
🔍 REVIEW - Quality assurance
```

#### Status Symbols
```
✓ Success    ⚠ Warning    ✗ Error    ⟳ Processing
◆ Active     ○ Inactive   ● Selected  ▸ Collapsed
```

## Voice & Tone

### Brand Voice Attributes
- **Technical**: Precise, accurate, engineering-focused
- **Efficient**: Concise, direct, no fluff
- **Empowering**: Supportive, enabling, collaborative
- **Professional**: Respectful, reliable, trustworthy

### Messaging Framework

#### Tagline
"AI-Powered AST Intelligence for Coders"

#### Value Propositions
1. "90% code compression with 100% context preservation"
2. "Three modes, infinite possibilities: Plan, Build, Review"
3. "Your code, your data, your control - fully local AI"
4. "From AST to AI: Understanding code the way compilers do"

#### Elevator Pitch
"AGCodex is a local-first AI coding assistant that uses AST intelligence to understand code structure deeply. With three distinct operating modes and 90% compression rates, it delivers GPT-5 level assistance while keeping your code private and your workflow efficient."

## Application Examples

### Terminal UI Theme
```rust
// Color scheme for TUI components
const THEME: Theme = Theme {
    background: Color::Rgb(26, 26, 26),      // Deep Carbon
    primary: Color::Rgb(13, 126, 162),       // Ocean Blue
    accent: Color::Rgb(0, 255, 179),         // Bright Accent
    text: Color::Rgb(230, 230, 230),         // Light Gray
    border: Color::Rgb(64, 64, 64),          // Medium Gray
    mode_plan: Color::Rgb(139, 92, 246),     // Plan Purple
    mode_build: Color::Rgb(249, 115, 22),    // Build Orange
    mode_review: Color::Rgb(16, 185, 129),   // Review Green
};
```

### Badge Design
```
╔═══════════╗  ╔═════════════╗  ╔═══════════════╗
║ AGCodex ✓ ║  ║ AST-Powered ║  ║ Local-First 🔒 ║
╚═══════════╝  ╚═════════════╝  ╚═══════════════╝
```

### Social Media Templates
```
Twitter/X Header:
┌─────────────────────────────────┐
│ AGCodex │ AST-Powered AI Coding │
│ 📋 Plan • 🔨 Build • 🔍 Review  │
└─────────────────────────────────┘

GitHub Badge:
[![AGCodex](https://img.shields.io/badge/AGCodex-AST--Powered-0D7EA2)](https://github.com/agcodex)
```

## Brand Don'ts

- ❌ Don't use gradients in terminal environments
- ❌ Don't alter the color palette for core branding
- ❌ Don't use Comic Sans or decorative fonts
- ❌ Don't compare directly to competitors in official materials
- ❌ Don't claim capabilities beyond current features
- ❌ Don't use emojis excessively in technical documentation

## Implementation Checklist

- [ ] Apply consistent color scheme across all TUI components
- [ ] Use mode colors (Purple/Orange/Green) for state indication
- [ ] Maintain ASCII art versions for all visual elements
- [ ] Ensure readability in both light and dark terminals
- [ ] Test visual elements at different terminal sizes
- [ ] Provide fallbacks for limited color support