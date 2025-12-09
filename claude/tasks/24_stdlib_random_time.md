# Task 24: Extend Standard Library with Random and Time Functions

**Status:** Not Started
**Priority:** Medium
**Dependencies:** Task 01 (Unified Type Registry - COMPLETE)

---

## Overview

Add random number generation and time functions to the standard library using the new macro-based FFI approach. These are commonly needed in game scripting and are currently stubbed out in test scripts.

---

## Functions to Implement

### Random Functions (`random` namespace)

| Function | Signature | Description |
|----------|-----------|-------------|
| `random()` | `float random()` | Random float in [0.0, 1.0) |
| `rand()` | `int rand()` | Random integer |
| `rand(max)` | `int rand(int)` | Random int in [0, max) |
| `rand(min, max)` | `int rand(int, int)` | Random int in [min, max) |
| `randomRange(min, max)` | `float randomRange(float, float)` | Random float in [min, max) |
| `seed(value)` | `void seed(uint)` | Seed the RNG |
| `RAND_MAX` | `const int` | Maximum value returned by rand() |

### Time Functions (`time` namespace)

| Function | Signature | Description |
|----------|-----------|-------------|
| `getSystemTime()` | `uint getSystemTime()` | System time in milliseconds |
| `getSystemTimeNs()` | `uint64 getSystemTimeNs()` | System time in nanoseconds |
| `getMonotonicTime()` | `uint64 getMonotonicTime()` | Monotonic time in nanoseconds |

---

## Implementation with New FFI Approach

### File: `crates/angelscript-modules/src/random.rs`

```rust
//! Random number generation module for AngelScript.
//!
//! All items are in the `random` namespace, e.g., `random::rand()`, `random::seed(42)`.

use angelscript_registry::Module;
use rand::Rng;
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<rand::rngs::StdRng> = RefCell::new(
        rand::rngs::StdRng::from_entropy()
    );
}

/// Maximum value returned by rand().
#[angelscript_macros::function(name = "RAND_MAX", const)]
pub fn rand_max() -> i32 {
    i32::MAX
}

/// Returns a random float in range [0.0, 1.0).
#[angelscript_macros::function]
pub fn random() -> f64 {
    RNG.with(|rng| rng.borrow_mut().gen())
}

/// Returns a random integer.
#[angelscript_macros::function(name = "rand")]
pub fn rand_int() -> i32 {
    RNG.with(|rng| rng.borrow_mut().gen())
}

/// Returns random int in range [0, max).
#[angelscript_macros::function(name = "rand")]
pub fn rand_max_bound(max: i32) -> i32 {
    if max <= 0 { return 0; }
    RNG.with(|rng| rng.borrow_mut().gen_range(0..max))
}

/// Returns random int in range [min, max).
#[angelscript_macros::function(name = "rand")]
pub fn rand_range(min: i32, max: i32) -> i32 {
    if max <= min { return min; }
    RNG.with(|rng| rng.borrow_mut().gen_range(min..max))
}

/// Returns random float in range [min, max).
#[angelscript_macros::function(name = "randomRange")]
pub fn random_range(min: f64, max: f64) -> f64 {
    if max <= min { return min; }
    RNG.with(|rng| rng.borrow_mut().gen_range(min..max))
}

/// Seed the random number generator.
#[angelscript_macros::function]
pub fn seed(value: u32) {
    use rand::SeedableRng;
    RNG.with(|rng| {
        *rng.borrow_mut() = rand::rngs::StdRng::seed_from_u64(value as u64);
    });
}

/// Creates the random module.
pub fn module() -> Module {
    Module::in_namespace(&["random"])
        .function(rand_max)
        .function(random)
        .function(rand_int)
        .function(rand_max_bound)
        .function(rand_range)
        .function(random_range)
        .function(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_in_range() {
        for _ in 0..100 {
            let val = __as_fn__random();
            assert!(val >= 0.0 && val < 1.0);
        }
    }

    #[test]
    fn test_rand_max_bound() {
        for _ in 0..100 {
            let val = __as_fn__rand_max_bound(10);
            assert!(val >= 0 && val < 10);
        }
    }

    #[test]
    fn test_rand_range() {
        for _ in 0..100 {
            let val = __as_fn__rand_range(5, 15);
            assert!(val >= 5 && val < 15);
        }
    }

    #[test]
    fn test_seed_reproducibility() {
        __as_fn__seed(42);
        let val1 = __as_fn__rand_int();
        __as_fn__seed(42);
        let val2 = __as_fn__rand_int();
        assert_eq!(val1, val2);
    }

    #[test]
    fn test_module_creates() {
        let m = module();
        assert_eq!(m.qualified_namespace(), "random");
        assert!(!m.functions.is_empty());
    }
}
```

### File: `crates/angelscript-modules/src/time.rs`

```rust
//! Time functions module for AngelScript.
//!
//! All items are in the `time` namespace, e.g., `time::getSystemTime()`.

use angelscript_registry::Module;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use std::sync::OnceLock;

static START_TIME: OnceLock<Instant> = OnceLock::new();

fn get_start_time() -> Instant {
    *START_TIME.get_or_init(Instant::now)
}

/// Returns system time in milliseconds since Unix epoch.
#[angelscript_macros::function(name = "getSystemTime")]
pub fn get_system_time() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(0)
}

/// Returns system time in nanoseconds since Unix epoch.
#[angelscript_macros::function(name = "getSystemTimeNs")]
pub fn get_system_time_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Returns monotonic time in nanoseconds (for measuring elapsed time).
/// This is not affected by system clock changes.
#[angelscript_macros::function(name = "getMonotonicTime")]
pub fn get_monotonic_time() -> u64 {
    get_start_time().elapsed().as_nanos() as u64
}

/// Returns elapsed time in seconds since engine start.
#[angelscript_macros::function(name = "getElapsedTime")]
pub fn get_elapsed_time() -> f64 {
    get_start_time().elapsed().as_secs_f64()
}

/// Creates the time module.
pub fn module() -> Module {
    Module::in_namespace(&["time"])
        .function(get_system_time)
        .function(get_system_time_ns)
        .function(get_monotonic_time)
        .function(get_elapsed_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_system_time_not_zero() {
        let time = __as_fn__get_system_time();
        assert!(time > 0);
    }

    #[test]
    fn test_system_time_ns_not_zero() {
        let time = __as_fn__get_system_time_ns();
        assert!(time > 0);
    }

    #[test]
    fn test_monotonic_time_increases() {
        let t1 = __as_fn__get_monotonic_time();
        sleep(Duration::from_millis(10));
        let t2 = __as_fn__get_monotonic_time();
        assert!(t2 > t1);
    }

    #[test]
    fn test_elapsed_time_positive() {
        let elapsed = __as_fn__get_elapsed_time();
        assert!(elapsed >= 0.0);
    }

    #[test]
    fn test_module_creates() {
        let m = module();
        assert_eq!(m.qualified_namespace(), "time");
        assert!(!m.functions.is_empty());
    }
}
```

---

## Session-Sized Tasks

| # | Task | Description | Dependencies | Status |
|---|------|-------------|--------------|--------|
| 1 | Add `rand` crate dependency | Add to `angelscript-modules/Cargo.toml` | None | Pending |
| 2 | Implement random module | Create `random.rs` with functions | 1 | Pending |
| 3 | Implement time module | Create `time.rs` with functions | None | Pending |
| 4 | Update lib.rs exports | Export random and time modules | 2, 3 | Pending |
| 5 | Update test scripts | Remove placeholder stubs from test_scripts | 4 | Pending |
| 6 | Integration testing | Verify modules work end-to-end | 4 | Pending |

---

## Task Details

### Task 1: Add rand crate dependency

Add to `crates/angelscript-modules/Cargo.toml`:

```toml
[dependencies]
rand = "0.8"
```

### Task 2: Implement random module

Create `crates/angelscript-modules/src/random.rs` as shown above.

Key implementation details:
- Use `thread_local!` for thread-safe RNG state
- Use `rand::rngs::StdRng` for reproducible seeding
- Handle edge cases (max <= 0, max <= min)
- Register overloaded `rand` functions with same name

### Task 3: Implement time module

Create `crates/angelscript-modules/src/time.rs` as shown above.

Key implementation details:
- Use `OnceLock` for lazy initialization of start time
- `getSystemTime()` returns wall-clock time (affected by system changes)
- `getMonotonicTime()` returns monotonic time (for duration measurement)

### Task 4: Update lib.rs exports

Update `crates/angelscript-modules/src/lib.rs`:

```rust
pub mod random;
pub mod time;

// Re-export module functions
pub use random::module as random_module;
pub use time::module as time_module;
```

### Task 5: Update test scripts

Remove placeholder stubs from:
- `test_scripts/utilities.as` (rand, RAND_MAX)
- `test_scripts/performance/large_500.as` (random)
- `test_scripts/performance/xlarge_1000.as` (random)
- `test_scripts/performance/xxlarge_5000.as` (random, getSystemTime)

### Task 6: Integration testing

Write integration tests that:
1. Load the random and time modules into a Context
2. Execute AngelScript code that uses the functions
3. Verify correct behavior

---

## Testing Strategy

### Unit Tests
- Each function has unit tests in its module
- Test edge cases (zero bounds, negative ranges, etc.)
- Test reproducibility with seeding

### Integration Tests
Add to `tests/module_tests.rs`:
```rust
#[test]
fn test_random_module() {
    let ctx = Context::with_default_modules();
    // Test random functions are available
}

#[test]
fn test_time_module() {
    let ctx = Context::with_default_modules();
    // Test time functions are available
}
```

### Test Scripts
After implementation, these test scripts should work without stubs:
- `test_scripts/utilities.as`
- Performance test scripts

---

## Cargo Features (Optional)

Consider adding feature flags for optional dependencies:

```toml
[features]
default = ["random", "time"]
random = ["rand"]
time = []
```

This allows users to exclude these modules if they don't need them.

---

## Risks & Considerations

1. **Function Overloading**: Multiple `rand` functions with same name but different arities - verify the registry handles this correctly

2. **Thread Safety**: Random state is thread-local, which is correct for scripting but means different threads get different sequences

3. **Platform Differences**: Time functions should work consistently across platforms (Windows, Linux, macOS)

4. **Precision**: `getSystemTime()` returns `u32` which wraps after ~49 days - document this limitation

---

## References

- [rand crate documentation](https://docs.rs/rand)
- [std::time documentation](https://doc.rust-lang.org/std/time/)
- [AngelScript addon:random](https://www.angelcode.com/angelscript/sdk/docs/manual/doc_addon_random.html)
