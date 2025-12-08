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
        .function(pi)
        .function(e)
        .function(tau)
        .function(sqrt2)
        .function(ln2)
        .function(ln10)
        .function(infinity)
        .function(neg_infinity)
        // Trigonometric
        .function(sin)
        .function(cos)
        .function(tan)
        .function(asin)
        .function(acos)
        .function(atan)
        .function(atan2)
        // Hyperbolic
        .function(sinh)
        .function(cosh)
        .function(tanh)
        .function(asinh)
        .function(acosh)
        .function(atanh)
        // Exponential and logarithmic
        .function(exp)
        .function(exp2)
        .function(exp_m1)
        .function(ln)
        .function(log2)
        .function(log10)
        .function(log)
        .function(ln_1p)
        // Power and root
        .function(pow)
        .function(powi)
        .function(sqrt)
        .function(cbrt)
        .function(hypot)
        // Rounding
        .function(floor)
        .function(ceil)
        .function(round)
        .function(trunc)
        .function(fract)
        // Absolute value and sign
        .function(abs_f64)
        .function(abs_i32)
        .function(abs_i64)
        .function(sign)
        .function(sign_i32)
        .function(copy_sign)
        // Min/max/clamp
        .function(min_f64)
        .function(min_i32)
        .function(max_f64)
        .function(max_i32)
        .function(clamp)
        .function(clamp_i32)
        // Interpolation
        .function(lerp)
        .function(inv_lerp)
        .function(smooth_step)
        // Special values
        .function(is_nan)
        .function(is_infinite)
        .function(is_finite)
        .function(is_normal)
        // Angle conversion
        .function(to_radians)
        .function(to_degrees)
        // Modulo and remainder
        .function(fmod)
        .function(ieee_remainder)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!((__as_fn__pi() - std::f64::consts::PI).abs() < f64::EPSILON);
        assert!((__as_fn__e() - std::f64::consts::E).abs() < f64::EPSILON);
        assert!((__as_fn__tau() - std::f64::consts::TAU).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trig() {
        assert!((__as_fn__sin(0.0)).abs() < f64::EPSILON);
        assert!((__as_fn__cos(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((__as_fn__tan(0.0)).abs() < f64::EPSILON);
        assert!((__as_fn__sin(std::f64::consts::PI / 2.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_exp_log() {
        assert!((__as_fn__exp(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((__as_fn__ln(1.0)).abs() < f64::EPSILON);
        assert!((__as_fn__ln(std::f64::consts::E) - 1.0).abs() < f64::EPSILON);
        assert!((__as_fn__log2(8.0) - 3.0).abs() < f64::EPSILON);
        assert!((__as_fn__log10(100.0) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pow_sqrt() {
        assert!((__as_fn__pow(2.0, 3.0) - 8.0).abs() < f64::EPSILON);
        assert!((__as_fn__sqrt(4.0) - 2.0).abs() < f64::EPSILON);
        assert!((__as_fn__cbrt(8.0) - 2.0).abs() < f64::EPSILON);
        assert!((__as_fn__hypot(3.0, 4.0) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding() {
        assert!((__as_fn__floor(3.7) - 3.0).abs() < f64::EPSILON);
        assert!((__as_fn__ceil(3.2) - 4.0).abs() < f64::EPSILON);
        assert!((__as_fn__round(3.5) - 4.0).abs() < f64::EPSILON);
        assert!((__as_fn__trunc(-3.7) + 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_abs_sign() {
        assert!((__as_fn__abs_f64(-5.0) - 5.0).abs() < f64::EPSILON);
        assert_eq!(__as_fn__abs_i32(-5), 5);
        assert!((__as_fn__sign(-5.0) + 1.0).abs() < f64::EPSILON);
        assert!((__as_fn__sign(5.0) - 1.0).abs() < f64::EPSILON);
        assert!(__as_fn__sign(0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_min_max_clamp() {
        assert!((__as_fn__min_f64(3.0, 5.0) - 3.0).abs() < f64::EPSILON);
        assert!((__as_fn__max_f64(3.0, 5.0) - 5.0).abs() < f64::EPSILON);
        assert!((__as_fn__clamp(10.0, 0.0, 5.0) - 5.0).abs() < f64::EPSILON);
        assert!((__as_fn__clamp(-10.0, 0.0, 5.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lerp() {
        assert!((__as_fn__lerp(0.0, 10.0, 0.5) - 5.0).abs() < f64::EPSILON);
        assert!((__as_fn__lerp(0.0, 10.0, 0.0)).abs() < f64::EPSILON);
        assert!((__as_fn__lerp(0.0, 10.0, 1.0) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_special_values() {
        assert!(__as_fn__is_nan(f64::NAN));
        assert!(__as_fn__is_infinite(f64::INFINITY));
        assert!(__as_fn__is_infinite(f64::NEG_INFINITY));
        assert!(__as_fn__is_finite(1.0));
        assert!(!__as_fn__is_finite(f64::INFINITY));
    }

    #[test]
    fn test_angle_conversion() {
        assert!((__as_fn__to_radians(180.0) - std::f64::consts::PI).abs() < f64::EPSILON);
        assert!((__as_fn__to_degrees(std::f64::consts::PI) - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_module_creates() {
        let m = module();
        assert_eq!(m.qualified_namespace(), "math");
        assert!(!m.functions.is_empty());
    }
}
