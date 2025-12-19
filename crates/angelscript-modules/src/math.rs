//! Math module providing constants and functions.
//!
//! All items are in the `math` namespace, e.g., `math::PI`, `math::sin(x)`.

use angelscript_registry::Module;

// =============================================================================
// TRIGONOMETRIC FUNCTIONS
// =============================================================================

/// Sine of angle in radians (f64).
#[angelscript_macros::function]
pub fn sin(x: f64) -> f64 {
    x.sin()
}

/// Sine of angle in radians (f32).
#[angelscript_macros::function(name = "sin")]
pub fn sin_f32(x: f32) -> f32 {
    x.sin()
}

/// Cosine of angle in radians (f64).
#[angelscript_macros::function]
pub fn cos(x: f64) -> f64 {
    x.cos()
}

/// Cosine of angle in radians (f32).
#[angelscript_macros::function(name = "cos")]
pub fn cos_f32(x: f32) -> f32 {
    x.cos()
}

/// Tangent of angle in radians (f64).
#[angelscript_macros::function]
pub fn tan(x: f64) -> f64 {
    x.tan()
}

/// Tangent of angle in radians (f32).
#[angelscript_macros::function(name = "tan")]
pub fn tan_f32(x: f32) -> f32 {
    x.tan()
}

/// Arcsine (f64).
#[angelscript_macros::function]
pub fn asin(x: f64) -> f64 {
    x.asin()
}

/// Arcsine (f32).
#[angelscript_macros::function(name = "asin")]
pub fn asin_f32(x: f32) -> f32 {
    x.asin()
}

/// Arccosine (f64).
#[angelscript_macros::function]
pub fn acos(x: f64) -> f64 {
    x.acos()
}

/// Arccosine (f32).
#[angelscript_macros::function(name = "acos")]
pub fn acos_f32(x: f32) -> f32 {
    x.acos()
}

/// Arctangent (f64).
#[angelscript_macros::function]
pub fn atan(x: f64) -> f64 {
    x.atan()
}

/// Arctangent (f32).
#[angelscript_macros::function(name = "atan")]
pub fn atan_f32(x: f32) -> f32 {
    x.atan()
}

/// Two-argument arctangent (f64).
#[angelscript_macros::function]
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Two-argument arctangent (f32).
#[angelscript_macros::function(name = "atan2")]
pub fn atan2_f32(y: f32, x: f32) -> f32 {
    y.atan2(x)
}

// =============================================================================
// HYPERBOLIC FUNCTIONS
// =============================================================================

/// Hyperbolic sine (f64).
#[angelscript_macros::function]
pub fn sinh(x: f64) -> f64 {
    x.sinh()
}

/// Hyperbolic sine (f32).
#[angelscript_macros::function(name = "sinh")]
pub fn sinh_f32(x: f32) -> f32 {
    x.sinh()
}

/// Hyperbolic cosine (f64).
#[angelscript_macros::function]
pub fn cosh(x: f64) -> f64 {
    x.cosh()
}

/// Hyperbolic cosine (f32).
#[angelscript_macros::function(name = "cosh")]
pub fn cosh_f32(x: f32) -> f32 {
    x.cosh()
}

/// Hyperbolic tangent (f64).
#[angelscript_macros::function]
pub fn tanh(x: f64) -> f64 {
    x.tanh()
}

/// Hyperbolic tangent (f32).
#[angelscript_macros::function(name = "tanh")]
pub fn tanh_f32(x: f32) -> f32 {
    x.tanh()
}

/// Inverse hyperbolic sine (f64).
#[angelscript_macros::function]
pub fn asinh(x: f64) -> f64 {
    x.asinh()
}

/// Inverse hyperbolic sine (f32).
#[angelscript_macros::function(name = "asinh")]
pub fn asinh_f32(x: f32) -> f32 {
    x.asinh()
}

/// Inverse hyperbolic cosine (f64).
#[angelscript_macros::function]
pub fn acosh(x: f64) -> f64 {
    x.acosh()
}

/// Inverse hyperbolic cosine (f32).
#[angelscript_macros::function(name = "acosh")]
pub fn acosh_f32(x: f32) -> f32 {
    x.acosh()
}

/// Inverse hyperbolic tangent (f64).
#[angelscript_macros::function]
pub fn atanh(x: f64) -> f64 {
    x.atanh()
}

/// Inverse hyperbolic tangent (f32).
#[angelscript_macros::function(name = "atanh")]
pub fn atanh_f32(x: f32) -> f32 {
    x.atanh()
}

// =============================================================================
// EXPONENTIAL AND LOGARITHMIC FUNCTIONS
// =============================================================================

/// Exponential e^x (f64).
#[angelscript_macros::function]
pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// Exponential e^x (f32).
#[angelscript_macros::function(name = "exp")]
pub fn exp_f32(x: f32) -> f32 {
    x.exp()
}

/// 2^x (f64).
#[angelscript_macros::function]
pub fn exp2(x: f64) -> f64 {
    x.exp2()
}

/// 2^x (f32).
#[angelscript_macros::function(name = "exp2")]
pub fn exp2_f32(x: f32) -> f32 {
    x.exp2()
}

/// e^x - 1 (f64).
#[angelscript_macros::function(name = "expM1")]
pub fn exp_m1(x: f64) -> f64 {
    x.exp_m1()
}

/// e^x - 1 (f32).
#[angelscript_macros::function(name = "expM1")]
pub fn exp_m1_f32(x: f32) -> f32 {
    x.exp_m1()
}

/// Natural logarithm (f64).
#[angelscript_macros::function]
pub fn ln(x: f64) -> f64 {
    x.ln()
}

/// Natural logarithm (f32).
#[angelscript_macros::function(name = "ln")]
pub fn ln_f32(x: f32) -> f32 {
    x.ln()
}

/// Logarithm base 2 (f64).
#[angelscript_macros::function]
pub fn log2(x: f64) -> f64 {
    x.log2()
}

/// Logarithm base 2 (f32).
#[angelscript_macros::function(name = "log2")]
pub fn log2_f32(x: f32) -> f32 {
    x.log2()
}

/// Logarithm base 10 (f64).
#[angelscript_macros::function]
pub fn log10(x: f64) -> f64 {
    x.log10()
}

/// Logarithm base 10 (f32).
#[angelscript_macros::function(name = "log10")]
pub fn log10_f32(x: f32) -> f32 {
    x.log10()
}

/// Logarithm with specified base (f64).
#[angelscript_macros::function]
pub fn log(x: f64, base: f64) -> f64 {
    x.log(base)
}

/// Logarithm with specified base (f32).
#[angelscript_macros::function(name = "log")]
pub fn log_f32(x: f32, base: f32) -> f32 {
    x.log(base)
}

/// ln(1 + x) (f64).
#[angelscript_macros::function(name = "ln1p")]
pub fn ln_1p(x: f64) -> f64 {
    x.ln_1p()
}

/// ln(1 + x) (f32).
#[angelscript_macros::function(name = "ln1p")]
pub fn ln_1p_f32(x: f32) -> f32 {
    x.ln_1p()
}

// =============================================================================
// POWER AND ROOT FUNCTIONS
// =============================================================================

/// x raised to power y (f64).
#[angelscript_macros::function]
pub fn pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}

/// x raised to power y (f32).
#[angelscript_macros::function(name = "pow")]
pub fn pow_f32(x: f32, y: f32) -> f32 {
    x.powf(y)
}

/// Integer power (f64).
#[angelscript_macros::function(name = "powi")]
pub fn powi(x: f64, n: i32) -> f64 {
    x.powi(n)
}

/// Integer power (f32).
#[angelscript_macros::function(name = "powi")]
pub fn powi_f32(x: f32, n: i32) -> f32 {
    x.powi(n)
}

/// Square root (f64).
#[angelscript_macros::function]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Square root (f32).
#[angelscript_macros::function(name = "sqrt")]
pub fn sqrt_f32(x: f32) -> f32 {
    x.sqrt()
}

/// Cube root (f64).
#[angelscript_macros::function]
pub fn cbrt(x: f64) -> f64 {
    x.cbrt()
}

/// Cube root (f32).
#[angelscript_macros::function(name = "cbrt")]
pub fn cbrt_f32(x: f32) -> f32 {
    x.cbrt()
}

/// Hypotenuse sqrt(x² + y²) (f64).
#[angelscript_macros::function]
pub fn hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

/// Hypotenuse sqrt(x² + y²) (f32).
#[angelscript_macros::function(name = "hypot")]
pub fn hypot_f32(x: f32, y: f32) -> f32 {
    x.hypot(y)
}

// =============================================================================
// ROUNDING FUNCTIONS
// =============================================================================

/// Floor (f64).
#[angelscript_macros::function]
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// Floor (f32).
#[angelscript_macros::function(name = "floor")]
pub fn floor_f32(x: f32) -> f32 {
    x.floor()
}

/// Ceiling (f64).
#[angelscript_macros::function]
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// Ceiling (f32).
#[angelscript_macros::function(name = "ceil")]
pub fn ceil_f32(x: f32) -> f32 {
    x.ceil()
}

/// Round (f64).
#[angelscript_macros::function]
pub fn round(x: f64) -> f64 {
    x.round()
}

/// Round (f32).
#[angelscript_macros::function(name = "round")]
pub fn round_f32(x: f32) -> f32 {
    x.round()
}

/// Truncate (f64).
#[angelscript_macros::function]
pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

/// Truncate (f32).
#[angelscript_macros::function(name = "trunc")]
pub fn trunc_f32(x: f32) -> f32 {
    x.trunc()
}

/// Fractional part (f64).
#[angelscript_macros::function]
pub fn fract(x: f64) -> f64 {
    x.fract()
}

/// Fractional part (f32).
#[angelscript_macros::function(name = "fract")]
pub fn fract_f32(x: f32) -> f32 {
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

/// Absolute value (f32).
#[angelscript_macros::function(name = "abs")]
pub fn abs_f32(x: f32) -> f32 {
    x.abs()
}

/// Absolute value (i32).
#[angelscript_macros::function(name = "abs")]
pub fn abs_i32(x: i32) -> i32 {
    x.abs()
}

/// Absolute value (i64).
#[angelscript_macros::function(name = "abs")]
pub fn abs_i64(x: i64) -> i64 {
    x.abs()
}

/// Sign (f64): -1.0, 0.0, or 1.0.
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

/// Signum (f32).
#[angelscript_macros::function(name = "sign")]
pub fn sign_f32(x: f32) -> f32 {
    x.signum()
}

/// Signum (i32).
#[angelscript_macros::function(name = "sign")]
pub fn sign_i32(x: i32) -> i32 {
    x.signum()
}

/// Signum (i64).
#[angelscript_macros::function(name = "sign")]
pub fn sign_i64(x: i64) -> i64 {
    x.signum()
}

/// Copy sign of y to x (f64).
#[angelscript_macros::function(name = "copysign")]
pub fn copy_sign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

/// Copy sign of y to x (f32).
#[angelscript_macros::function(name = "copysign")]
pub fn copy_sign_f32(x: f32, y: f32) -> f32 {
    x.copysign(y)
}

// =============================================================================
// MIN/MAX/CLAMP
// =============================================================================

/// Minimum (f64).
#[angelscript_macros::function(name = "min")]
pub fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Minimum (f32).
#[angelscript_macros::function(name = "min")]
pub fn min_f32(a: f32, b: f32) -> f32 {
    a.min(b)
}

/// Minimum (i32).
#[angelscript_macros::function(name = "min")]
pub fn min_i32(a: i32, b: i32) -> i32 {
    a.min(b)
}

/// Minimum (i64).
#[angelscript_macros::function(name = "min")]
pub fn min_i64(a: i64, b: i64) -> i64 {
    a.min(b)
}

/// Minimum (u32).
#[angelscript_macros::function(name = "min")]
pub fn min_u32(a: u32, b: u32) -> u32 {
    a.min(b)
}

/// Minimum (u64).
#[angelscript_macros::function(name = "min")]
pub fn min_u64(a: u64, b: u64) -> u64 {
    a.min(b)
}

/// Maximum (f64).
#[angelscript_macros::function(name = "max")]
pub fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Maximum (f32).
#[angelscript_macros::function(name = "max")]
pub fn max_f32(a: f32, b: f32) -> f32 {
    a.max(b)
}

/// Maximum (i32).
#[angelscript_macros::function(name = "max")]
pub fn max_i32(a: i32, b: i32) -> i32 {
    a.max(b)
}

/// Maximum (i64).
#[angelscript_macros::function(name = "max")]
pub fn max_i64(a: i64, b: i64) -> i64 {
    a.max(b)
}

/// Maximum (u32).
#[angelscript_macros::function(name = "max")]
pub fn max_u32(a: u32, b: u32) -> u32 {
    a.max(b)
}

/// Maximum (u64).
#[angelscript_macros::function(name = "max")]
pub fn max_u64(a: u64, b: u64) -> u64 {
    a.max(b)
}

/// Clamp (f64).
#[angelscript_macros::function]
pub fn clamp(x: f64, min_val: f64, max_val: f64) -> f64 {
    x.clamp(min_val, max_val)
}

/// Clamp (f32).
#[angelscript_macros::function(name = "clamp")]
pub fn clamp_f32(x: f32, min_val: f32, max_val: f32) -> f32 {
    x.clamp(min_val, max_val)
}

/// Clamp (i32).
#[angelscript_macros::function(name = "clamp")]
pub fn clamp_i32(x: i32, min_val: i32, max_val: i32) -> i32 {
    x.clamp(min_val, max_val)
}

/// Clamp (i64).
#[angelscript_macros::function(name = "clamp")]
pub fn clamp_i64(x: i64, min_val: i64, max_val: i64) -> i64 {
    x.clamp(min_val, max_val)
}

/// Clamp (u32).
#[angelscript_macros::function(name = "clamp")]
pub fn clamp_u32(x: u32, min_val: u32, max_val: u32) -> u32 {
    x.clamp(min_val, max_val)
}

/// Clamp (u64).
#[angelscript_macros::function(name = "clamp")]
pub fn clamp_u64(x: u64, min_val: u64, max_val: u64) -> u64 {
    x.clamp(min_val, max_val)
}

// =============================================================================
// INTERPOLATION
// =============================================================================

/// Linear interpolation (f64).
#[angelscript_macros::function]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Linear interpolation (f32).
#[angelscript_macros::function(name = "lerp")]
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Inverse lerp (f64).
#[angelscript_macros::function(name = "invLerp")]
pub fn inv_lerp(a: f64, b: f64, x: f64) -> f64 {
    if (b - a).abs() < f64::EPSILON {
        0.0
    } else {
        (x - a) / (b - a)
    }
}

/// Inverse lerp (f32).
#[angelscript_macros::function(name = "invLerp")]
pub fn inv_lerp_f32(a: f32, b: f32, x: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        0.0
    } else {
        (x - a) / (b - a)
    }
}

/// Smooth step (f64).
#[angelscript_macros::function(name = "smoothstep")]
pub fn smooth_step(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth step (f32).
#[angelscript_macros::function(name = "smoothstep")]
pub fn smooth_step_f32(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// =============================================================================
// SPECIAL VALUE CHECKS
// =============================================================================

/// Check if x is NaN (f64).
#[angelscript_macros::function(name = "isNan")]
pub fn is_nan(x: f64) -> bool {
    x.is_nan()
}

/// Check if x is NaN (f32).
#[angelscript_macros::function(name = "isNan")]
pub fn is_nan_f32(x: f32) -> bool {
    x.is_nan()
}

/// Check if x is infinite (f64).
#[angelscript_macros::function(name = "isInfinite")]
pub fn is_infinite(x: f64) -> bool {
    x.is_infinite()
}

/// Check if x is infinite (f32).
#[angelscript_macros::function(name = "isInfinite")]
pub fn is_infinite_f32(x: f32) -> bool {
    x.is_infinite()
}

/// Check if x is finite (f64).
#[angelscript_macros::function(name = "isFinite")]
pub fn is_finite(x: f64) -> bool {
    x.is_finite()
}

/// Check if x is finite (f32).
#[angelscript_macros::function(name = "isFinite")]
pub fn is_finite_f32(x: f32) -> bool {
    x.is_finite()
}

/// Check if x is normal (f64).
#[angelscript_macros::function(name = "isNormal")]
pub fn is_normal(x: f64) -> bool {
    x.is_normal()
}

/// Check if x is normal (f32).
#[angelscript_macros::function(name = "isNormal")]
pub fn is_normal_f32(x: f32) -> bool {
    x.is_normal()
}

/// Check if x is subnormal (f64).
#[angelscript_macros::function(name = "isSubnormal")]
pub fn is_subnormal(x: f64) -> bool {
    x.is_subnormal()
}

/// Check if x is subnormal (f32).
#[angelscript_macros::function(name = "isSubnormal")]
pub fn is_subnormal_f32(x: f32) -> bool {
    x.is_subnormal()
}

/// Check if x has positive sign (f64).
#[angelscript_macros::function(name = "isSignPositive")]
pub fn is_sign_positive(x: f64) -> bool {
    x.is_sign_positive()
}

/// Check if x has positive sign (f32).
#[angelscript_macros::function(name = "isSignPositive")]
pub fn is_sign_positive_f32(x: f32) -> bool {
    x.is_sign_positive()
}

/// Check if x has negative sign (f64).
#[angelscript_macros::function(name = "isSignNegative")]
pub fn is_sign_negative(x: f64) -> bool {
    x.is_sign_negative()
}

/// Check if x has negative sign (f32).
#[angelscript_macros::function(name = "isSignNegative")]
pub fn is_sign_negative_f32(x: f32) -> bool {
    x.is_sign_negative()
}

// =============================================================================
// ANGLE CONVERSION
// =============================================================================

/// Convert degrees to radians (f64).
#[angelscript_macros::function(name = "toRadians")]
pub fn to_radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

/// Convert degrees to radians (f32).
#[angelscript_macros::function(name = "toRadians")]
pub fn to_radians_f32(degrees: f32) -> f32 {
    degrees.to_radians()
}

/// Convert radians to degrees (f64).
#[angelscript_macros::function(name = "toDegrees")]
pub fn to_degrees(radians: f64) -> f64 {
    radians.to_degrees()
}

/// Convert radians to degrees (f32).
#[angelscript_macros::function(name = "toDegrees")]
pub fn to_degrees_f32(radians: f32) -> f32 {
    radians.to_degrees()
}

// =============================================================================
// MODULO AND REMAINDER
// =============================================================================

/// Floating-point remainder (f64).
#[angelscript_macros::function(name = "fmod")]
pub fn fmod(x: f64, y: f64) -> f64 {
    x % y
}

/// Floating-point remainder (f32).
#[angelscript_macros::function(name = "fmod")]
pub fn fmod_f32(x: f32, y: f32) -> f32 {
    x % y
}

/// IEEE remainder (f64).
#[angelscript_macros::function(name = "remainder")]
pub fn ieee_remainder(x: f64, y: f64) -> f64 {
    let n = (x / y).round();
    x - n * y
}

/// IEEE remainder (f32).
#[angelscript_macros::function(name = "remainder")]
pub fn ieee_remainder_f32(x: f32, y: f32) -> f32 {
    let n = (x / y).round();
    x - n * y
}

// =============================================================================
// FUSED MULTIPLY-ADD
// =============================================================================

/// Fused multiply-add: x * a + b (f64).
#[angelscript_macros::function(name = "mulAdd")]
pub fn mul_add(x: f64, a: f64, b: f64) -> f64 {
    x.mul_add(a, b)
}

/// Fused multiply-add: x * a + b (f32).
#[angelscript_macros::function(name = "mulAdd")]
pub fn mul_add_f32(x: f32, a: f32, b: f32) -> f32 {
    x.mul_add(a, b)
}

// =============================================================================
// EUCLIDEAN DIVISION
// =============================================================================

/// Euclidean division (f64).
#[angelscript_macros::function(name = "divEuclid")]
pub fn div_euclid_f64(x: f64, y: f64) -> f64 {
    x.div_euclid(y)
}

/// Euclidean division (f32).
#[angelscript_macros::function(name = "divEuclid")]
pub fn div_euclid_f32(x: f32, y: f32) -> f32 {
    x.div_euclid(y)
}

/// Euclidean division (i32).
#[angelscript_macros::function(name = "divEuclid")]
pub fn div_euclid_i32(x: i32, y: i32) -> i32 {
    x.div_euclid(y)
}

/// Euclidean division (i64).
#[angelscript_macros::function(name = "divEuclid")]
pub fn div_euclid_i64(x: i64, y: i64) -> i64 {
    x.div_euclid(y)
}

/// Euclidean remainder (f64).
#[angelscript_macros::function(name = "remEuclid")]
pub fn rem_euclid_f64(x: f64, y: f64) -> f64 {
    x.rem_euclid(y)
}

/// Euclidean remainder (f32).
#[angelscript_macros::function(name = "remEuclid")]
pub fn rem_euclid_f32(x: f32, y: f32) -> f32 {
    x.rem_euclid(y)
}

/// Euclidean remainder (i32).
#[angelscript_macros::function(name = "remEuclid")]
pub fn rem_euclid_i32(x: i32, y: i32) -> i32 {
    x.rem_euclid(y)
}

/// Euclidean remainder (i64).
#[angelscript_macros::function(name = "remEuclid")]
pub fn rem_euclid_i64(x: i64, y: i64) -> i64 {
    x.rem_euclid(y)
}

// =============================================================================
// BIT CONVERSION
// =============================================================================

/// Convert f64 to bits.
#[angelscript_macros::function(name = "toBits")]
pub fn to_bits_f64(x: f64) -> u64 {
    x.to_bits()
}

/// Convert f32 to bits.
#[angelscript_macros::function(name = "toBits")]
pub fn to_bits_f32(x: f32) -> u32 {
    x.to_bits()
}

/// Convert bits to f64.
#[angelscript_macros::function(name = "fromBits")]
pub fn from_bits_f64(bits: u64) -> f64 {
    f64::from_bits(bits)
}

/// Convert bits to f32.
#[angelscript_macros::function(name = "fromBits")]
pub fn from_bits_f32(bits: u32) -> f32 {
    f32::from_bits(bits)
}

// =============================================================================
// MODULE CREATION
// =============================================================================

/// Creates the math module with constants and functions.
///
/// Everything is in the `math` namespace, accessible as `math::sin(x)`, `math::PI`, etc.
pub fn module() -> Module {
    Module::in_namespace(&["math"])
        // Constants (f64)
        .global("PI", std::f64::consts::PI)
        .global("E", std::f64::consts::E)
        .global("TAU", std::f64::consts::TAU)
        .global("SQRT2", std::f64::consts::SQRT_2)
        .global("LN2", std::f64::consts::LN_2)
        .global("LN10", std::f64::consts::LN_10)
        .global("INFINITY", f64::INFINITY)
        .global("NEG_INFINITY", f64::NEG_INFINITY)
        .global("EPSILON", f64::EPSILON)
        .global("FRAC_PI_2", std::f64::consts::FRAC_PI_2)
        .global("FRAC_PI_3", std::f64::consts::FRAC_PI_3)
        .global("FRAC_PI_4", std::f64::consts::FRAC_PI_4)
        .global("FRAC_PI_6", std::f64::consts::FRAC_PI_6)
        .global("FRAC_PI_8", std::f64::consts::FRAC_PI_8)
        .global("FRAC_1_PI", std::f64::consts::FRAC_1_PI)
        .global("FRAC_2_PI", std::f64::consts::FRAC_2_PI)
        .global("FRAC_2_SQRT_PI", std::f64::consts::FRAC_2_SQRT_PI)
        .global("FRAC_1_SQRT_2", std::f64::consts::FRAC_1_SQRT_2)
        .global("LOG2_E", std::f64::consts::LOG2_E)
        .global("LOG2_10", std::f64::consts::LOG2_10)
        .global("LOG10_E", std::f64::consts::LOG10_E)
        .global("LOG10_2", std::f64::consts::LOG10_2)
        .global("DBL_MIN", f64::MIN)
        .global("DBL_MAX", f64::MAX)
        .global("DBL_MIN_POSITIVE", f64::MIN_POSITIVE)
        // Constants (f32)
        .global("FLT_INFINITY", f32::INFINITY)
        .global("FLT_NEG_INFINITY", f32::NEG_INFINITY)
        .global("FLT_EPSILON", f32::EPSILON)
        .global("FLT_MIN", f32::MIN)
        .global("FLT_MAX", f32::MAX)
        .global("FLT_MIN_POSITIVE", f32::MIN_POSITIVE)
        // Trigonometric
        .function(sin)
        .function(sin_f32)
        .function(cos)
        .function(cos_f32)
        .function(tan)
        .function(tan_f32)
        .function(asin)
        .function(asin_f32)
        .function(acos)
        .function(acos_f32)
        .function(atan)
        .function(atan_f32)
        .function(atan2)
        .function(atan2_f32)
        // Hyperbolic
        .function(sinh)
        .function(sinh_f32)
        .function(cosh)
        .function(cosh_f32)
        .function(tanh)
        .function(tanh_f32)
        .function(asinh)
        .function(asinh_f32)
        .function(acosh)
        .function(acosh_f32)
        .function(atanh)
        .function(atanh_f32)
        // Exponential and logarithmic
        .function(exp)
        .function(exp_f32)
        .function(exp2)
        .function(exp2_f32)
        .function(exp_m1)
        .function(exp_m1_f32)
        .function(ln)
        .function(ln_f32)
        .function(log2)
        .function(log2_f32)
        .function(log10)
        .function(log10_f32)
        .function(log)
        .function(log_f32)
        .function(ln_1p)
        .function(ln_1p_f32)
        // Power and root
        .function(pow)
        .function(pow_f32)
        .function(powi)
        .function(powi_f32)
        .function(sqrt)
        .function(sqrt_f32)
        .function(cbrt)
        .function(cbrt_f32)
        .function(hypot)
        .function(hypot_f32)
        // Rounding
        .function(floor)
        .function(floor_f32)
        .function(ceil)
        .function(ceil_f32)
        .function(round)
        .function(round_f32)
        .function(trunc)
        .function(trunc_f32)
        .function(fract)
        .function(fract_f32)
        // Absolute value and sign
        .function(abs_f64)
        .function(abs_f32)
        .function(abs_i32)
        .function(abs_i64)
        .function(sign)
        .function(sign_f32)
        .function(sign_i32)
        .function(sign_i64)
        .function(copy_sign)
        .function(copy_sign_f32)
        // Min/max/clamp
        .function(min_f64)
        .function(min_f32)
        .function(min_i32)
        .function(min_i64)
        .function(min_u32)
        .function(min_u64)
        .function(max_f64)
        .function(max_f32)
        .function(max_i32)
        .function(max_i64)
        .function(max_u32)
        .function(max_u64)
        .function(clamp)
        .function(clamp_f32)
        .function(clamp_i32)
        .function(clamp_i64)
        .function(clamp_u32)
        .function(clamp_u64)
        // Interpolation
        .function(lerp)
        .function(lerp_f32)
        .function(inv_lerp)
        .function(inv_lerp_f32)
        .function(smooth_step)
        .function(smooth_step_f32)
        // Special values
        .function(is_nan)
        .function(is_nan_f32)
        .function(is_infinite)
        .function(is_infinite_f32)
        .function(is_finite)
        .function(is_finite_f32)
        .function(is_normal)
        .function(is_normal_f32)
        .function(is_subnormal)
        .function(is_subnormal_f32)
        .function(is_sign_positive)
        .function(is_sign_positive_f32)
        .function(is_sign_negative)
        .function(is_sign_negative_f32)
        // Angle conversion
        .function(to_radians)
        .function(to_radians_f32)
        .function(to_degrees)
        .function(to_degrees_f32)
        // Modulo and remainder
        .function(fmod)
        .function(fmod_f32)
        .function(ieee_remainder)
        .function(ieee_remainder_f32)
        // Fused multiply-add
        .function(mul_add)
        .function(mul_add_f32)
        // Euclidean division
        .function(div_euclid_f64)
        .function(div_euclid_f32)
        .function(div_euclid_i32)
        .function(div_euclid_i64)
        .function(rem_euclid_f64)
        .function(rem_euclid_f32)
        .function(rem_euclid_i32)
        .function(rem_euclid_i64)
        // Bit conversion
        .function(to_bits_f64)
        .function(to_bits_f32)
        .function(from_bits_f64)
        .function(from_bits_f32)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!m.globals.is_empty());
    }
}
