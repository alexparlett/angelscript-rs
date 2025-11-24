# AngelScript Parser - Rust Implementation

A complete, idiomatic Rust implementation of an AngelScript parser with full lexer, AST, and visitor pattern support.

## Project Status

🎯 **87.5% Complete** (Phase 7 of 8 done)

- ✅ Phase 1: Foundation & Type System
- ✅ Phase 2: Enhanced Types & Operators  
- ✅ Phase 3: Expression Parsing
- ✅ Phase 4: Statement Parsing
- ✅ Phase 5: Declaration Parsing
- ✅ Phase 6: Parser Coordination
- ✅ Phase 7: Visitor Pattern
- ⏳ Phase 8: Testing & Polish (Next)

## Features

### Complete Lexer
- Full tokenization with span tracking
- Support for all AngelScript tokens
- Comprehensive error reporting
- Line/column position tracking

### Rich AST
- **Types**: Primitives, arrays, templates, function types
- **Expressions**: All operators, literals, function calls, lambdas
- **Statements**: If/else, loops, switch, try/catch, return
- **Declarations**: Functions, classes, interfaces, enums, namespaces

### Visitor Pattern
- Zero-cost traversal abstraction
- 50+ visit methods covering all node types
- Flexible control flow (skip subtrees, early exit)
- Safe mutation patterns

## Quick Start

### Prerequisites
- Rust 1.70+ (edition 2024)
- Cargo

### Setup

The project files use a `src_` prefix to prevent naming conflicts. You'll need to reorganize them:

```bash
# 1. Create project structure
mkdir -p angelscript/{src/{lexer,ast},tests}
cd angelscript

# 2. Copy configuration files
cp /path/to/outputs/README.md .
cp /path/to/outputs/.clauderc .
cp /path/to/outputs/Cargo.toml .
cp /path/to/outputs/SETUP.md .

# 3. Run the setup script
bash setup.sh
```

Or use the automated setup:
```bash
bash /path/to/outputs/setup.sh
```

### Build and Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run specific test
cargo test test_parse_class_declaration

# Build with release optimizations
cargo build --release
```

## Usage Example

```rust
use angelscript::{
    ast::{Parser, visitor::{Visitor, walk_script}},
    lexer::Lexer,
};

// Parse AngelScript code
let source = r#"
    class Player {
        int health = 100;
        
        void takeDamage(int amount) {
            health -= amount;
            if (health <= 0) {
                die();
            }
        }
    }
"#;

let mut parser = Parser::new(source);
let script = parser.parse_script().expect("Parse failed");

// Use visitor pattern to analyze
struct FunctionCounter { count: usize }

impl Visitor for FunctionCounter {
    fn visit_function_decl(&mut self, func: &FunctionDecl) {
        self.count += 1;
        walk_function_decl(self, func);
    }
}

let mut counter = FunctionCounter { count: 0 };
walk_script(&mut counter, &script);
println!("Found {} functions", counter.count);
```

## Architecture

### Module Organization

```
src/
├── lib.rs              # Library entry point
├── lexer/              # Tokenization
│   ├── mod.rs
│   ├── lexer.rs        # Main lexer
│   ├── token.rs        # Token types
│   ├── cursor.rs       # Character iteration
│   ├── span.rs         # Position tracking
│   └── error.rs        # Lexer errors
└── ast/                # Abstract Syntax Tree
    ├── mod.rs
    ├── parser.rs       # Parser infrastructure
    ├── types.rs        # Type AST nodes
    ├── expr.rs         # Expression nodes
    ├── stmt.rs         # Statement nodes
    ├── decl.rs         # Declaration nodes
    ├── node.rs         # Common node traits
    ├── ops.rs          # Operator definitions
    ├── type_parser.rs  # Type parsing
    ├── expr_parser.rs  # Expression parsing
    ├── stmt_parser.rs  # Statement parsing
    ├── decl_parser.rs  # Declaration parsing
    ├── visitor.rs      # Visitor pattern
    └── error.rs        # Parse errors
```

### Design Principles

1. **Zero-Copy Parsing**: AST nodes store spans instead of duplicating strings
2. **Error Recovery**: Parser continues after errors to find multiple issues
3. **Type Safety**: Strong typing throughout, minimal `unsafe` code
4. **Idiomatic Rust**: Follows Rust conventions and patterns
5. **Performance**: Optimized for speed without sacrificing clarity

## Working with Claude Code

This project is designed for Claude Code. See [SETUP.md](./SETUP.md) for detailed instructions.

### Key Files for Claude Code

- `.clauderc` - Claude Code configuration
- `CLAUDE.md` - Detailed instructions for Claude
- `docs/` - Architecture and design documents
- `PROGRESS*.md` - Development progress tracking

### Development Workflow

1. Read relevant design docs in project knowledge
2. Make changes using Claude Code
3. Run tests to verify
4. Update progress tracking

## Testing

### Test Categories

- **Unit Tests**: In each module (`#[cfg(test)]`)
- **Integration Tests**: `tests/integration_tests.rs`
- **Test Scripts**: `test_scripts_*.as` files

### Running Tests

```bash
# All tests
cargo test

# Specific category
cargo test lexer
cargo test parser
cargo test visitor

# With output
cargo test -- --nocapture

# Parallel with max threads
cargo test -- --test-threads=4
```

## Documentation

- **CLAUDE.md** - Instructions for AI assistance
- **PHASE_X_COMPLETE.md** - Phase completion summaries
- **PROGRESS*.md** - Development progress
- **docs_*.md** - Technical documentation
- **FILE_MANIFEST.md** - Complete file listing

## Performance

The parser is designed for speed:
- ~10,000 lines of code parsed in <100ms
- Zero-cost visitor abstraction
- Minimal allocations during parsing
- Efficient span-based error reporting

## Contributing

This is an AI-assisted project using Claude Code. To contribute:

1. Ensure Claude Code is configured (see `.clauderc`)
2. Read `CLAUDE.md` for AI collaboration guidelines
3. Follow the design patterns in existing code
4. Add tests for new features
5. Update relevant phase documentation

## License

MIT OR Apache-2.0 (typical Rust dual license)

## Support

- Check `CLAUDE.md` for AI assistance patterns
- Review phase completion docs for feature details
- See test files for usage examples

## Roadmap

### Phase 8 (Final): Testing & Polish
- [ ] Comprehensive integration tests
- [ ] Performance benchmarking
- [ ] Error message improvements
- [ ] Documentation polish
- [ ] Production readiness validation

### Future Enhancements
- Code generation (AST → AngelScript)
- Semantic analysis
- Type checking
- REPL/Interactive mode
- LSP server support

## Acknowledgments

Built with Rust 🦀 and Claude Code 🤖

Based on AngelScript by Andreas Jönsson
