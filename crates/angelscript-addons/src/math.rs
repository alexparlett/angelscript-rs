use crate::Addon;
use angelscript_core::core::script_generic::ScriptGeneric;

macro_rules! f64_generic_wrapper {
    ($func_name:ident) => {
        paste::paste! {
            fn [<$func_name _generic>](g: &ScriptGeneric) {
                let input: f64 = g.get_address_of_arg(0).unwrap().read();
                let output: f64 = input.$func_name();
                g.get_address_of_return_location()
                    .unwrap()
                    .set::<f64>(output);
            }
        }
    };
}

// Usage for f64 methods:
f64_generic_wrapper!(cos);
f64_generic_wrapper!(sin);
f64_generic_wrapper!(tan);
f64_generic_wrapper!(acos);
f64_generic_wrapper!(asin);
f64_generic_wrapper!(atan);
f64_generic_wrapper!(cosh);
f64_generic_wrapper!(sinh);
f64_generic_wrapper!(tanh);
f64_generic_wrapper!(ln); // Note: ln instead of log
f64_generic_wrapper!(log10);
f64_generic_wrapper!(sqrt);
f64_generic_wrapper!(ceil);
f64_generic_wrapper!(abs); // Note: abs instead of fabs
f64_generic_wrapper!(floor);
f64_generic_wrapper!(fract); // Note: fract instead of fraction
f64_generic_wrapper!(exp); // e^x
f64_generic_wrapper!(exp2); // 2^x
f64_generic_wrapper!(exp_m1); // e^x - 1 (more accurate for small x)
f64_generic_wrapper!(ln_1p); // ln(1 + x) (more accurate for small x)
f64_generic_wrapper!(log2); // base-2 logarithm
f64_generic_wrapper!(to_degrees); // radians to degrees
f64_generic_wrapper!(to_radians); // degrees to radians
f64_generic_wrapper!(acosh); // inverse hyperbolic cosine
f64_generic_wrapper!(asinh); // inverse hyperbolic sine
f64_generic_wrapper!(atanh); // inverse hyperbolic tangent
f64_generic_wrapper!(round); // round to nearest integer
f64_generic_wrapper!(trunc); // truncate to integer (towards zero)
f64_generic_wrapper!(signum); // sign of the number (-1, 0, or 1)
f64_generic_wrapper!(recip); // 1/x
f64_generic_wrapper!(cbrt); // cube root

macro_rules! f64_generic_wrapper_2_args {
    ($func_name:ident) => {
        paste::paste! {
            fn [<$func_name _generic>](g: &ScriptGeneric) {
                let input1: f64 = g.get_address_of_arg(0).unwrap().read();
                let input2: f64 = g.get_address_of_arg(1).unwrap().read();
                let output: f64 = input1.$func_name(input2);
                g.get_address_of_return_location()
                    .unwrap()
                    .set::<f64>(output);
            }
        }
    };
}

// For two-argument functions:
f64_generic_wrapper_2_args!(atan2);
f64_generic_wrapper_2_args!(powf);
f64_generic_wrapper_2_args!(hypot);
f64_generic_wrapper_2_args!(copysign);
f64_generic_wrapper_2_args!(max);
f64_generic_wrapper_2_args!(min);
f64_generic_wrapper_2_args!(rem_euclid);

pub fn addon() -> Addon {
    Addon::new()
        .with_global_function("double cos(double)", cos_generic, None)
        .with_global_function("double sin(double)", sin_generic, None)
        .with_global_function("double tan(double)", tan_generic, None)
        .with_global_function("double acos(double)", acos_generic, None)
        .with_global_function("double asin(double)", asin_generic, None)
        .with_global_function("double atan(double)", atan_generic, None)
        .with_global_function("double atan2(double,double)", atan2_generic, None)
        .with_global_function("double cosh(double)", cosh_generic, None)
        .with_global_function("double sinh(double)", sinh_generic, None)
        .with_global_function("double tanh(double)", tanh_generic, None)
        .with_global_function("double ln(double)", ln_generic, None)
        .with_global_function("double log10(double)", log10_generic, None)
        .with_global_function("double powf(double, double)", powf_generic, None)
        .with_global_function("double sqrt(double)", sqrt_generic, None)
        .with_global_function("double ceil(double)", ceil_generic, None)
        .with_global_function("double abs(double)", abs_generic, None)
        .with_global_function("double floor(double)", floor_generic, None)
        .with_global_function("double fract(double)", fract_generic, None)
        .with_global_function("double exp(double)", exp_generic, None)
        .with_global_function("double exp2(double)", exp2_generic, None)
        .with_global_function("double exp_m1(double)", exp_m1_generic, None)
        .with_global_function("double ln_1p(double)", ln_1p_generic, None)
        .with_global_function("double log2(double)", log2_generic, None)
        .with_global_function("double to_degrees(double)", to_degrees_generic, None)
        .with_global_function("double to_radians(double)", to_radians_generic, None)
        .with_global_function("double acosh(double)", acosh_generic, None)
        .with_global_function("double asinh(double)", asinh_generic, None)
        .with_global_function("double atanh(double)", atanh_generic, None)
        .with_global_function("double round(double)", round_generic, None)
        .with_global_function("double trunc(double)", trunc_generic, None)
        .with_global_function("double signum(double)", signum_generic, None)
        .with_global_function("double recip(double)", recip_generic, None)
        .with_global_function("double cbrt(double)", cbrt_generic, None)
        .with_global_function("double hypot(double, double)", hypot_generic, None)
        .with_global_function("double copysign(double, double)", copysign_generic, None)
        .with_global_function("double max(double, double)", max_generic, None)
        .with_global_function("double min(double, double)", min_generic, None)
        .with_global_function(
            "double rem_euclid(double, double)",
            rem_euclid_generic,
            None,
        )
        .with_global_property("const double PI", Box::from(std::f64::consts::PI))
        .with_global_property("const double E", Box::from(std::f64::consts::E))
        .with_global_property("const double TAU", Box::from(std::f64::consts::TAU))
}
