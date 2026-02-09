extern crate proc_macro;

mod script_function;
mod script_type;

use proc_macro::TokenStream;

/// Derive macro that generates `ScriptType` trait impl for a Rust struct,
/// providing metadata needed to register it as an AngelScript type.
///
/// # Attributes
/// - `#[script(name = "Name")]` — script-visible name (defaults to struct name)
/// - `#[script(property)]` — on a field, exposes it as a script property
///
/// # Example
/// ```ignore
/// #[derive(ScriptType)]
/// #[script(name = "Player")]
/// struct Player {
///     #[script(property)]
///     health: f32,
///     #[script(property)]
///     name: String,
///     // not exposed to script:
///     internal_id: u64,
/// }
/// ```
#[proc_macro_derive(ScriptType, attributes(script))]
pub fn derive_script_type(input: TokenStream) -> TokenStream {
    script_type::derive_script_type_impl(input)
}

/// Attribute macro for registering a free function as a script-callable native function.
///
/// Generates a wrapper that adapts the Rust function to the `NativeCallContext` interface.
///
/// # Supported parameter types
/// - `i32`, `u32`, `f32`, `i64`, `u64`, `f64`, `bool`
///
/// # Supported return types
/// - `()`, `i32`, `u32`, `f32`, `i64`, `u64`, `f64`, `bool`
///
/// # Example
/// ```ignore
/// #[script_function]
/// fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
/// ```
#[proc_macro_attribute]
pub fn script_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    script_function::script_function_impl(item)
}
