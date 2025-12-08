//! Math module providing constants and functions.
//!
//! All items are in the `math` namespace, e.g., `math::PI`, `math::sin(x)`.

use angelscript_registry::Module;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Pi constant.
#[angelscript_macros::function(name = "PI", const)]
pub fn pi() -> f64 {
    std::f64::consts::PI
}

/// Euler's number.
#[angelscript_macros::function(name = "E", const)]
pub fn e() -> f64 {
    std::f64::consts::E
}

/// Tau (2*PI).
#[angelscript_macros::function(name = "TAU", const)]
pub fn tau() -> f64 {
    std::f64::consts::TAU
}

/// Square root of 2.
#[angelscript_macros::function(name = "SQRT2", const)]
pub fn sqrt2() -> f64 {
    std::f64::consts::SQRT_2
}

/// Natural log of 2.
#[angelscript_macros::function(name = "LN2", const)]
pub fn ln2() -> f64 {
    std::f64::consts::LN_2
}

/// Natural log of 10.
#[angelscript_macros::function(name = "LN10", const)]
pub fn ln10() -> f64 {
    std::f64::consts::LN_10
}

/// Infinity.
#[angelscript_macros::function(name = "INFINITY", const)]
pub fn infinity() -> f64 {
    f64::INFINITY
}

/// Negative infinity.
#[angelscript_macros::function(name = "NEG_INFINITY", const)]
pub fn neg_infinity() -> f64 {
    f64::NEG_INFINITY
}

// =============================================================================
// TRIGONOMETRIC FUNCTIONS
// =============================================================================

/// Sine of angle in radians.
#[angelscript_macros::function]
pub fn sin(x: f64) -> f64 {
    x.sin()
}

/// Cosine of angle in radians.
#[angelscript_macros::function]
pub fn cos(x: f64) -> f64 {
    x.cos()
}

/// Tangent of angle in radians.
#[angelscript_macros::function]
pub fn tan(x: f64) -> f64 {
    x.tan()
}

/// Arcsine - returns angle in radians.
#[angelscript_macros::function]
pub fn asin(x: f64) -> f64 {
    x.asin()
}

/// Arccosine - returns angle in radians.
#[angelscript_macros::function]
pub fn acos(x: f64) -> f64 {
    x.acos()
}

/// Arctangent - returns angle in radians.
#[angelscript_macros::function]
pub fn atan(x: f64) -> f64 {
    x.atan()
}

/// Two-argument arctangent - returns angle in radians.
#[angelscript_macros::function]
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

// =============================================================================
// HYPERBOLIC FUNCTIONS
// =============================================================================

/// Hyperbolic sine.
#[angelscript_macros::function]
pub fn sinh(x: f64) -> f64 {
    x.sinh()
}

/// Hyperbolic cosine.
#[angelscript_macros::function]
pub fn cosh(x: f64) -> f64 {
    x.cosh()
}

/// Hyperbolic tangent.
#[angelscript_macros::function]
pub fn tanh(x: f64) -> f64 {
    x.tanh()
}

/// Inverse hyperbolic sine.
#[angelscript_macros::function]
pub fn asinh(x: f64) -> f64 {
    x.asinh()
}

/// Inverse hyperbolic cosine.
#[angelscript_macros::function]
pub fn acosh(x: f64) -> f64 {
    x.acosh()
}

/// Inverse hyperbolic tangent.
#[angelscript_macros::function]
pub fn atanh(x: f64) -> f64 {
    x.atanh()
}

// =============================================================================
// EXPONENTIAL AND LOGARITHMIC FUNCTIONS
// =============================================================================

/// Exponential function e^x.
#[angelscript_macros::function]
pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// 2^x.
#[angelscript_macros::function]
pub fn exp2(x: f64) -> f64 {
    x.exp2()
}

/// e^x - 1 (more accurate for small x).
#[angelscript_macros::function(name = "expM1")]
pub fn exp_m1(x: f64) -> f64 {
    x.exp_m1()
}

/// Natural logarithm (base e).
#[angelscript_macros::function]
pub fn ln(x: f64) -> f64 {
    x.ln()
}

/// Logarithm base 2.
#[angelscript_macros::function]
pub fn log2(x: f64) -> f64 {
    x.log2()
}

/// Logarithm base 10.
#[angelscript_macros::function]
pub fn log10(x: f64) -> f64 {
    x.log10()
}

/// Logarithm with specified base.
#[angelscript_macros::function]
pub fn log(x: f64, base: f64) -> f64 {
    x.log(base)
}

/// ln(1 + x) (more accurate for small x).
#[angelscript_macros::function(name = "ln1p")]
pub fn ln_1p(x: f64) -> f64 {
    x.ln_1p()
}

// =============================================================================
// POWER AND ROOT FUNCTIONS
// =============================================================================

/// x raised to power y.
#[angelscript_macros::function]
pub fn pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}

/// Integer power.
#[angelscript_macros::function(name = "powi")]
pub fn powi(x: f64, n: i32) -> f64 {
    x.powi(n)
}

/// Square root.
#[angelscript_macros::function]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Cube root.
#[angelscript_macros::function]
pub fn cbrt(x: f64) -> f64 {
    x.cbrt()
}

/// Hypotenuse - sqrt(x² + y²).
#[angelscript_macros::function]
pub fn hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

// =============================================================================
// ROUNDING FUNCTIONS
// =============================================================================

/// Floor - largest integer <= x.
#[angelscript_macros::function]
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// Ceiling - smallest integer >= x.
#[angelscript_macros::function]
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// Round to nearest integer (ties to even).
#[angelscript_macros::function]
pub fn round(x: f64) -> f64 {
    x.round()
}

/// Truncate toward zero.
#[angelscript_macros::function]
pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

/// Fractional part.
#[angelscript_macros::function]
pub fn fract(x: f64) -> f64 {
    x.fract()
}

// =============================================================================
// ABSOLUTE VALUE AND SIGN
// =============================================================================

/// Absolute value (f64).
#[angelscript_macros::function(name = "abs")]
pub fn abs_f64(x: f64) -> f64 {
    x.abs()
}

/// Absolute value (i32).
#[angelscript_macros::function(name = "absi")]
pub fn abs_i32(x: i32) -> i32 {
    x.abs()
}

/// Absolute value (i64).
#[angelscript_macros::function(name = "absl")]
pub fn abs_i64(x: i64) -> i64 {
    x.abs()
}

/// Sign of x: -1, 0, or 1.
#[angelscript_macros::function]
pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Sign of integer: -1, 0, or 1.
#[angelscript_macros::function(name = "signi")]
pub fn sign_i32(x: i32) -> i32 {
    x.signum()
}

/// Copy sign of y to x.
#[angelscript_macros::function(name = "copysign")]
pub fn copy_sign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

// =============================================================================
// MIN/MAX/CLAMP
// =============================================================================

/// Minimum of two values (f64).
#[angelscript_macros::function(name = "min")]
pub fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Minimum of two values (i32).
#[angelscript_macros::function(name = "mini")]
pub fn min_i32(a: i32, b: i32) -> i32 {
    a.min(b)
}

/// Maximum of two values (f64).
#[angelscript_macros::function(name = "max")]
pub fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Maximum of two values (i32).
#[angelscript_macros::function(name = "maxi")]
pub fn max_i32(a: i32, b: i32) -> i32 {
    a.max(b)
}

/// Clamp value between min and max (f64).
#[angelscript_macros::function]
pub fn clamp(x: f64, min_val: f64, max_val: f64) -> f64 {
    x.clamp(min_val, max_val)
}

/// Clamp value between min and max (i32).
#[angelscript_macros::function(name = "clampi")]
pub fn clamp_i32(x: i32, min_val: i32, max_val: i32) -> i32 {
    x.clamp(min_val, max_val)
}

// =============================================================================
// INTERPOLATION
// =============================================================================

/// Linear interpolation: a + t*(b - a).
#[angelscript_macros::function]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Inverse lerp: returns t such that lerp(a, b, t) = x.
#[angelscript_macros::function(name = "invLerp")]
pub fn inv_lerp(a: f64, b: f64, x: f64) -> f64 {
    if (b - a).abs() < f64::EPSILON {
        0.0
    } else {
        (x - a) / (b - a)
    }
}

/// Smooth step interpolation (cubic Hermite).
#[angelscript_macros::function(name = "smoothstep")]
pub fn smooth_step(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// =============================================================================
// SPECIAL VALUES
// =============================================================================

/// Check if x is NaN.
#[angelscript_macros::function(name = "isNan")]
pub fn is_nan(x: f64) -> bool {
    x.is_nan()
}

/// Check if x is infinite.
#[angelscript_macros::function(name = "isInfinite")]
pub fn is_infinite(x: f64) -> bool {
    x.is_infinite()
}

/// Check if x is finite (not NaN or infinite).
#[angelscript_macros::function(name = "isFinite")]
pub fn is_finite(x: f64) -> bool {
    x.is_finite()
}

/// Check if x is normal (not zero, subnormal, infinite, or NaN).
#[angelscript_macros::function(name = "isNormal")]
pub fn is_normal(x: f64) -> bool {
    x.is_normal()
}

// =============================================================================
// ANGLE CONVERSION
// =============================================================================

/// Convert degrees to radians.
#[angelscript_macros::function(name = "toRadians")]
pub fn to_radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

/// Convert radians to degrees.
#[angelscript_macros::function(name = "toDegrees")]
pub fn to_degrees(radians: f64) -> f64 {
    radians.to_degrees()
}

// =============================================================================
// MODULO AND REMAINDER
// =============================================================================

/// Floating-point remainder (same sign as dividend).
#[angelscript_macros::function(name = "fmod")]
pub fn fmod(x: f64, y: f64) -> f64 {
    x % y
}

/// IEEE remainder (may have different sign from dividend).
#[angelscript_macros::function(name = "remainder")]
pub fn ieee_remainder(x: f64, y: f64) -> f64 {
    // IEEE 754 remainder
    let n = (x / y).round();
    x - n * y
}

// =============================================================================
// MODULE CREATION
// =============================================================================

/// Creates the math module with constants and functions.
///
/// Everything is in the `math` namespace, accessible as `math::sin(x)`, `math::PI`, etc.
pub fn module() -> Module {
    Module::in_namespace(&["math"])
        .function_meta(pi__meta)
        .function_meta(e__meta)
        .function_meta(tau__meta)
        .function_meta(sqrt2__meta)
        .function_meta(ln2__meta)
        .function_meta(ln10__meta)
        .function_meta(infinity__meta)
        .function_meta(neg_infinity__meta)
        // Trigonometric
        .function_meta(sin__meta)
        .function_meta(cos__meta)
        .function_meta(tan__meta)
        .function_meta(asin__meta)
        .function_meta(acos__meta)
        .function_meta(atan__meta)
        .function_meta(atan2__meta)
        // Hyperbolic
        .function_meta(sinh__meta)
        .function_meta(cosh__meta)
        .function_meta(tanh__meta)
        .function_meta(asinh__meta)
        .function_meta(acosh__meta)
        .function_meta(atanh__meta)
        // Exponential and logarithmic
        .function_meta(exp__meta)
        .function_meta(exp2__meta)
        .function_meta(exp_m1__meta)
        .function_meta(ln__meta)
        .function_meta(log2__meta)
        .function_meta(log10__meta)
        .function_meta(log__meta)
        .function_meta(ln_1p__meta)
        // Power and root
        .function_meta(pow__meta)
        .function_meta(powi__meta)
        .function_meta(sqrt__meta)
        .function_meta(cbrt__meta)
        .function_meta(hypot__meta)
        // Rounding
        .function_meta(floor__meta)
        .function_meta(ceil__meta)
        .function_meta(round__meta)
        .function_meta(trunc__meta)
        .function_meta(fract__meta)
        // Absolute value and sign
        .function_meta(abs_f64__meta)
        .function_meta(abs_i32__meta)
        .function_meta(abs_i64__meta)
        .function_meta(sign__meta)
        .function_meta(sign_i32__meta)
        .function_meta(copy_sign__meta)
        // Min/max/clamp
        .function_meta(min_f64__meta)
        .function_meta(min_i32__meta)
        .function_meta(max_f64__meta)
        .function_meta(max_i32__meta)
        .function_meta(clamp__meta)
        .function_meta(clamp_i32__meta)
        // Interpolation
        .function_meta(lerp__meta)
        .function_meta(inv_lerp__meta)
        .function_meta(smooth_step__meta)
        // Special values
        .function_meta(is_nan__meta)
        .function_meta(is_infinite__meta)
        .function_meta(is_finite__meta)
        .function_meta(is_normal__meta)
        // Angle conversion
        .function_meta(to_radians__meta)
        .function_meta(to_degrees__meta)
        // Modulo and remainder
        .function_meta(fmod__meta)
        .function_meta(ieee_remainder__meta)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!((pi() - std::f64::consts::PI).abs() < f64::EPSILON);
        assert!((e() - std::f64::consts::E).abs() < f64::EPSILON);
        assert!((tau() - std::f64::consts::TAU).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trig() {
        assert!((sin(0.0)).abs() < f64::EPSILON);
        assert!((cos(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((tan(0.0)).abs() < f64::EPSILON);
        assert!((sin(std::f64::consts::PI / 2.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_exp_log() {
        assert!((exp(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((ln(1.0)).abs() < f64::EPSILON);
        assert!((ln(std::f64::consts::E) - 1.0).abs() < f64::EPSILON);
        assert!((log2(8.0) - 3.0).abs() < f64::EPSILON);
        assert!((log10(100.0) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pow_sqrt() {
        assert!((pow(2.0, 3.0) - 8.0).abs() < f64::EPSILON);
        assert!((sqrt(4.0) - 2.0).abs() < f64::EPSILON);
        assert!((cbrt(8.0) - 2.0).abs() < f64::EPSILON);
        assert!((hypot(3.0, 4.0) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding() {
        assert!((floor(3.7) - 3.0).abs() < f64::EPSILON);
        assert!((ceil(3.2) - 4.0).abs() < f64::EPSILON);
        assert!((round(3.5) - 4.0).abs() < f64::EPSILON);
        assert!((trunc(-3.7) + 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_abs_sign() {
        assert!((abs_f64(-5.0) - 5.0).abs() < f64::EPSILON);
        assert_eq!(abs_i32(-5), 5);
        assert!((sign(-5.0) + 1.0).abs() < f64::EPSILON);
        assert!((sign(5.0) - 1.0).abs() < f64::EPSILON);
        assert!(sign(0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_min_max_clamp() {
        assert!((min_f64(3.0, 5.0) - 3.0).abs() < f64::EPSILON);
        assert!((max_f64(3.0, 5.0) - 5.0).abs() < f64::EPSILON);
        assert!((clamp(10.0, 0.0, 5.0) - 5.0).abs() < f64::EPSILON);
        assert!((clamp(-10.0, 0.0, 5.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < f64::EPSILON);
        assert!((lerp(0.0, 10.0, 0.0)).abs() < f64::EPSILON);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_special_values() {
        assert!(is_nan(f64::NAN));
        assert!(is_infinite(f64::INFINITY));
        assert!(is_infinite(f64::NEG_INFINITY));
        assert!(is_finite(1.0));
        assert!(!is_finite(f64::INFINITY));
    }

    #[test]
    fn test_angle_conversion() {
        assert!((to_radians(180.0) - std::f64::consts::PI).abs() < f64::EPSILON);
        assert!((to_degrees(std::f64::consts::PI) - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_module_creates() {
        let m = module();
        assert_eq!(m.qualified_namespace(), "math");
        assert!(!m.functions.is_empty());
    }
}
