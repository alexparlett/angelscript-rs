//! AngelScript Proc Macros
//!
//! This crate provides procedural macros for ergonomic FFI type registration
//! with the AngelScript scripting engine.
//!
//! # Macros
//!
//! - `#[derive(Any)]` - Implement the `Any` trait for a type
//! - `#[angelscript::function]` - Generate function metadata
//! - `#[angelscript::interface]` - Define an interface
//! - `#[angelscript::funcdef]` - Define a function pointer type
//!
//! # Example
//!
//! ```ignore
//! use angelscript_macros::Any;
//!
//! #[derive(Any)]
//! #[angelscript(name = "Player")]
//! pub struct Player {
//!     #[angelscript(get, set)]
//!     pub health: i32,
//! }
//! ```

use proc_macro::TokenStream;

mod derive_any;
mod function;
mod interface;
mod funcdef;
mod attrs;

/// Derive the `Any` trait for a type.
///
/// This macro generates an implementation of the `Any` trait which provides
/// type identity information for registration with the AngelScript engine.
///
/// # Attributes
///
/// - `#[angelscript(name = "...")]` - Override the AngelScript type name
/// - `#[angelscript(value)]` - Mark as a value type
/// - `#[angelscript(pod)]` - Mark as a POD value type
/// - `#[angelscript(reference)]` - Mark as a reference type
/// - `#[angelscript(scoped)]` - Mark as a scoped reference type
/// - `#[angelscript(nocount)]` - Mark as a single-ref type (no ref counting)
/// - `#[angelscript(template = "<T>")]` - Mark as a template type
///
/// # Field Attributes
///
/// - `#[angelscript(get)]` - Generate getter for property
/// - `#[angelscript(set)]` - Generate setter for property
/// - `#[angelscript(get, set)]` - Generate both getter and setter
/// - `#[angelscript(name = "...")]` - Override property name
///
/// # Example
///
/// ```ignore
/// #[derive(Any)]
/// #[angelscript(name = "MyClass", value)]
/// pub struct MyClass {
///     #[angelscript(get, set)]
///     pub value: i32,
///
///     #[angelscript(get)]
///     pub id: u64,
///
///     #[angelscript(get, set, name = "count")]
///     pub internal_count: i32,
/// }
/// ```
#[proc_macro_derive(Any, attributes(angelscript))]
pub fn derive_any(input: TokenStream) -> TokenStream {
    derive_any::derive_any_impl(input)
}

/// Mark a function for registration with AngelScript.
///
/// This attribute generates function metadata that can be collected by `Module`
/// for registration with the `TypeRegistry`.
///
/// # Attributes
///
/// ## Function Types
/// - `#[angelscript::function]` - Global function
/// - `#[angelscript::function(instance)]` - Instance method
/// - `#[angelscript::function(constructor)]` - Constructor
/// - `#[angelscript::function(factory)]` - Factory function
/// - `#[angelscript::function(destructor)]` - Destructor
///
/// ## Modifiers
/// - `const` - Method is const (doesn't modify object)
/// - `property` - Virtual property accessor
/// - `operator = Operator::Add` - Operator overload
/// - `generic` - Uses generic calling convention
///
/// # Example
///
/// ```ignore
/// impl MyClass {
///     #[angelscript::function(constructor)]
///     pub fn new(value: i32) -> Self { Self { value } }
///
///     #[angelscript::function(instance)]
///     pub fn get_value(&self) -> i32 { self.value }
///
///     #[angelscript::function(instance, operator = Operator::Add)]
///     pub fn add(&self, other: &MyClass) -> MyClass { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn function(attr: TokenStream, item: TokenStream) -> TokenStream {
    function::function_impl(attr, item)
}

/// Define an AngelScript interface from a Rust trait.
///
/// This attribute transforms a Rust trait into an AngelScript interface
/// definition, generating metadata that can be used for registration.
///
/// # Attributes
///
/// - `name = "..."` - Override the AngelScript interface name
///
/// # Example
///
/// ```ignore
/// #[angelscript::interface(name = "IDrawable")]
/// pub trait Drawable {
///     fn draw(&self);
///     fn get_bounds(&self) -> Rect;
/// }
/// ```
#[proc_macro_attribute]
pub fn interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    interface::interface_impl(attr, item)
}

/// Define an AngelScript funcdef from a type alias.
///
/// This attribute creates an AngelScript function pointer type (funcdef)
/// from a Rust function type alias.
///
/// # Attributes
///
/// - `name = "..."` - Override the AngelScript funcdef name
///
/// # Example
///
/// ```ignore
/// #[angelscript::funcdef(name = "Callback")]
/// pub type MyCallback = fn(i32) -> bool;
/// ```
#[proc_macro_attribute]
pub fn funcdef(attr: TokenStream, item: TokenStream) -> TokenStream {
    funcdef::funcdef_impl(attr, item)
}
