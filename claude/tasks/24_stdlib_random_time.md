# Task 24: Extend Standard Library with Random and Time Functions

**Status:** Not Started
**Priority:** Medium
**Dependencies:** Task 08 (FFI builtin modules)

---

## Overview

Add random number generation and time functions to the standard library. These are commonly needed in game scripting and are currently stubbed out in test scripts.

---

## Functions to Implement

### Random Functions (math namespace or separate random namespace)

1. **`float random()`** - Returns a random float in range [0.0, 1.0)
2. **`int rand()`** - Returns a random integer
3. **`int rand(int max)`** - Returns random int in range [0, max)
4. **`int rand(int min, int max)`** - Returns random int in range [min, max)
5. **`float random(float min, float max)`** - Returns random float in range [min, max)
6. **`void seed(uint value)`** - Seed the random number generator
7. **`const int RAND_MAX`** - Maximum value returned by rand()

### Time Functions (separate time namespace)

1. **`uint getSystemTime()`** - Returns system time in milliseconds
2. **`uint64 getSystemTimeNs()`** - Returns system time in nanoseconds
3. **`double getElapsedTime()`** - Returns elapsed time since engine start

---

## Implementation Notes

### Option 1: Feature Flag

Put behind a cargo feature flag since random/time may have platform dependencies:

```toml
[features]
default = ["std-random", "std-time"]
std-random = []
std-time = []
```

### Option 2: Separate Modules

Create as separate optional modules:
- `src/modules/random.rs`
- `src/modules/time.rs`

### Random Implementation

Consider using the `rand` crate for proper random number generation:

```rust
use rand::Rng;

fn random() -> f64 {
    rand::thread_rng().gen()
}
```

### Thread Safety

Random state should be thread-local to avoid synchronization overhead.

---

## Test Script Updates

After implementation, remove placeholder stubs from:
- `test_scripts/utilities.as` (rand, RAND_MAX)
- `test_scripts/performance/large_500.as` (random)
- `test_scripts/performance/xlarge_1000.as` (random)
- `test_scripts/performance/xxlarge_5000.as` (random, getSystemTime)

---

## Acceptance Criteria

1. Random functions work correctly and produce reasonable distribution
2. Time functions return accurate system time
3. All test scripts pass without placeholder stubs
4. Feature flags allow optional inclusion
5. Documentation with usage examples
