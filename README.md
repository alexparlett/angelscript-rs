# angelscript-rs

Rust bindings for the AngelScript scripting language.

## Overview

AngelScript is a flexible cross-platform scripting library designed to allow applications to extend their functionality through external scripts. This crate provides safe Rust bindings to the AngelScript C++ library.

## Features

- Safe Rust wrapper around AngelScript C++ API
- Memory-safe script execution
- Type registration for Rust types
- Function binding support
- Cross-platform compatibility

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
angelscript-rs = "0.1.0"
```

## Quick Start

```rust
use angelscript_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    
    // Register functions, types, etc.
    
    let module = engine.get_module("MyModule", true)?;
    module.add_script_section("script", r#"
        void main() {
            print("Hello from AngelScript!");
        }
    "#)?;
    
    module.build()?;
    
    // Execute the script
    let context = engine.create_context()?;
    let function = module.get_function_by_name("main")?;
    context.prepare(function)?;
    context.execute()?;
    
    Ok(())
}
```

## Building

### Prerequisites

- Rust 1.70 or later
- CMake
- C++ compiler with C++11 support

### Build Instructions

```bash
git clone https://github.com/alexparlett/angelscript-rs.git
cd angelscript-rs
cargo build
```

## Documentation

Run `cargo doc --open` to build and view the documentation locally.

## Examples

See the `examples/` directory for more usage examples.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the BSD 0-Clause License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [AngelScript](https://www.angelcode.com/angelscript/) - The original scripting library
