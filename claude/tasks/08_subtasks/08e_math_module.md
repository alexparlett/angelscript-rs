# Task 08e: Math Module

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08d (modules structure exists)

---

## Objective

Create the math module with constants and functions in the `math` namespace. All functions wrap Rust std methods directly.

## Files to Create/Modify

- `src/modules/math.rs` - Math constants and functions (new)
- Update `src/modules/mod.rs` - Add export

## Implementation

### src/modules/math.rs

```rust
//! Math module providing constants and functions.
//!
//! All items are in the `math` namespace, e.g., `math::PI`, `math::sin(x)`.

use crate::ffi::{Module, FfiRegistrationError};

/// Creates the math module with constants and functions.
///
/// Everything is in the `math` namespace.
pub fn math_module<'app>() -> Result<Module<'app>, FfiRegistrationError> {
    let mut module = Module::new(&["math"]);

    // =========================================================================
    // CONSTANTS
    // =========================================================================

    register_constants(&mut module)?;

    // =========================================================================
    // TRIGONOMETRIC FUNCTIONS
    // =========================================================================

    register_trig(&mut module)?;

    // =========================================================================
    // HYPERBOLIC FUNCTIONS
    // =========================================================================

    register_hyperbolic(&mut module)?;

    // =========================================================================
    // EXPONENTIAL AND LOGARITHMIC
    // =========================================================================

    register_exp_log(&mut module)?;

    // =========================================================================
    // POWER AND ROOTS
    // =========================================================================

    register_power(&mut module)?;

    // =========================================================================
    // ROUNDING
    // =========================================================================

    register_rounding(&mut module)?;

    // =========================================================================
    // ABSOLUTE VALUE AND SIGN
    // =========================================================================

    register_abs_sign(&mut module)?;

    // =========================================================================
    // MIN/MAX/CLAMP
    // =========================================================================

    register_minmax(&mut module)?;

    // =========================================================================
    // FLOATING POINT CLASSIFICATION
    // =========================================================================

    register_classification(&mut module)?;

    // =========================================================================
    // MISCELLANEOUS
    // =========================================================================

    register_misc(&mut module)?;

    Ok(module)
}

fn register_constants(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Mathematical constants (f64)
    module.register_global_property("const double PI", &std::f64::consts::PI)?;
    module.register_global_property("const double E", &std::f64::consts::E)?;
    module.register_global_property("const double TAU", &std::f64::consts::TAU)?;
    module.register_global_property("const double FRAC_PI_2", &std::f64::consts::FRAC_PI_2)?;
    module.register_global_property("const double FRAC_PI_3", &std::f64::consts::FRAC_PI_3)?;
    module.register_global_property("const double FRAC_PI_4", &std::f64::consts::FRAC_PI_4)?;
    module.register_global_property("const double FRAC_PI_6", &std::f64::consts::FRAC_PI_6)?;
    module.register_global_property("const double FRAC_PI_8", &std::f64::consts::FRAC_PI_8)?;
    module.register_global_property("const double FRAC_1_PI", &std::f64::consts::FRAC_1_PI)?;
    module.register_global_property("const double FRAC_2_PI", &std::f64::consts::FRAC_2_PI)?;
    module.register_global_property("const double FRAC_2_SQRT_PI", &std::f64::consts::FRAC_2_SQRT_PI)?;
    module.register_global_property("const double SQRT_2", &std::f64::consts::SQRT_2)?;
    module.register_global_property("const double FRAC_1_SQRT_2", &std::f64::consts::FRAC_1_SQRT_2)?;
    module.register_global_property("const double LN_2", &std::f64::consts::LN_2)?;
    module.register_global_property("const double LN_10", &std::f64::consts::LN_10)?;
    module.register_global_property("const double LOG2_E", &std::f64::consts::LOG2_E)?;
    module.register_global_property("const double LOG2_10", &std::f64::consts::LOG2_10)?;
    module.register_global_property("const double LOG10_E", &std::f64::consts::LOG10_E)?;
    module.register_global_property("const double LOG10_2", &std::f64::consts::LOG10_2)?;

    // Special f64 values
    module.register_global_property("const double INFINITY", &f64::INFINITY)?;
    module.register_global_property("const double NEG_INFINITY", &f64::NEG_INFINITY)?;
    module.register_global_property("const double NAN", &f64::NAN)?;
    module.register_global_property("const double EPSILON", &f64::EPSILON)?;
    module.register_global_property("const double DBL_MIN", &f64::MIN)?;
    module.register_global_property("const double DBL_MAX", &f64::MAX)?;
    module.register_global_property("const double DBL_MIN_POSITIVE", &f64::MIN_POSITIVE)?;

    // Special f32 values
    module.register_global_property("const float FLT_INFINITY", &f32::INFINITY)?;
    module.register_global_property("const float FLT_NEG_INFINITY", &f32::NEG_INFINITY)?;
    module.register_global_property("const float FLT_NAN", &f32::NAN)?;
    module.register_global_property("const float FLT_EPSILON", &f32::EPSILON)?;
    module.register_global_property("const float FLT_MIN", &f32::MIN)?;
    module.register_global_property("const float FLT_MAX", &f32::MAX)?;
    module.register_global_property("const float FLT_MIN_POSITIVE", &f32::MIN_POSITIVE)?;

    Ok(())
}

fn register_trig(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Basic trig (f64)
    module.register_fn("double sin(double x)", |x: f64| x.sin())?;
    module.register_fn("double cos(double x)", |x: f64| x.cos())?;
    module.register_fn("double tan(double x)", |x: f64| x.tan())?;

    // Basic trig (f32)
    module.register_fn("float sinf(float x)", |x: f32| x.sin())?;
    module.register_fn("float cosf(float x)", |x: f32| x.cos())?;
    module.register_fn("float tanf(float x)", |x: f32| x.tan())?;

    // Inverse trig (f64)
    module.register_fn("double asin(double x)", |x: f64| x.asin())?;
    module.register_fn("double acos(double x)", |x: f64| x.acos())?;
    module.register_fn("double atan(double x)", |x: f64| x.atan())?;
    module.register_fn("double atan2(double y, double x)", |y: f64, x: f64| y.atan2(x))?;

    // Inverse trig (f32)
    module.register_fn("float asinf(float x)", |x: f32| x.asin())?;
    module.register_fn("float acosf(float x)", |x: f32| x.acos())?;
    module.register_fn("float atanf(float x)", |x: f32| x.atan())?;
    module.register_fn("float atan2f(float y, float x)", |y: f32, x: f32| y.atan2(x))?;

    Ok(())
}

fn register_hyperbolic(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Hyperbolic (f64)
    module.register_fn("double sinh(double x)", |x: f64| x.sinh())?;
    module.register_fn("double cosh(double x)", |x: f64| x.cosh())?;
    module.register_fn("double tanh(double x)", |x: f64| x.tanh())?;

    // Hyperbolic (f32)
    module.register_fn("float sinhf(float x)", |x: f32| x.sinh())?;
    module.register_fn("float coshf(float x)", |x: f32| x.cosh())?;
    module.register_fn("float tanhf(float x)", |x: f32| x.tanh())?;

    // Inverse hyperbolic (f64)
    module.register_fn("double asinh(double x)", |x: f64| x.asinh())?;
    module.register_fn("double acosh(double x)", |x: f64| x.acosh())?;
    module.register_fn("double atanh(double x)", |x: f64| x.atanh())?;

    // Inverse hyperbolic (f32)
    module.register_fn("float asinhf(float x)", |x: f32| x.asinh())?;
    module.register_fn("float acoshf(float x)", |x: f32| x.acosh())?;
    module.register_fn("float atanhf(float x)", |x: f32| x.atanh())?;

    Ok(())
}

fn register_exp_log(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Exponential (f64)
    module.register_fn("double exp(double x)", |x: f64| x.exp())?;
    module.register_fn("double exp2(double x)", |x: f64| x.exp2())?;
    module.register_fn("double exp_m1(double x)", |x: f64| x.exp_m1())?;

    // Exponential (f32)
    module.register_fn("float expf(float x)", |x: f32| x.exp())?;
    module.register_fn("float exp2f(float x)", |x: f32| x.exp2())?;
    module.register_fn("float exp_m1f(float x)", |x: f32| x.exp_m1())?;

    // Logarithmic (f64)
    module.register_fn("double ln(double x)", |x: f64| x.ln())?;
    module.register_fn("double log(double x)", |x: f64| x.ln())?; // Alias
    module.register_fn("double log2(double x)", |x: f64| x.log2())?;
    module.register_fn("double log10(double x)", |x: f64| x.log10())?;
    module.register_fn("double log_base(double x, double base)", |x: f64, base: f64| x.log(base))?;
    module.register_fn("double ln_1p(double x)", |x: f64| x.ln_1p())?;

    // Logarithmic (f32)
    module.register_fn("float lnf(float x)", |x: f32| x.ln())?;
    module.register_fn("float logf(float x)", |x: f32| x.ln())?; // Alias
    module.register_fn("float log2f(float x)", |x: f32| x.log2())?;
    module.register_fn("float log10f(float x)", |x: f32| x.log10())?;
    module.register_fn("float log_basef(float x, float base)", |x: f32, base: f32| x.log(base))?;
    module.register_fn("float ln_1pf(float x)", |x: f32| x.ln_1p())?;

    Ok(())
}

fn register_power(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Power (f64)
    module.register_fn("double pow(double base, double exp)", |base: f64, exp: f64| base.powf(exp))?;
    module.register_fn("double powi(double base, int exp)", |base: f64, exp: i32| base.powi(exp))?;

    // Power (f32)
    module.register_fn("float powf(float base, float exp)", |base: f32, exp: f32| base.powf(exp))?;
    module.register_fn("float powif(float base, int exp)", |base: f32, exp: i32| base.powi(exp))?;

    // Roots (f64)
    module.register_fn("double sqrt(double x)", |x: f64| x.sqrt())?;
    module.register_fn("double cbrt(double x)", |x: f64| x.cbrt())?;
    module.register_fn("double hypot(double x, double y)", |x: f64, y: f64| x.hypot(y))?;

    // Roots (f32)
    module.register_fn("float sqrtf(float x)", |x: f32| x.sqrt())?;
    module.register_fn("float cbrtf(float x)", |x: f32| x.cbrt())?;
    module.register_fn("float hypotf(float x, float y)", |x: f32, y: f32| x.hypot(y))?;

    Ok(())
}

fn register_rounding(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Rounding (f64)
    module.register_fn("double floor(double x)", |x: f64| x.floor())?;
    module.register_fn("double ceil(double x)", |x: f64| x.ceil())?;
    module.register_fn("double round(double x)", |x: f64| x.round())?;
    module.register_fn("double trunc(double x)", |x: f64| x.trunc())?;
    module.register_fn("double fract(double x)", |x: f64| x.fract())?;

    // Rounding (f32)
    module.register_fn("float floorf(float x)", |x: f32| x.floor())?;
    module.register_fn("float ceilf(float x)", |x: f32| x.ceil())?;
    module.register_fn("float roundf(float x)", |x: f32| x.round())?;
    module.register_fn("float truncf(float x)", |x: f32| x.trunc())?;
    module.register_fn("float fractf(float x)", |x: f32| x.fract())?;

    Ok(())
}

fn register_abs_sign(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Absolute value
    module.register_fn("double abs(double x)", |x: f64| x.abs())?;
    module.register_fn("float absf(float x)", |x: f32| x.abs())?;
    module.register_fn("int abs(int x)", |x: i32| x.abs())?;
    module.register_fn("int64 abs(int64 x)", |x: i64| x.abs())?;

    // Signum
    module.register_fn("double signum(double x)", |x: f64| x.signum())?;
    module.register_fn("float signumf(float x)", |x: f32| x.signum())?;
    module.register_fn("int signum(int x)", |x: i32| x.signum())?;
    module.register_fn("int64 signum(int64 x)", |x: i64| x.signum())?;

    // Copy sign
    module.register_fn("double copysign(double x, double y)", |x: f64, y: f64| x.copysign(y))?;
    module.register_fn("float copysignf(float x, float y)", |x: f32, y: f32| x.copysign(y))?;

    Ok(())
}

fn register_minmax(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Min
    module.register_fn("double min(double a, double b)", |a: f64, b: f64| a.min(b))?;
    module.register_fn("float minf(float a, float b)", |a: f32, b: f32| a.min(b))?;
    module.register_fn("int min(int a, int b)", |a: i32, b: i32| a.min(b))?;
    module.register_fn("int64 min(int64 a, int64 b)", |a: i64, b: i64| a.min(b))?;
    module.register_fn("uint min(uint a, uint b)", |a: u32, b: u32| a.min(b))?;
    module.register_fn("uint64 min(uint64 a, uint64 b)", |a: u64, b: u64| a.min(b))?;

    // Max
    module.register_fn("double max(double a, double b)", |a: f64, b: f64| a.max(b))?;
    module.register_fn("float maxf(float a, float b)", |a: f32, b: f32| a.max(b))?;
    module.register_fn("int max(int a, int b)", |a: i32, b: i32| a.max(b))?;
    module.register_fn("int64 max(int64 a, int64 b)", |a: i64, b: i64| a.max(b))?;
    module.register_fn("uint max(uint a, uint b)", |a: u32, b: u32| a.max(b))?;
    module.register_fn("uint64 max(uint64 a, uint64 b)", |a: u64, b: u64| a.max(b))?;

    // Clamp
    module.register_fn("double clamp(double x, double min, double max)",
        |x: f64, min: f64, max: f64| x.clamp(min, max))?;
    module.register_fn("float clampf(float x, float min, float max)",
        |x: f32, min: f32, max: f32| x.clamp(min, max))?;
    module.register_fn("int clamp(int x, int min, int max)",
        |x: i32, min: i32, max: i32| x.clamp(min, max))?;
    module.register_fn("int64 clamp(int64 x, int64 min, int64 max)",
        |x: i64, min: i64, max: i64| x.clamp(min, max))?;
    module.register_fn("uint clamp(uint x, uint min, uint max)",
        |x: u32, min: u32, max: u32| x.clamp(min, max))?;
    module.register_fn("uint64 clamp(uint64 x, uint64 min, uint64 max)",
        |x: u64, min: u64, max: u64| x.clamp(min, max))?;

    Ok(())
}

fn register_classification(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Classification (f64)
    module.register_fn("bool is_nan(double x)", |x: f64| x.is_nan())?;
    module.register_fn("bool is_infinite(double x)", |x: f64| x.is_infinite())?;
    module.register_fn("bool is_finite(double x)", |x: f64| x.is_finite())?;
    module.register_fn("bool is_normal(double x)", |x: f64| x.is_normal())?;
    module.register_fn("bool is_subnormal(double x)", |x: f64| x.is_subnormal())?;
    module.register_fn("bool is_sign_positive(double x)", |x: f64| x.is_sign_positive())?;
    module.register_fn("bool is_sign_negative(double x)", |x: f64| x.is_sign_negative())?;

    // Classification (f32)
    module.register_fn("bool is_nanf(float x)", |x: f32| x.is_nan())?;
    module.register_fn("bool is_infinitef(float x)", |x: f32| x.is_infinite())?;
    module.register_fn("bool is_finitef(float x)", |x: f32| x.is_finite())?;
    module.register_fn("bool is_normalf(float x)", |x: f32| x.is_normal())?;
    module.register_fn("bool is_subnormalf(float x)", |x: f32| x.is_subnormal())?;
    module.register_fn("bool is_sign_positivef(float x)", |x: f32| x.is_sign_positive())?;
    module.register_fn("bool is_sign_negativef(float x)", |x: f32| x.is_sign_negative())?;

    Ok(())
}

fn register_misc(module: &mut Module) -> Result<(), FfiRegistrationError> {
    // Fused multiply-add
    module.register_fn("double mul_add(double x, double a, double b)",
        |x: f64, a: f64, b: f64| x.mul_add(a, b))?;
    module.register_fn("float mul_addf(float x, float a, float b)",
        |x: f32, a: f32, b: f32| x.mul_add(a, b))?;

    // Euclidean division (f64)
    module.register_fn("double div_euclid(double x, double y)", |x: f64, y: f64| x.div_euclid(y))?;
    module.register_fn("double rem_euclid(double x, double y)", |x: f64, y: f64| x.rem_euclid(y))?;

    // Euclidean division (f32)
    module.register_fn("float div_euclidf(float x, float y)", |x: f32, y: f32| x.div_euclid(y))?;
    module.register_fn("float rem_euclidf(float x, float y)", |x: f32, y: f32| x.rem_euclid(y))?;

    // Euclidean division (integers)
    module.register_fn("int div_euclid(int x, int y)", |x: i32, y: i32| x.div_euclid(y))?;
    module.register_fn("int rem_euclid(int x, int y)", |x: i32, y: i32| x.rem_euclid(y))?;
    module.register_fn("int64 div_euclid(int64 x, int64 y)", |x: i64, y: i64| x.div_euclid(y))?;
    module.register_fn("int64 rem_euclid(int64 x, int64 y)", |x: i64, y: i64| x.rem_euclid(y))?;

    // Angle conversion
    module.register_fn("double to_radians(double degrees)", |d: f64| d.to_radians())?;
    module.register_fn("double to_degrees(double radians)", |r: f64| r.to_degrees())?;
    module.register_fn("float to_radiansf(float degrees)", |d: f32| d.to_radians())?;
    module.register_fn("float to_degreesf(float radians)", |r: f32| r.to_degrees())?;

    // Bit conversion
    module.register_fn("uint64 to_bits(double x)", |x: f64| x.to_bits())?;
    module.register_fn("double from_bits(uint64 bits)", f64::from_bits)?;
    module.register_fn("uint to_bitsf(float x)", |x: f32| x.to_bits())?;
    module.register_fn("float from_bitsf(uint bits)", f32::from_bits)?;

    Ok(())
}
```

## Summary

### Constants (~26)
- Mathematical: PI, E, TAU, FRAC_PI_*, SQRT_2, LN_*, LOG*
- Special f64: INFINITY, NEG_INFINITY, NAN, EPSILON, DBL_MIN/MAX
- Special f32: FLT_INFINITY, FLT_NAN, FLT_EPSILON, FLT_MIN/MAX

### Functions (~60)
- Trig: sin, cos, tan, asin, acos, atan, atan2 (f64+f32)
- Hyperbolic: sinh, cosh, tanh, asinh, acosh, atanh (f64+f32)
- Exp/Log: exp, exp2, ln, log2, log10, log_base, exp_m1, ln_1p (f64+f32)
- Power: pow, powi, sqrt, cbrt, hypot (f64+f32)
- Rounding: floor, ceil, round, trunc, fract (f64+f32)
- Abs/Sign: abs, signum, copysign (f64+f32+i32+i64)
- Min/Max: min, max (f64+f32+i32+i64+u32+u64)
- Clamp: clamp (f64+f32+i32+i64+u32+u64)
- Classification: is_nan, is_infinite, is_finite, is_normal, is_subnormal, is_sign_positive, is_sign_negative (f64+f32)
- FMA: mul_add (f64+f32)
- Euclidean: div_euclid, rem_euclid (f64+f32+i32+i64)
- Angles: to_radians, to_degrees (f64+f32)
- Bits: to_bits, from_bits (f64+f32)

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_module_builds() {
        let module = math_module().expect("math module should build");
        // Should have many functions registered
        assert!(module.functions().len() > 50);
    }
}
```

## Acceptance Criteria

- [ ] `src/modules/math.rs` created
- [ ] All ~26 constants registered
- [ ] All ~60 functions registered
- [ ] Functions in `math` namespace
- [ ] Unit test passes
- [ ] `cargo build --lib` succeeds
