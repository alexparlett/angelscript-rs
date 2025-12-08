//! Integration tests for AngelScript proc macros.

use angelscript::{Any, TypeHash, TypeKind, ClassMeta, Behavior, FunctionMeta, Operator, InterfaceMeta, FuncdefMeta};
use angelscript::{function, interface, funcdef};
use angelscript::{RefModifier, ReturnMode, ListPatternMeta};
use angelscript::CallContext;  // For generic calling convention examples

/// Test basic `#[derive(Any)]` usage.
#[derive(Any)]
struct SimpleType {
    value: i32,
}

#[test]
fn test_simple_any_derive() {
    // Check the trait is implemented
    assert_eq!(SimpleType::type_name(), "SimpleType");
    assert_eq!(SimpleType::type_hash(), TypeHash::from_name("SimpleType"));
}

/// Test `#[derive(Any)]` with custom name.
#[derive(Any)]
#[angelscript(name = "Player")]
struct PlayerType {
    health: i32,
}

#[test]
fn test_any_derive_with_name() {
    assert_eq!(PlayerType::type_name(), "Player");
    assert_eq!(PlayerType::type_hash(), TypeHash::from_name("Player"));
}

/// Test `#[derive(Any)]` with value type.
#[derive(Any)]
#[angelscript(name = "Vec3", value)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[test]
fn test_any_derive_value_type() {
    assert_eq!(Vec3::type_name(), "Vec3");

    // Check the type metadata
    let meta = Vec3::__as_type_meta();
    assert_eq!(meta.name, "Vec3");
    assert!(meta.type_kind.is_value());
}

/// Test `#[derive(Any)]` with properties.
#[derive(Any)]
#[angelscript(name = "Entity")]
struct Entity {
    #[angelscript(get, set)]
    health: i32,

    #[angelscript(get)]
    id: u64,

    #[angelscript(get, set, name = "position")]
    pos: f32,
}

#[test]
fn test_any_derive_with_properties() {
    let meta = Entity::__as_type_meta();
    assert_eq!(meta.name, "Entity");
    assert_eq!(meta.properties.len(), 3);

    // Check health property (get + set)
    let health = meta.properties.iter().find(|p| p.name == "health").unwrap();
    assert!(health.get);
    assert!(health.set);

    // Check id property (get only)
    let id = meta.properties.iter().find(|p| p.name == "id").unwrap();
    assert!(id.get);
    assert!(!id.set);

    // Check position property (renamed)
    let pos = meta.properties.iter().find(|p| p.name == "position").unwrap();
    assert!(pos.get);
    assert!(pos.set);
}

/// Test `#[derive(Any)]` with POD type.
#[derive(Any)]
#[angelscript(name = "Color", pod)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[test]
fn test_any_derive_pod_type() {
    let meta = Color::__as_type_meta();
    assert_eq!(meta.name, "Color");
    assert!(meta.type_kind.is_value());
    assert!(meta.type_kind.is_pod());
}

/// Test `#[derive(Any)]` with reference type.
#[derive(Any)]
#[angelscript(name = "Sprite", reference)]
struct Sprite {
    texture_id: u32,
}

#[test]
fn test_any_derive_reference_type() {
    let meta = Sprite::__as_type_meta();
    assert_eq!(meta.name, "Sprite");
    assert!(meta.type_kind.is_reference());
}

// ============================================================================
// Function Macro Tests
// ============================================================================

/// Test basic global function.
#[function]
fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_function_global() {
    let meta = __as_add_numbers_meta();
    assert_eq!(meta.name, "add_numbers");
    assert_eq!(meta.params.len(), 2);
    assert_eq!(meta.params[0].name, "a");
    assert_eq!(meta.params[1].name, "b");
    assert!(!meta.is_method);
    assert!(meta.behavior.is_none());
}

/// Test struct with methods.
#[derive(Any)]
#[angelscript(name = "Counter", value)]
struct Counter {
    #[angelscript(get)]
    value: i32,
}

impl Counter {
    #[function(constructor)]
    fn new(initial: i32) -> Self {
        Counter { value: initial }
    }

    #[function(instance)]
    fn increment(&mut self) {
        self.value += 1;
    }

    #[function(instance, const)]
    fn get_value(&self) -> i32 {
        self.value
    }

    #[function(instance, operator = Operator::Add)]
    fn add(&self, other: &Counter) -> Counter {
        Counter { value: self.value + other.value }
    }
}

#[test]
fn test_function_constructor() {
    let meta = Counter::__as_new_meta();
    assert_eq!(meta.name, "new");
    assert_eq!(meta.params.len(), 1);
    assert_eq!(meta.params[0].name, "initial");
    // Constructors don't have self parameter - they create the object
    assert!(!meta.is_method);
    assert_eq!(meta.behavior, Some(Behavior::Constructor));
}

#[test]
fn test_function_instance_method() {
    let meta = Counter::__as_increment_meta();
    assert_eq!(meta.name, "increment");
    assert_eq!(meta.params.len(), 0); // self is not included
    assert!(meta.is_method);
    assert!(meta.behavior.is_none());
}

#[test]
fn test_function_const_method() {
    let meta = Counter::__as_get_value_meta();
    assert_eq!(meta.name, "get_value");
    assert!(meta.is_method);
    assert!(meta.is_const);
}

#[test]
fn test_function_operator() {
    let meta = Counter::__as_add_meta();
    assert_eq!(meta.name, "add");
    assert!(meta.is_method);
    assert_eq!(meta.behavior, Some(Behavior::Operator(Operator::Add)));
}

/// Test factory function.
#[derive(Any)]
#[angelscript(name = "Sprite2", reference)]
struct Sprite2 {
    id: u32,
}

impl Sprite2 {
    #[function(factory)]
    fn create(id: u32) -> Sprite2 {
        Sprite2 { id }
    }
}

#[test]
fn test_function_factory() {
    let meta = Sprite2::__as_create_meta();
    assert_eq!(meta.name, "create");
    assert_eq!(meta.behavior, Some(Behavior::Factory));
}

/// Test copy constructor.
impl Counter {
    #[function(constructor, copy)]
    fn copy_from(other: &Counter) -> Self {
        Counter { value: other.value }
    }
}

#[test]
fn test_function_copy_constructor() {
    let meta = Counter::__as_copy_from_meta();
    assert_eq!(meta.name, "copy_from");
    assert_eq!(meta.behavior, Some(Behavior::CopyConstructor));
}

/// Test property accessor.
impl Counter {
    #[function(instance, property, const)]
    fn get_doubled(&self) -> i32 {
        self.value * 2
    }
}

#[test]
fn test_function_property() {
    let meta = Counter::__as_get_doubled_meta();
    assert_eq!(meta.name, "get_doubled");
    assert!(meta.is_property);
    assert!(meta.is_const);
}

// ============================================================================
// Interface Macro Tests
// ============================================================================

/// Test basic interface.
#[interface]
pub trait Drawable {
    fn draw(&self);
    fn get_width(&self) -> i32;
    fn set_position(&mut self, x: f32, y: f32);
}

#[test]
fn test_interface_basic() {
    let meta = __as_Drawable_interface_meta();
    assert_eq!(meta.name, "Drawable");
    assert_eq!(meta.methods.len(), 3);
}

#[test]
fn test_interface_methods() {
    let meta = __as_Drawable_interface_meta();

    // draw is const (takes &self)
    let draw = meta.methods.iter().find(|m| m.name == "draw").unwrap();
    assert!(draw.is_const);
    assert_eq!(draw.param_types.len(), 0);

    // get_width is const
    let get_width = meta.methods.iter().find(|m| m.name == "get_width").unwrap();
    assert!(get_width.is_const);

    // set_position is not const (takes &mut self)
    let set_position = meta.methods.iter().find(|m| m.name == "set_position").unwrap();
    assert!(!set_position.is_const);
    assert_eq!(set_position.param_types.len(), 2);
}

/// Test interface with custom name.
#[interface(name = "IUpdatable")]
pub trait Updatable {
    fn update(&mut self, dt: f32);
}

#[test]
fn test_interface_custom_name() {
    let meta = __as_Updatable_interface_meta();
    assert_eq!(meta.name, "IUpdatable");
    assert_eq!(meta.type_hash, TypeHash::from_name("IUpdatable"));
}

// ============================================================================
// Funcdef Macro Tests
// ============================================================================

/// Test basic funcdef.
#[funcdef]
pub type Callback = fn(i32) -> bool;

#[test]
fn test_funcdef_basic() {
    let meta = __as_Callback_funcdef_meta();
    assert_eq!(meta.name, "Callback");
    assert_eq!(meta.param_types.len(), 1);
}

/// Test funcdef with custom name.
#[funcdef(name = "EventHandler")]
pub type MyEventHandler = fn(u32, f32) -> ();

#[test]
fn test_funcdef_custom_name() {
    let meta = __as_MyEventHandler_funcdef_meta();
    assert_eq!(meta.name, "EventHandler");
    assert_eq!(meta.type_hash, TypeHash::from_name("EventHandler"));
    assert_eq!(meta.param_types.len(), 2);
}

/// Test funcdef with no return.
#[funcdef]
pub type VoidCallback = fn();

#[test]
fn test_funcdef_void() {
    let meta = __as_VoidCallback_funcdef_meta();
    assert_eq!(meta.name, "VoidCallback");
    assert_eq!(meta.param_types.len(), 0);
}

// ============================================================================
// New Functionality Tests (Phase 4)
// ============================================================================

// --- Function Name Override Tests ---

/// Test function with explicit name override.
#[function(name = "AddIntegers")]
fn add_ints(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_function_name_override() {
    let meta = __as_add_ints_meta();
    assert_eq!(meta.name, "add_ints");  // Rust name
    assert_eq!(meta.as_name, Some("AddIntegers"));  // AngelScript name
}

// --- Property Name Tests ---

/// Test type for property tests.
#[derive(Any)]
#[angelscript(name = "PropertyTest", value)]
struct PropertyTest {
    data: i32,
}

impl PropertyTest {
    /// Property with inferred name from get_ prefix.
    #[function(instance, property, const)]
    fn get_data(&self) -> i32 {
        self.data
    }

    /// Property with inferred name from set_ prefix.
    #[function(instance, property)]
    fn set_data(&mut self, value: i32) {
        self.data = value;
    }

    /// Property with explicit name override.
    #[function(instance, property, const, property_name = "computed")]
    fn get_computed_value(&self) -> i32 {
        self.data * 2
    }
}

#[test]
fn test_property_name_inference() {
    let meta = PropertyTest::__as_get_data_meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("data"));  // Inferred from get_data
}

#[test]
fn test_property_name_inference_setter() {
    let meta = PropertyTest::__as_set_data_meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("data"));  // Inferred from set_data
}

#[test]
fn test_property_name_explicit() {
    let meta = PropertyTest::__as_get_computed_value_meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("computed"));  // Explicit override
}

// --- Generic Calling Convention Tests ---

/// Array-like type for generic calling convention examples.
#[derive(Any)]
#[angelscript(name = "DynamicArray", reference)]
struct DynamicArray {
    items: Vec<i32>,
}

impl DynamicArray {
    /// Generic calling convention with typed parameter.
    #[function(instance, generic)]
    #[param(type = i32)]
    fn push_int(&mut self, _ctx: &mut CallContext) {
        // In real implementation, would extract value from ctx
    }

    /// Generic calling convention with variable type parameter.
    #[function(instance, generic)]
    #[param(variable, in)]
    fn push_any(&mut self, _ctx: &mut CallContext) {
        // Variable type parameter
    }

    /// Generic calling convention with variadic parameter.
    #[function(instance, generic)]
    #[param(type = i32, variadic)]
    fn push_all(&mut self, _ctx: &mut CallContext) {
        // Variadic parameter
    }

    /// Multiple params with different modes.
    #[function(instance, generic)]
    #[param(type = i32, in)]
    #[param(type = i32, out)]
    fn swap(&mut self, _ctx: &mut CallContext) {
        // in and out params
    }
}

#[test]
fn test_generic_param_typed() {
    let meta = DynamicArray::__as_push_int_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    assert!(meta.generic_params[0].param_type.is_some());  // Has explicit type
    assert!(!meta.generic_params[0].is_variadic);
}

#[test]
fn test_generic_param_variable() {
    let meta = DynamicArray::__as_push_any_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    assert!(meta.generic_params[0].param_type.is_none());  // Variable type
    assert_eq!(meta.generic_params[0].ref_mode, RefModifier::In);
}

#[test]
fn test_generic_param_variadic() {
    let meta = DynamicArray::__as_push_all_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    assert!(meta.generic_params[0].is_variadic);
}

#[test]
fn test_generic_multiple_params() {
    let meta = DynamicArray::__as_swap_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 2);
    assert_eq!(meta.generic_params[0].ref_mode, RefModifier::In);
    assert_eq!(meta.generic_params[1].ref_mode, RefModifier::Out);
}

// --- Default Parameter Value Tests ---

/// Type for default parameter tests.
#[derive(Any)]
#[angelscript(name = "SearchTest", reference)]
struct SearchTest {
    items: Vec<i32>,
}

impl SearchTest {
    /// Function with default parameter value (count = -1 means all).
    #[function(instance, generic)]
    #[param(type = i32)]
    #[param(type = i32, default = "-1")]
    fn find_with_count(&self, _ctx: &mut CallContext) -> i32 {
        0
    }

    /// Function with default string parameter.
    #[function(instance, generic)]
    #[param(type = String, default = "\"\"")]
    fn search_with_pattern(&self, _ctx: &mut CallContext) {
        // Empty string default
    }

    /// Multiple parameters with defaults.
    #[function(instance, generic)]
    #[param(type = i32)]
    #[param(type = i32, default = "0")]
    #[param(type = i32, default = "-1")]
    fn slice(&self, _ctx: &mut CallContext) {
        // start, offset=0, count=-1
    }
}

#[test]
fn test_default_param_integer() {
    let meta = SearchTest::__as_find_with_count_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 2);
    assert!(meta.generic_params[0].default_value.is_none());  // No default
    assert_eq!(meta.generic_params[1].default_value, Some("-1"));  // Has default
}

#[test]
fn test_default_param_string() {
    let meta = SearchTest::__as_search_with_pattern_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    assert_eq!(meta.generic_params[0].default_value, Some("\"\""));
}

#[test]
fn test_multiple_defaults() {
    let meta = SearchTest::__as_slice_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 3);
    assert!(meta.generic_params[0].default_value.is_none());  // Required
    assert_eq!(meta.generic_params[1].default_value, Some("0"));  // Optional
    assert_eq!(meta.generic_params[2].default_value, Some("-1"));  // Optional
}

// --- Regular Parameter Defaults (non-generic calling convention) ---

/// Type for regular param default tests.
#[derive(Any)]
#[angelscript(name = "DamageTest", value)]
struct DamageTest {
    health: i32,
}

impl DamageTest {
    /// Function with default on regular parameter.
    #[function(instance)]
    fn take_damage(&mut self, #[default("5")] amount: i32) {
        self.health -= amount;
    }

    /// Multiple params with some defaults.
    #[function(instance)]
    fn take_damage_with_type(
        &mut self,
        #[default("10")] amount: i32,
        #[default("0")] damage_type: i32,
    ) {
        self.health -= amount + damage_type;
    }

    /// Mixed required and optional params.
    #[function(instance)]
    fn complex_damage(
        &mut self,
        source_id: i32,
        #[default("1")] amount: i32,
        #[default("false")] is_critical: bool,
    ) {
        let _ = (source_id, amount, is_critical);
    }
}

#[test]
fn test_regular_param_default() {
    let meta = DamageTest::__as_take_damage_meta();
    assert!(!meta.is_generic);
    assert_eq!(meta.params.len(), 1);
    assert_eq!(meta.params[0].name, "amount");
    assert_eq!(meta.params[0].default_value, Some("5"));
}

#[test]
fn test_regular_param_multiple_defaults() {
    let meta = DamageTest::__as_take_damage_with_type_meta();
    assert!(!meta.is_generic);
    assert_eq!(meta.params.len(), 2);
    assert_eq!(meta.params[0].default_value, Some("10"));
    assert_eq!(meta.params[1].default_value, Some("0"));
}

#[test]
fn test_regular_param_mixed_required_optional() {
    let meta = DamageTest::__as_complex_damage_meta();
    assert!(!meta.is_generic);
    assert_eq!(meta.params.len(), 3);
    assert!(meta.params[0].default_value.is_none());  // source_id - required
    assert_eq!(meta.params[1].default_value, Some("1"));  // amount - optional
    assert_eq!(meta.params[2].default_value, Some("false"));  // is_critical - optional
}

// --- Return Metadata Tests ---

impl DynamicArray {
    /// Return by reference.
    #[function(instance, const)]
    #[returns(ref)]
    fn get_first_ref(&self) -> &i32 {
        &self.items[0]
    }

    /// Return const reference.
    #[function(instance, const)]
    #[returns(ref, const)]
    fn get_first_const(&self) -> &i32 {
        &self.items[0]
    }

    /// Return as handle.
    #[function(instance, const)]
    #[returns(handle)]
    fn get_handle(&self) -> i32 {
        0  // Simplified
    }

    /// Variable type return (for generic calling conv).
    #[function(instance, generic)]
    #[returns(variable)]
    fn get_any(&self, _ctx: &mut CallContext) {
        // Variable return type
    }
}

#[test]
fn test_return_by_reference() {
    let meta = DynamicArray::__as_get_first_ref_meta();
    assert_eq!(meta.return_meta.mode, ReturnMode::Reference);
    assert!(!meta.return_meta.is_const);
}

#[test]
fn test_return_const_reference() {
    let meta = DynamicArray::__as_get_first_const_meta();
    assert_eq!(meta.return_meta.mode, ReturnMode::Reference);
    assert!(meta.return_meta.is_const);
}

#[test]
fn test_return_handle() {
    let meta = DynamicArray::__as_get_handle_meta();
    assert_eq!(meta.return_meta.mode, ReturnMode::Handle);
}

#[test]
fn test_return_variable() {
    let meta = DynamicArray::__as_get_any_meta();
    assert!(meta.return_meta.is_variable);
}

// --- List Pattern Tests ---

/// Dictionary type for list pattern example.
#[derive(Any)]
#[angelscript(name = "Dictionary", reference)]
struct Dictionary {
    // ...
}

impl Dictionary {
    /// List constructor with repeat pattern.
    #[function(list_construct, generic)]
    #[list_pattern(repeat = i32)]
    fn from_list_repeat(_ctx: &mut CallContext) -> Self {
        Dictionary {}
    }

    /// List constructor with repeat tuple pattern (key-value).
    #[function(list_construct, generic)]
    #[list_pattern(repeat_tuple(String, i32))]
    fn from_list_kv(_ctx: &mut CallContext) -> Self {
        Dictionary {}
    }
}

#[test]
fn test_list_pattern_repeat() {
    let meta = Dictionary::__as_from_list_repeat_meta();
    assert_eq!(meta.behavior, Some(Behavior::ListConstruct));
    assert!(meta.list_pattern.is_some());
    match &meta.list_pattern {
        Some(ListPatternMeta::Repeat(_)) => {}  // Correct
        _ => panic!("Expected Repeat pattern"),
    }
}

#[test]
fn test_list_pattern_repeat_tuple() {
    let meta = Dictionary::__as_from_list_kv_meta();
    assert!(meta.list_pattern.is_some());
    match &meta.list_pattern {
        Some(ListPatternMeta::RepeatTuple(types)) => {
            assert_eq!(types.len(), 2);
        }
        _ => panic!("Expected RepeatTuple pattern"),
    }
}

// --- Interface with Function Attributes ---

/// Interface with method name overrides.
#[interface(name = "IRenderer")]
pub trait Renderer {
    #[function(name = "Render")]
    fn render(&self);

    #[function(name = "SetViewport")]
    fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32);
}

#[test]
fn test_interface_method_name_override() {
    let meta = __as_Renderer_interface_meta();
    assert_eq!(meta.name, "IRenderer");

    // Check method names were overridden
    let render = meta.methods.iter().find(|m| m.name == "Render").unwrap();
    assert!(render.is_const);

    let set_viewport = meta.methods.iter().find(|m| m.name == "SetViewport").unwrap();
    assert!(!set_viewport.is_const);
    assert_eq!(set_viewport.param_types.len(), 4);
}
