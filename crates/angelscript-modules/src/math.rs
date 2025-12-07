//! Math module providing constants and functions.
//!
//! All items are in the `math` namespace, e.g., `math::PI()`, `math::sin(x)`.
//!
//! # Constants
//!
//! Math constants are implemented as zero-argument functions that return the value:
//! - `math::PI()` - π (3.14159...)
//! - `math::E()` - Euler's number (2.71828...)
//! - `math::TAU()` - τ = 2π
//!
//! # Functions
//!
//! ## Trigonometric
//! - `sin`, `cos`, `tan` - Basic trig functions
//! - `asin`, `acos`, `atan`, `atan2` - Inverse trig functions
//!
//! ## Hyperbolic
//! - `sinh`, `cosh`, `tanh` - Hyperbolic functions
//! - `asinh`, `acosh`, `atanh` - Inverse hyperbolic functions
//!
//! ## Exponential and Logarithmic
//! - `exp`, `exp2` - Exponential functions
//! - `ln`, `log2`, `log10`, `log_base` - Logarithmic functions
//!
//! ## Power and Roots
//! - `pow`, `sqrt`, `cbrt`, `hypot` - Power and root functions
//!
//! ## Rounding
//! - `floor`, `ceil`, `round`, `trunc`, `fract` - Rounding functions
//!
//! ## Comparison
//! - `min`, `max`, `clamp` - Comparison functions
//! - `abs`, `signum`, `copysign` - Sign functions
//!
//! # Float Variants
//!
//! Most functions have both double (f64) and float (f32) variants.
//! Float variants end with 'f': `sinf`, `cosf`, `sqrtf`, etc.

use angelscript_module::{RegistrationError, Module};

/// Creates the math module with constants and functions.
///
/// Everything is in the `math` namespace, accessible as `math::sin(x)`, `math::PI()`, etc.
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::math_module;
///
/// let module = math_module().expect("failed to create math module");
/// // Register with engine...
/// ```
pub fn math_module<'app>() -> Result<Module<'app>, RegistrationError> {
    let mut module = Module::new(&["math"]);

    // =========================================================================
    // CONSTANTS (as zero-arg functions)
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

fn register_constants(module: &mut Module) -> Result<(), RegistrationError> {
    // Mathematical constants (f64) - implemented as functions
    module.register_fn("double PI()", || std::f64::consts::PI)?;
    module.register_fn("double E()", || std::f64::consts::E)?;
    module.register_fn("double TAU()", || std::f64::consts::TAU)?;
    module.register_fn("double FRAC_PI_2()", || std::f64::consts::FRAC_PI_2)?;
    module.register_fn("double FRAC_PI_3()", || std::f64::consts::FRAC_PI_3)?;
    module.register_fn("double FRAC_PI_4()", || std::f64::consts::FRAC_PI_4)?;
    module.register_fn("double FRAC_PI_6()", || std::f64::consts::FRAC_PI_6)?;
    module.register_fn("double FRAC_PI_8()", || std::f64::consts::FRAC_PI_8)?;
    module.register_fn("double FRAC_1_PI()", || std::f64::consts::FRAC_1_PI)?;
    module.register_fn("double FRAC_2_PI()", || std::f64::consts::FRAC_2_PI)?;
    module.register_fn("double FRAC_2_SQRT_PI()", || std::f64::consts::FRAC_2_SQRT_PI)?;
    module.register_fn("double SQRT_2()", || std::f64::consts::SQRT_2)?;
    module.register_fn("double FRAC_1_SQRT_2()", || std::f64::consts::FRAC_1_SQRT_2)?;
    module.register_fn("double LN_2()", || std::f64::consts::LN_2)?;
    module.register_fn("double LN_10()", || std::f64::consts::LN_10)?;
    module.register_fn("double LOG2_E()", || std::f64::consts::LOG2_E)?;
    module.register_fn("double LOG2_10()", || std::f64::consts::LOG2_10)?;
    module.register_fn("double LOG10_E()", || std::f64::consts::LOG10_E)?;
    module.register_fn("double LOG10_2()", || std::f64::consts::LOG10_2)?;

    // Special f64 values
    module.register_fn("double INFINITY()", || f64::INFINITY)?;
    module.register_fn("double NEG_INFINITY()", || f64::NEG_INFINITY)?;
    module.register_fn("double NAN()", || f64::NAN)?;
    module.register_fn("double EPSILON()", || f64::EPSILON)?;
    module.register_fn("double DBL_MIN()", || f64::MIN)?;
    module.register_fn("double DBL_MAX()", || f64::MAX)?;
    module.register_fn("double DBL_MIN_POSITIVE()", || f64::MIN_POSITIVE)?;

    // Special f32 values
    module.register_fn("float FLT_INFINITY()", || f32::INFINITY)?;
    module.register_fn("float FLT_NEG_INFINITY()", || f32::NEG_INFINITY)?;
    module.register_fn("float FLT_NAN()", || f32::NAN)?;
    module.register_fn("float FLT_EPSILON()", || f32::EPSILON)?;
    module.register_fn("float FLT_MIN()", || f32::MIN)?;
    module.register_fn("float FLT_MAX()", || f32::MAX)?;
    module.register_fn("float FLT_MIN_POSITIVE()", || f32::MIN_POSITIVE)?;

    Ok(())
}

fn register_trig(module: &mut Module) -> Result<(), RegistrationError> {
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

fn register_hyperbolic(module: &mut Module) -> Result<(), RegistrationError> {
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

fn register_exp_log(module: &mut Module) -> Result<(), RegistrationError> {
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

fn register_power(module: &mut Module) -> Result<(), RegistrationError> {
    // Power (f64)
    module.register_fn("double pow(double base, double exp)", |base: f64, exp: f64| {
        base.powf(exp)
    })?;
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

fn register_rounding(module: &mut Module) -> Result<(), RegistrationError> {
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

fn register_abs_sign(module: &mut Module) -> Result<(), RegistrationError> {
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

fn register_minmax(module: &mut Module) -> Result<(), RegistrationError> {
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
    module.register_fn(
        "double clamp(double x, double min, double max)",
        |x: f64, min: f64, max: f64| x.clamp(min, max),
    )?;
    module.register_fn(
        "float clampf(float x, float min, float max)",
        |x: f32, min: f32, max: f32| x.clamp(min, max),
    )?;
    module.register_fn("int clamp(int x, int min, int max)", |x: i32, min: i32, max: i32| {
        x.clamp(min, max)
    })?;
    module.register_fn(
        "int64 clamp(int64 x, int64 min, int64 max)",
        |x: i64, min: i64, max: i64| x.clamp(min, max),
    )?;
    module.register_fn(
        "uint clamp(uint x, uint min, uint max)",
        |x: u32, min: u32, max: u32| x.clamp(min, max),
    )?;
    module.register_fn(
        "uint64 clamp(uint64 x, uint64 min, uint64 max)",
        |x: u64, min: u64, max: u64| x.clamp(min, max),
    )?;

    Ok(())
}

fn register_classification(module: &mut Module) -> Result<(), RegistrationError> {
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

fn register_misc(module: &mut Module) -> Result<(), RegistrationError> {
    // Fused multiply-add
    module.register_fn(
        "double mul_add(double x, double a, double b)",
        |x: f64, a: f64, b: f64| x.mul_add(a, b),
    )?;
    module.register_fn(
        "float mul_addf(float x, float a, float b)",
        |x: f32, a: f32, b: f32| x.mul_add(a, b),
    )?;

    // Euclidean division (f64)
    module.register_fn("double div_euclid(double x, double y)", |x: f64, y: f64| {
        x.div_euclid(y)
    })?;
    module.register_fn("double rem_euclid(double x, double y)", |x: f64, y: f64| {
        x.rem_euclid(y)
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_module_creates_successfully() {
        let result = math_module();
        assert!(result.is_ok(), "math module should be created successfully");
    }

    #[test]
    fn test_math_module_in_math_namespace() {
        let module = math_module().expect("math module should build");
        assert!(!module.is_root(), "math module should not be in root namespace");
        assert_eq!(module.namespace(), &["math"]);
    }

    #[test]
    fn test_math_module_has_many_functions() {
        let module = math_module().expect("math module should build");
        // Should have constants (33) + trig (14) + hyperbolic (12) + exp/log (18) +
        // power (10) + rounding (10) + abs/sign (10) + minmax (18) + classification (14) + misc (16)
        // Total: ~155 functions
        assert!(
            module.functions().len() > 100,
            "math module should have >100 functions, got {}",
            module.functions().len()
        );
    }

    #[test]
    fn test_math_module_qualified_names() {
        let module = math_module().expect("math module should build");
        assert_eq!(module.qualified_name("sin"), "math::sin");
        assert_eq!(module.qualified_name("PI"), "math::PI");
    }

    // Constants tests
    #[test]
    fn test_constant_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"PI"), "should have PI");
        assert!(names.contains(&"E"), "should have E");
        assert!(names.contains(&"TAU"), "should have TAU");
        assert!(names.contains(&"INFINITY"), "should have INFINITY");
        assert!(names.contains(&"NAN"), "should have NAN");
    }

    // Trig tests
    #[test]
    fn test_trig_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"sin"), "should have sin");
        assert!(names.contains(&"cos"), "should have cos");
        assert!(names.contains(&"tan"), "should have tan");
        assert!(names.contains(&"asin"), "should have asin");
        assert!(names.contains(&"acos"), "should have acos");
        assert!(names.contains(&"atan"), "should have atan");
        assert!(names.contains(&"atan2"), "should have atan2");
        assert!(names.contains(&"sinf"), "should have sinf");
        assert!(names.contains(&"cosf"), "should have cosf");
    }

    // Hyperbolic tests
    #[test]
    fn test_hyperbolic_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"sinh"), "should have sinh");
        assert!(names.contains(&"cosh"), "should have cosh");
        assert!(names.contains(&"tanh"), "should have tanh");
        assert!(names.contains(&"asinh"), "should have asinh");
        assert!(names.contains(&"acosh"), "should have acosh");
        assert!(names.contains(&"atanh"), "should have atanh");
    }

    // Exp/Log tests
    #[test]
    fn test_exp_log_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"exp"), "should have exp");
        assert!(names.contains(&"exp2"), "should have exp2");
        assert!(names.contains(&"ln"), "should have ln");
        assert!(names.contains(&"log"), "should have log");
        assert!(names.contains(&"log2"), "should have log2");
        assert!(names.contains(&"log10"), "should have log10");
        assert!(names.contains(&"log_base"), "should have log_base");
    }

    // Power tests
    #[test]
    fn test_power_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"pow"), "should have pow");
        assert!(names.contains(&"powi"), "should have powi");
        assert!(names.contains(&"sqrt"), "should have sqrt");
        assert!(names.contains(&"cbrt"), "should have cbrt");
        assert!(names.contains(&"hypot"), "should have hypot");
    }

    // Rounding tests
    #[test]
    fn test_rounding_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"floor"), "should have floor");
        assert!(names.contains(&"ceil"), "should have ceil");
        assert!(names.contains(&"round"), "should have round");
        assert!(names.contains(&"trunc"), "should have trunc");
        assert!(names.contains(&"fract"), "should have fract");
    }

    // Abs/Sign tests
    #[test]
    fn test_abs_sign_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"abs"), "should have abs");
        assert!(names.contains(&"absf"), "should have absf");
        assert!(names.contains(&"signum"), "should have signum");
        assert!(names.contains(&"copysign"), "should have copysign");
    }

    // Min/Max tests
    #[test]
    fn test_minmax_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"min"), "should have min");
        assert!(names.contains(&"max"), "should have max");
        assert!(names.contains(&"clamp"), "should have clamp");
        assert!(names.contains(&"minf"), "should have minf");
        assert!(names.contains(&"maxf"), "should have maxf");
        assert!(names.contains(&"clampf"), "should have clampf");
    }

    // Classification tests
    #[test]
    fn test_classification_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"is_nan"), "should have is_nan");
        assert!(names.contains(&"is_infinite"), "should have is_infinite");
        assert!(names.contains(&"is_finite"), "should have is_finite");
        assert!(names.contains(&"is_normal"), "should have is_normal");
        assert!(names.contains(&"is_subnormal"), "should have is_subnormal");
        assert!(names.contains(&"is_sign_positive"), "should have is_sign_positive");
        assert!(names.contains(&"is_sign_negative"), "should have is_sign_negative");
    }

    // Misc tests
    #[test]
    fn test_misc_functions_exist() {
        let module = math_module().expect("math module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"mul_add"), "should have mul_add");
        assert!(names.contains(&"div_euclid"), "should have div_euclid");
        assert!(names.contains(&"rem_euclid"), "should have rem_euclid");
        assert!(names.contains(&"to_radians"), "should have to_radians");
        assert!(names.contains(&"to_degrees"), "should have to_degrees");
        assert!(names.contains(&"to_bits"), "should have to_bits");
        assert!(names.contains(&"from_bits"), "should have from_bits");
    }

    // Verify all functions have correct parameter counts
    #[test]
    fn test_function_parameter_counts() {
        let module = math_module().expect("math module should build");

        for (func, _native_fn) in module.functions() {
            let name = func.name.as_str();
            let param_count = func.params.len();

            // Constants (zero params)
            if matches!(
                name,
                "PI" | "E"
                    | "TAU"
                    | "FRAC_PI_2"
                    | "FRAC_PI_3"
                    | "FRAC_PI_4"
                    | "FRAC_PI_6"
                    | "FRAC_PI_8"
                    | "FRAC_1_PI"
                    | "FRAC_2_PI"
                    | "FRAC_2_SQRT_PI"
                    | "SQRT_2"
                    | "FRAC_1_SQRT_2"
                    | "LN_2"
                    | "LN_10"
                    | "LOG2_E"
                    | "LOG2_10"
                    | "LOG10_E"
                    | "LOG10_2"
                    | "INFINITY"
                    | "NEG_INFINITY"
                    | "NAN"
                    | "EPSILON"
                    | "DBL_MIN"
                    | "DBL_MAX"
                    | "DBL_MIN_POSITIVE"
                    | "FLT_INFINITY"
                    | "FLT_NEG_INFINITY"
                    | "FLT_NAN"
                    | "FLT_EPSILON"
                    | "FLT_MIN"
                    | "FLT_MAX"
                    | "FLT_MIN_POSITIVE"
            ) {
                assert_eq!(
                    param_count, 0,
                    "constant function {} should have 0 params",
                    name
                );
            }
            // Two-param functions
            else if matches!(
                name,
                "atan2"
                    | "atan2f"
                    | "pow"
                    | "powi"
                    | "powf"
                    | "powif"
                    | "hypot"
                    | "hypotf"
                    | "log_base"
                    | "log_basef"
                    | "copysign"
                    | "copysignf"
                    | "min"
                    | "minf"
                    | "max"
                    | "maxf"
                    | "div_euclid"
                    | "div_euclidf"
                    | "rem_euclid"
                    | "rem_euclidf"
            ) {
                assert_eq!(param_count, 2, "function {} should have 2 params", name);
            }
            // Three-param functions
            else if matches!(name, "clamp" | "clampf" | "mul_add" | "mul_addf") {
                assert_eq!(param_count, 3, "function {} should have 3 params", name);
            }
            // One-param functions (most math functions)
            else {
                assert_eq!(param_count, 1, "function {} should have 1 param", name);
            }
        }
    }

    #[test]
    fn test_count_total_functions() {
        let module = math_module().expect("math module should build");
        let count = module.functions().len();
        // Print for visibility during development
        println!("Total math functions: {}", count);
        // 33 constants + 14 trig + 12 hyperbolic + 18 exp/log + 10 power +
        // 10 rounding + 10 abs/sign + 18 minmax + 14 classification + 16 misc
        // = 155 expected
        assert!(count >= 100, "Expected at least 100 functions, got {}", count);
    }
}
