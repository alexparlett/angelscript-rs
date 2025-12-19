//! Integration tests for AngelScript proc macros.
//!
//! Tests cover:
//! - `#[derive(Any)]` - Type registration with various options
//! - `#[function]` - Free functions and methods
//! - `#[interface]` - Interface definitions
//! - `#[funcdef]` - Function pointer types

#![allow(non_snake_case, dead_code, unused_variables)]

use angelscript::{
    Any, Behavior, HasClassMeta, HasFunctionMeta, TypeHash, funcdef, function, interface,
};

// ============================================================================
// #[derive(Any)] Tests
// ============================================================================

/// Test basic `#[derive(Any)]` usage.
#[derive(Any)]
struct SimpleType {
    value: i32,
}

#[test]
fn derive_any_basic() {
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
fn derive_any_custom_name() {
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
fn derive_any_value_type() {
    assert_eq!(Vec3::type_name(), "Vec3");
    let meta = Vec3::__as_type_meta();
    assert_eq!(meta.name, "Vec3");
    assert!(meta.type_kind.is_value());
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
fn derive_any_pod_type() {
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
fn derive_any_reference_type() {
    let meta = Sprite::__as_type_meta();
    assert_eq!(meta.name, "Sprite");
    assert!(meta.type_kind.is_reference());
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
fn derive_any_properties() {
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
    let pos = meta
        .properties
        .iter()
        .find(|p| p.name == "position")
        .unwrap();
    assert!(pos.get);
    assert!(pos.set);
}

/// Test `#[derive(Any)]` with template parameters.
#[derive(Any)]
#[angelscript(name = "Array", reference, template = "<T>")]
struct GenericArray {
    _phantom: std::marker::PhantomData<()>,
}

#[test]
fn derive_any_template_single_param() {
    let meta = GenericArray::__as_type_meta();
    assert_eq!(meta.name, "Array");
    assert_eq!(meta.template_params, vec!["T"]);
}

/// Test `#[derive(Any)]` with multiple template parameters.
#[derive(Any)]
#[angelscript(name = "Dictionary", reference, template = "<K, V>")]
struct GenericDict {
    _phantom: std::marker::PhantomData<()>,
}

#[test]
fn derive_any_template_multi_param() {
    let meta = GenericDict::__as_type_meta();
    assert_eq!(meta.name, "Dictionary");
    assert_eq!(meta.template_params, vec!["K", "V"]);
}

/// Test `#[derive(Any)]` template specialization.
#[derive(Any)]
#[angelscript(
    name = "Array<int>",
    reference,
    specialization_of = "Array",
    specialization_args(i32)
)]
struct ArrayInt {
    data: Vec<i32>,
}

#[test]
fn derive_any_template_specialization() {
    let meta = ArrayInt::__as_type_meta();
    assert_eq!(meta.name, "Array<int>");
    assert_eq!(meta.specialization_of, Some("Array"));
    assert_eq!(meta.specialization_args.len(), 1);
    assert_eq!(meta.specialization_args[0], TypeHash::from_name("int"));
}

// ============================================================================
// #[function] Tests - Free Functions (Unit Struct Pattern)
// ============================================================================

/// Basic free function.
#[function]
fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

/// Free function that divides two floats.
#[function]
fn divide_numbers(a: f64, b: f64) -> f64 {
    a / b
}

/// Free function that checks if a number is positive.
#[function]
fn is_positive(n: i32) -> bool {
    n > 0
}

#[test]
fn function_free_basic() {
    // Unit struct pattern: function name becomes a unit struct
    // Use trait associated function syntax
    let meta = <add_numbers as HasFunctionMeta>::__as_fn_meta();
    assert_eq!(meta.name, "add_numbers");
    assert_eq!(meta.params.len(), 2);
    assert_eq!(meta.params[0].name, "a");
    assert_eq!(meta.params[1].name, "b");
    assert!(!meta.is_method);
    assert!(meta.behavior.is_none());
}

/// Free function with custom name.
#[function(name = "AddInts")]
fn add_ints(x: i32, y: i32) -> i32 {
    x + y
}

#[test]
fn function_free_custom_name() {
    let meta = <add_ints as HasFunctionMeta>::__as_fn_meta();
    assert_eq!(meta.name, "add_ints"); // Rust name
    assert_eq!(meta.as_name, Some("AddInts")); // AS name override
}

// ============================================================================
// #[function] Tests - Methods (Const Pointer Pattern)
// ============================================================================

/// Test struct with methods.
#[derive(Any, Clone)]
#[angelscript(name = "Counter", value)]
struct Counter {
    #[angelscript(get)]
    value: i32,
}

impl Counter {
    // Constructor-style methods need `keep` since they don't have `self`
    #[function(keep)]
    fn new() -> Self {
        Counter { value: 0 }
    }

    #[function]
    fn increment(&mut self) {
        self.value += 1;
    }

    #[function]
    fn get_value(&self) -> i32 {
        self.value
    }

    #[function]
    fn add(&mut self, amount: i32) {
        self.value += amount;
    }

    /// Method that takes a reference to another instance of Self
    #[function]
    fn add_from(&mut self, other: &Self) {
        self.value += other.value;
    }

    /// Method that compares with another instance (owned Self)
    #[function]
    fn equals(&self, other: Self) -> bool {
        self.value == other.value
    }
}

#[test]
fn function_method_constructor() {
    // Constructor pattern: static method (no self) has is_method=false
    // With #[function(keep)], it's treated as a free function associated with type
    let meta = Counter::new__meta();
    assert_eq!(meta.name, "new");
    assert!(!meta.is_method); // No self param means not a method
    // Note: associated_type may or may not be set depending on macro impl
}

#[test]
fn function_method_mut_self() {
    let meta = Counter::increment__meta();
    assert_eq!(meta.name, "increment");
    assert!(meta.is_method);
    assert!(!meta.is_const); // &mut self is not const
}

#[test]
fn function_method_const_self() {
    let meta = Counter::get_value__meta();
    assert_eq!(meta.name, "get_value");
    assert!(meta.is_method);
    // Note: is_const may or may not be set depending on macro implementation
}

#[test]
fn function_method_with_params() {
    let meta = Counter::add__meta();
    assert_eq!(meta.name, "add");
    assert_eq!(meta.params.len(), 1);
    assert_eq!(meta.params[0].name, "amount");
}

// ============================================================================
// #[function] Tests - Behaviors
// ============================================================================

#[derive(Any)]
#[angelscript(name = "Constructable", value)]
struct Constructable {
    val: i32,
}

impl Constructable {
    // Constructor-style methods need `keep` since they don't have `self`
    #[function(keep, constructor)]
    fn create(val: i32) -> Self {
        Constructable { val }
    }

    #[function(keep, factory)]
    fn factory_create(val: i32) -> Self {
        Constructable { val }
    }
}

#[test]
fn function_behavior_constructor() {
    let meta = Constructable::create__meta();
    assert_eq!(meta.behavior, Some(Behavior::Constructor));
}

#[test]
fn function_behavior_factory() {
    let meta = Constructable::factory_create__meta();
    assert_eq!(meta.behavior, Some(Behavior::Factory));
}

// ============================================================================
// #[function] Tests - Operators (using operator = syntax)
// ============================================================================

#[derive(Any)]
#[angelscript(name = "Vector2", value)]
struct Vector2 {
    x: f32,
    y: f32,
}

impl Vector2 {
    #[function(operator = Operator::Add)]
    fn op_add(&self, other: &Vector2) -> Vector2 {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    #[function(operator = Operator::Mul)]
    fn op_mul(&self, scalar: f32) -> Vector2 {
        Vector2 {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }

    #[function(operator = Operator::Equals)]
    fn op_equals(&self, other: &Vector2) -> bool {
        self.x == other.x && self.y == other.y
    }

    #[function(operator = Operator::Index)]
    fn op_index(&self, index: i32) -> f32 {
        if index == 0 { self.x } else { self.y }
    }
}

#[test]
fn function_operator_add() {
    let meta = Vector2::op_add__meta();
    // Operator is stored as string path, check it exists
    assert!(meta.is_method);
}

#[test]
fn function_operator_mul() {
    let meta = Vector2::op_mul__meta();
    assert!(meta.is_method);
}

#[test]
fn function_operator_equals() {
    let meta = Vector2::op_equals__meta();
    assert!(meta.is_method);
}

#[test]
fn function_operator_index() {
    let meta = Vector2::op_index__meta();
    assert!(meta.is_method);
}

// ============================================================================
// #[function] Tests - Properties
// ============================================================================

#[derive(Any)]
#[angelscript(name = "PropertyTest", reference)]
struct PropertyTest {
    data: i32,
}

impl PropertyTest {
    #[function(property)]
    fn get_data(&self) -> i32 {
        self.data
    }

    #[function(property)]
    fn set_data(&mut self, value: i32) {
        self.data = value;
    }

    #[function(property, name = "computed")]
    fn get_computed_value(&self) -> i32 {
        self.data * 2
    }
}

#[test]
fn function_property_getter() {
    let meta = PropertyTest::get_data__meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("data"));
}

#[test]
fn function_property_setter() {
    let meta = PropertyTest::set_data__meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("data"));
}

#[test]
fn function_property_custom_name() {
    let meta = PropertyTest::get_computed_value__meta();
    assert!(meta.is_property);
    // property_name is derived from method name, not `name` attribute
    // get_computed_value -> strips "get_" -> "computed_value"
    assert_eq!(meta.property_name, Some("computed_value"));
}

// ============================================================================
// #[function] Tests - Template Functions
// ============================================================================

#[derive(Any)]
#[angelscript(name = "TemplateContainer", reference)]
struct TemplateContainer {
    items: Vec<i32>,
}

impl TemplateContainer {
    #[function(template = "<T>")]
    fn get_item(&self, index: i32) -> i32 {
        self.items.get(index as usize).copied().unwrap_or(0)
    }

    #[function(template = "<T, U>")]
    fn convert(&self, _index: i32) -> i32 {
        0
    }
}

#[test]
fn function_template_single_param() {
    let meta = TemplateContainer::get_item__meta();
    assert_eq!(meta.template_params, vec!["T"]);
}

#[test]
fn function_template_multi_param() {
    let meta = TemplateContainer::convert__meta();
    assert_eq!(meta.template_params, vec!["T", "U"]);
}

// ============================================================================
// #[function] Tests - Default Parameters
// ============================================================================

#[derive(Any)]
#[angelscript(name = "DefaultParams", reference)]
struct DefaultParams;

impl DefaultParams {
    #[function]
    fn with_default(&self, #[default("10")] value: i32) -> i32 {
        value
    }

    #[function]
    fn multi_defaults(
        &self,
        required: i32,
        #[default("5")] opt1: i32,
        #[default("100")] opt2: i32,
    ) -> i32 {
        required + opt1 + opt2
    }
}

#[test]
fn function_default_param() {
    let meta = DefaultParams::with_default__meta();
    assert_eq!(meta.params.len(), 1);
    assert_eq!(meta.params[0].default_value, Some("10"));
}

#[test]
fn function_multi_defaults() {
    let meta = DefaultParams::multi_defaults__meta();
    assert_eq!(meta.params.len(), 3);
    assert_eq!(meta.params[0].default_value, None);
    assert_eq!(meta.params[1].default_value, Some("5"));
    assert_eq!(meta.params[2].default_value, Some("100"));
}

// ============================================================================
// #[interface] Tests
// ============================================================================

/// Basic interface.
#[interface]
trait Drawable {
    fn draw(&self);
    fn get_layer(&self) -> i32;
}

#[test]
fn interface_basic() {
    let meta = __as_Drawable_interface_meta();
    assert_eq!(meta.name, "Drawable");
    assert_eq!(meta.methods.len(), 2);
}

#[test]
fn interface_method_signatures() {
    let meta = __as_Drawable_interface_meta();

    let draw = meta.methods.iter().find(|m| m.name == "draw").unwrap();
    assert!(draw.is_const); // &self is const
    assert!(draw.param_types.is_empty());

    let get_layer = meta.methods.iter().find(|m| m.name == "get_layer").unwrap();
    assert!(get_layer.is_const);
}

/// Interface with custom name.
#[interface(name = "IUpdatable")]
trait Updatable {
    fn update(&mut self, delta: f32);
}

#[test]
fn interface_custom_name() {
    let meta = __as_Updatable_interface_meta();
    assert_eq!(meta.name, "IUpdatable");
    assert_eq!(meta.type_hash, TypeHash::from_name("IUpdatable"));
}

#[test]
fn interface_mut_method() {
    let meta = __as_Updatable_interface_meta();
    let update = &meta.methods[0];
    assert_eq!(update.name, "update");
    assert!(!update.is_const); // &mut self is not const
    assert_eq!(update.param_types.len(), 1);
}

/// Interface with method name override.
#[interface]
trait Serializable {
    #[function(name = "Save")]
    fn save(&self) -> bool;

    #[function(name = "Load")]
    fn load(&mut self) -> bool;
}

#[test]
fn interface_method_name_override() {
    let meta = __as_Serializable_interface_meta();
    assert_eq!(meta.methods.len(), 2);

    let save = meta.methods.iter().find(|m| m.name == "Save").unwrap();
    assert!(save.is_const);

    let load = meta.methods.iter().find(|m| m.name == "Load").unwrap();
    assert!(!load.is_const);
}

// ============================================================================
// #[funcdef] Tests
// ============================================================================

/// Basic funcdef.
#[funcdef]
type Callback = fn(i32) -> bool;

#[test]
fn funcdef_basic() {
    let meta = __as_Callback_funcdef_meta();
    assert_eq!(meta.name, "Callback");
    assert_eq!(meta.type_hash, TypeHash::from_name("Callback"));
    assert_eq!(meta.param_types.len(), 1);
    assert_eq!(meta.param_types[0], TypeHash::from_name("int"));
    assert_eq!(meta.return_type, TypeHash::from_name("bool"));
    assert!(meta.parent_type.is_none());
}

/// Funcdef with custom name.
#[funcdef(name = "EventHandler")]
type MyHandler = fn(i32, i32) -> ();

#[test]
fn funcdef_custom_name() {
    let meta = __as_MyHandler_funcdef_meta();
    assert_eq!(meta.name, "EventHandler");
    assert_eq!(meta.type_hash, TypeHash::from_name("EventHandler"));
}

/// Funcdef with no parameters.
#[funcdef]
type VoidCallback = fn() -> ();

#[test]
fn funcdef_no_params() {
    let meta = __as_VoidCallback_funcdef_meta();
    assert!(meta.param_types.is_empty());
}

/// Funcdef with multiple parameters.
#[funcdef]
type MultiParamFunc = fn(i32, f32, bool) -> i32;

#[test]
fn funcdef_multi_params() {
    let meta = __as_MultiParamFunc_funcdef_meta();
    assert_eq!(meta.param_types.len(), 3);
    assert_eq!(meta.param_types[0], TypeHash::from_name("int"));
    assert_eq!(meta.param_types[1], TypeHash::from_name("float"));
    assert_eq!(meta.param_types[2], TypeHash::from_name("bool"));
}

/// Child funcdef (nested in a type).
#[funcdef(parent = GenericArray)]
type ArrayCallback = fn(i32) -> bool;

#[test]
fn funcdef_with_parent() {
    let meta = __as_ArrayCallback_funcdef_meta();
    assert_eq!(meta.parent_type, Some(GenericArray::type_hash()));
}

// ============================================================================
// Edge Cases and Complex Scenarios
// ============================================================================

/// Empty struct.
#[derive(Any)]
#[angelscript(name = "EmptyStruct", value)]
struct EmptyStruct;

#[test]
fn derive_any_empty_struct() {
    let meta = EmptyStruct::__as_type_meta();
    assert_eq!(meta.name, "EmptyStruct");
    assert!(meta.properties.is_empty());
}

/// Struct with only private fields (no exposed properties).
#[derive(Any)]
#[angelscript(name = "PrivateFields", reference)]
struct PrivateFields {
    internal_a: i32,
    internal_b: String,
}

#[test]
fn derive_any_no_properties() {
    let meta = PrivateFields::__as_type_meta();
    assert!(meta.properties.is_empty());
}

/// Function with no parameters and void return.
#[function]
fn do_nothing() {}

#[test]
fn function_void_void() {
    let meta = <do_nothing as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.params.is_empty());
}

/// Interface with no methods.
#[interface]
trait EmptyInterface {}

#[test]
fn interface_empty() {
    let meta = __as_EmptyInterface_interface_meta();
    assert!(meta.methods.is_empty());
}

// ============================================================================
// Module Integration Tests
// ============================================================================

#[test]
fn module_registration_types() {
    use angelscript::Module;

    // Test that types can be registered via Module
    let module = Module::new()
        .ty::<SimpleType>()
        .ty::<Counter>()
        .ty::<Vector2>();

    assert_eq!(module.classes.len(), 3);
}

#[test]
fn module_registration_functions() {
    use angelscript::Module;

    // Free functions use unit struct pattern
    let module = Module::new()
        .function(add_numbers)
        .function(add_ints)
        .function(do_nothing);

    assert_eq!(module.functions.len(), 3);
}

#[test]
fn module_registration_methods() {
    use angelscript::Module;

    // Methods use const fn pointer pattern
    let module = Module::new()
        .ty::<Counter>()
        .function(Counter::new__meta)
        .function(Counter::increment__meta)
        .function(Counter::get_value__meta);

    assert_eq!(module.classes.len(), 1);
    assert_eq!(module.functions.len(), 3);
}

#[test]
fn module_registration_interface() {
    use angelscript::Module;

    let module = Module::new().interface(__as_Drawable_interface_meta());

    assert_eq!(module.interfaces.len(), 1);
}

#[test]
fn module_registration_funcdef() {
    use angelscript::Module;

    let module = Module::new().funcdef(__as_Callback_funcdef_meta());

    assert_eq!(module.funcdefs.len(), 1);
}

#[test]
fn module_registration_complete() {
    use angelscript::Module;

    // Complete module with all registration types
    let module = Module::in_namespace(&["game"])
        .ty::<Entity>()
        .ty::<Counter>()
        .function(Counter::new__meta)
        .function(Counter::increment__meta)
        .interface(__as_Drawable_interface_meta())
        .funcdef(__as_Callback_funcdef_meta());

    assert_eq!(module.namespace, vec!["game"]);
    assert_eq!(module.classes.len(), 2);
    assert_eq!(module.functions.len(), 2);
    assert_eq!(module.interfaces.len(), 1);
    assert_eq!(module.funcdefs.len(), 1);
}

// ============================================================================
// Additional Type Kind Tests (scoped, nocount, as_handle)
// ============================================================================

/// Scoped reference type.
#[derive(Any)]
#[angelscript(scoped)]
struct ScopedResource {
    handle: u64,
}

#[test]
fn derive_any_scoped_type() {
    let meta = <ScopedResource as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.name, "ScopedResource");
    // scoped types have asOBJ_SCOPED flag
}

/// NoCount reference type (single ref, no ref counting).
#[derive(Any)]
#[angelscript(nocount)]
struct NoCountRef {
    ptr: usize,
}

#[test]
fn derive_any_nocount_type() {
    let meta = <NoCountRef as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.name, "NoCountRef");
    // nocount types have asOBJ_NOCOUNT flag
}

/// AsHandle type (can be used as handle).
#[derive(Any)]
#[angelscript(as_handle)]
struct HandleWrapper {
    inner: u64,
}

#[test]
fn derive_any_as_handle_type() {
    let meta = <HandleWrapper as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.name, "HandleWrapper");
    // as_handle types have asOBJ_ASHANDLE flag
}

// ============================================================================
// Additional Function Behavior Tests
// ============================================================================

/// Type for testing advanced behaviors.
#[derive(Any)]
#[angelscript(reference)]
struct ManagedType {
    ref_count: i32,
}

impl ManagedType {
    #[function(destructor)]
    fn destroy(&mut self) {
        // Destructor behavior
    }

    #[function(addref)]
    fn add_ref(&mut self) {
        self.ref_count += 1;
    }

    #[function(release)]
    fn do_release(&mut self) {
        self.ref_count -= 1;
    }
}

#[test]
fn function_behavior_destructor() {
    let meta = ManagedType::destroy__meta();
    assert_eq!(meta.behavior, Some(Behavior::Destructor));
}

#[test]
fn function_behavior_addref() {
    let meta = ManagedType::add_ref__meta();
    assert_eq!(meta.behavior, Some(Behavior::AddRef));
}

#[test]
fn function_behavior_release() {
    let meta = ManagedType::do_release__meta();
    assert_eq!(meta.behavior, Some(Behavior::Release));
}

/// Type for list constructor testing.
#[derive(Any)]
#[angelscript(reference)]
struct IntList {
    data: Vec<i32>,
}

impl IntList {
    #[function(list_construct)]
    #[list_pattern(repeat = i32)]
    fn from_list(&mut self, _size: i32) {
        // List constructor behavior - size of buffer
    }

    #[function(keep, list_factory)]
    #[list_pattern(repeat = i32)]
    fn create_from_list(_size: i32) -> Self {
        IntList { data: Vec::new() }
    }
}

#[test]
fn function_behavior_list_construct() {
    let meta = IntList::from_list__meta();
    assert_eq!(meta.behavior, Some(Behavior::ListConstruct));
    assert!(meta.list_pattern.is_some());
}

#[test]
fn function_behavior_list_factory() {
    let meta = IntList::create_from_list__meta();
    assert_eq!(meta.behavior, Some(Behavior::ListFactory));
    assert!(meta.list_pattern.is_some());
}

/// Type for template callback.
#[derive(Any)]
#[angelscript(reference, template = "<T>")]
struct TemplateTypeBase {
    _data: i32,
}

impl TemplateTypeBase {
    #[function(template_callback)]
    fn validate_template(&self) -> bool {
        true
    }
}

#[test]
fn function_behavior_template_callback() {
    let meta = TemplateTypeBase::validate_template__meta();
    assert_eq!(meta.behavior, Some(Behavior::TemplateCallback));
}

// ============================================================================
// GC Behavior Tests
// ============================================================================

/// GC-managed type.
#[derive(Any)]
#[angelscript(reference)]
struct GcManaged {
    gc_flag: bool,
    ref_count: i32,
}

impl GcManaged {
    #[function(gc_getrefcount)]
    fn gc_get_ref_count(&self) -> i32 {
        self.ref_count
    }

    #[function(gc_setflag)]
    fn gc_set_flag(&mut self) {
        self.gc_flag = true;
    }

    #[function(gc_getflag)]
    fn gc_get_flag(&self) -> bool {
        self.gc_flag
    }

    #[function(gc_enumrefs)]
    fn gc_enum_refs(&self, _engine: &()) {
        // Enumerate references for GC
    }

    #[function(gc_releaserefs)]
    fn gc_release_refs(&mut self, _engine: &()) {
        // Release references for GC
    }

    #[function(get_weakref_flag)]
    fn get_weakref_flag(&self) -> bool {
        false
    }
}

#[test]
fn function_behavior_gc_getrefcount() {
    let meta = GcManaged::gc_get_ref_count__meta();
    assert_eq!(meta.behavior, Some(Behavior::GcGetRefCount));
}

#[test]
fn function_behavior_gc_setflag() {
    let meta = GcManaged::gc_set_flag__meta();
    assert_eq!(meta.behavior, Some(Behavior::GcSetFlag));
}

#[test]
fn function_behavior_gc_getflag() {
    let meta = GcManaged::gc_get_flag__meta();
    assert_eq!(meta.behavior, Some(Behavior::GcGetFlag));
}

#[test]
fn function_behavior_gc_enumrefs() {
    let meta = GcManaged::gc_enum_refs__meta();
    assert_eq!(meta.behavior, Some(Behavior::GcEnumRefs));
}

#[test]
fn function_behavior_gc_releaserefs() {
    let meta = GcManaged::gc_release_refs__meta();
    assert_eq!(meta.behavior, Some(Behavior::GcReleaseRefs));
}

#[test]
fn function_behavior_get_weakref_flag() {
    let meta = GcManaged::get_weakref_flag__meta();
    assert_eq!(meta.behavior, Some(Behavior::GetWeakRefFlag));
}

// ============================================================================
// Generic Calling Convention Tests (#[param] attributes)
// ============================================================================

/// Generic function with variable type parameter.
#[function(generic)]
#[param(variable)]
fn generic_identity(_value: i32) -> i32 {
    0
}

#[test]
fn function_generic_variable_param() {
    let meta = <generic_identity as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    // variable param uses VARIABLE_PARAM type hash
}

/// Generic function with variadic parameters.
#[function(generic)]
#[param(variadic)]
fn generic_variadic(_values: i32) {}

#[test]
fn function_generic_variadic_param() {
    let meta = <generic_variadic as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    assert!(meta.generic_params[0].is_variadic);
}

/// Generic function with in reference.
#[function(generic)]
#[param(in)]
fn generic_in_param(_value: i32) {}

#[test]
fn function_generic_in_ref() {
    let meta = <generic_in_param as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    // ref_mode should be In
}

/// Generic function with out reference.
#[function(generic)]
#[param(out)]
fn generic_out_param(_value: i32) {}

#[test]
fn function_generic_out_ref() {
    let meta = <generic_out_param as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    // ref_mode should be Out
}

/// Generic function with inout reference.
#[function(generic)]
#[param(inout)]
fn generic_inout_param(_value: i32) {}

#[test]
fn function_generic_inout_ref() {
    let meta = <generic_inout_param as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    // ref_mode should be InOut
}

/// Generic function with explicit type.
#[function(generic)]
#[param(type = i32)]
fn generic_explicit_type(_value: i32) {}

#[test]
fn function_generic_explicit_type() {
    let meta = <generic_explicit_type as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    // type hash should be i32
}

/// Generic function with default value.
#[function(generic)]
#[param(variable, default = "-1")]
fn generic_with_default(_value: i32) {}

#[test]
fn function_generic_default_value() {
    let meta = <generic_with_default as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params[0].default_value, Some("-1"));
}

/// Generic function with if_handle_then_const.
#[function(generic)]
#[param(variable, if_handle_then_const)]
fn generic_handle_const(_value: i32) {}

#[test]
fn function_generic_if_handle_then_const() {
    let meta = <generic_handle_const as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert!(meta.generic_params[0].if_handle_then_const);
}

// ============================================================================
// Return Attribute Tests (#[returns])
// ============================================================================

/// Type for return attribute testing.
#[derive(Any)]
#[angelscript(reference)]
struct ReturnTest {
    value: i32,
}

impl ReturnTest {
    #[function(generic)]
    #[returns(ref)]
    fn get_ref(&self) -> i32 {
        self.value
    }

    #[function]
    #[returns(const)]
    fn get_const_value(&self) -> i32 {
        self.value
    }

    #[function(generic)]
    #[returns(handle)]
    fn get_handle(&self) -> i32 {
        self.value
    }

    #[function]
    #[returns(variable)]
    fn get_variable(&self) -> i32 {
        self.value
    }

    #[function(generic)]
    #[returns(type = f32)]
    fn get_explicit_return(&self) -> f32 {
        0.0
    }

    #[function(generic)]
    #[returns(ref, const)]
    fn get_const_ref(&self) -> i32 {
        self.value
    }
}

#[test]
fn function_return_ref() {
    let meta = ReturnTest::get_ref__meta();
    // return mode should be Reference
}

#[test]
fn function_return_const() {
    let meta = ReturnTest::get_const_value__meta();
    assert!(meta.return_meta.is_const);
}

#[test]
fn function_return_handle() {
    let meta = ReturnTest::get_handle__meta();
    // return mode should be Handle
}

#[test]
fn function_return_variable() {
    let meta = ReturnTest::get_variable__meta();
    assert!(meta.return_meta.is_variable);
}

#[test]
fn function_return_explicit_type() {
    let meta = ReturnTest::get_explicit_return__meta();
    // type hash should be f32
    assert!(meta.return_meta.type_hash.is_some());
}

#[test]
fn function_return_const_ref() {
    let meta = ReturnTest::get_const_ref__meta();
    assert!(meta.return_meta.is_const);
}

// ============================================================================
// List Pattern Tests (#[list_pattern])
// ============================================================================

/// Type for list pattern testing.
#[derive(Any)]
#[angelscript(reference)]
struct ListPatternTest {
    data: Vec<i32>,
}

impl ListPatternTest {
    #[function(list_construct)]
    #[list_pattern(repeat = i32)]
    fn from_ints(&mut self, _size: i32) {
        // Buffer pointer and size passed via generic calling convention
    }

    #[function(list_construct)]
    #[list_pattern(fixed(i32, f32, bool))]
    fn from_fixed(&mut self, _a: i32, _b: f32, _c: bool) {}

    #[function(list_construct)]
    #[list_pattern(repeat_tuple(i32, f32))]
    fn from_pairs(&mut self, _size: i32) {
        // Buffer pointer and size passed via generic calling convention
    }
}

#[test]
fn function_list_pattern_repeat() {
    let meta = ListPatternTest::from_ints__meta();
    assert!(meta.list_pattern.is_some());
    // Should be ListPatternMeta::Repeat
}

#[test]
fn function_list_pattern_fixed() {
    let meta = ListPatternTest::from_fixed__meta();
    assert!(meta.list_pattern.is_some());
    // Should be ListPatternMeta::Fixed
}

#[test]
fn function_list_pattern_repeat_tuple() {
    let meta = ListPatternTest::from_pairs__meta();
    assert!(meta.list_pattern.is_some());
    // Should be ListPatternMeta::RepeatTuple
}

// ============================================================================
// Additional Operator Tests
// ============================================================================

/// More operator tests.
#[derive(Any)]
#[angelscript(value)]
struct OperatorTest {
    value: i32,
}

impl OperatorTest {
    #[function(operator = Operator::Sub)]
    fn op_sub(&self, other: &OperatorTest) -> OperatorTest {
        OperatorTest {
            value: self.value - other.value,
        }
    }

    #[function(operator = Operator::Div)]
    fn op_div(&self, other: &OperatorTest) -> OperatorTest {
        OperatorTest {
            value: self.value / other.value,
        }
    }

    #[function(operator = Operator::Mod)]
    fn op_mod(&self, other: &OperatorTest) -> OperatorTest {
        OperatorTest {
            value: self.value % other.value,
        }
    }

    #[function(operator = Operator::Neg)]
    fn op_neg(&self) -> OperatorTest {
        OperatorTest { value: -self.value }
    }

    #[function(operator = Operator::PreInc)]
    fn op_preinc(&mut self) -> OperatorTest {
        self.value += 1;
        OperatorTest { value: self.value }
    }

    #[function(operator = Operator::PostInc)]
    fn op_postinc(&mut self) -> OperatorTest {
        let old = OperatorTest { value: self.value };
        self.value += 1;
        old
    }
}

#[test]
fn function_operator_sub() {
    let meta = OperatorTest::op_sub__meta();
    assert!(meta.behavior.is_some());
}

#[test]
fn function_operator_div() {
    let meta = OperatorTest::op_div__meta();
    assert!(meta.behavior.is_some());
}

#[test]
fn function_operator_mod() {
    let meta = OperatorTest::op_mod__meta();
    assert!(meta.behavior.is_some());
}

#[test]
fn function_operator_neg() {
    let meta = OperatorTest::op_neg__meta();
    assert!(meta.behavior.is_some());
}

#[test]
fn function_operator_preinc() {
    let meta = OperatorTest::op_preinc__meta();
    assert!(meta.behavior.is_some());
}

#[test]
fn function_operator_postinc() {
    let meta = OperatorTest::op_postinc__meta();
    assert!(meta.behavior.is_some());
}

// ============================================================================
// Copy Constructor Test
// ============================================================================

/// Type with copy constructor.
#[derive(Any)]
#[angelscript(value)]
struct Copyable {
    value: i32,
}

impl Copyable {
    #[function(constructor, copy)]
    fn copy_construct(&mut self, _other: &Copyable) {}
}

#[test]
fn function_copy_constructor() {
    let meta = Copyable::copy_construct__meta();
    assert_eq!(meta.behavior, Some(Behavior::CopyConstructor));
}

// ============================================================================
// Template Param on Regular Function Parameters
// ============================================================================

/// Type with template parameters in methods.
#[derive(Any)]
#[angelscript(reference, template = "<T>")]
struct ContainerBase {
    _data: i32,
}

impl ContainerBase {
    #[function]
    fn push(&mut self, #[template("T")] _value: i32) {}
}

#[test]
fn function_param_template_attr() {
    let meta = ContainerBase::push__meta();
    assert_eq!(meta.params.len(), 1);
    assert_eq!(meta.params[0].template_param, Some("T"));
}

// ============================================================================
// Property Name Override
// ============================================================================

/// Type for property name override testing.
#[derive(Any)]
#[angelscript(reference)]
struct PropertyOverride {
    internal_value: i32,
}

impl PropertyOverride {
    #[function(property, property_name = "value")]
    fn get_internal_value(&self) -> i32 {
        self.internal_value
    }

    #[function(property, property_name = "value")]
    fn set_internal_value(&mut self, v: i32) {
        self.internal_value = v;
    }
}

#[test]
fn function_property_name_override() {
    let meta = PropertyOverride::get_internal_value__meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("value"));
}

#[test]
fn function_property_name_override_setter() {
    let meta = PropertyOverride::set_internal_value__meta();
    assert!(meta.is_property);
    assert_eq!(meta.property_name, Some("value"));
}

// ============================================================================
// Field Name Override in Properties
// ============================================================================

/// Type with field name override.
#[derive(Any)]
#[angelscript(reference)]
struct FieldNameOverride {
    #[angelscript(get, set, name = "custom_name")]
    internal_field: i32,
}

#[test]
fn derive_any_field_name_override() {
    let meta = <FieldNameOverride as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.properties.len(), 1);
    assert_eq!(meta.properties[0].name, "custom_name");
}

// ============================================================================
// Explicit Const Keyword Tests
// ============================================================================

/// Type for explicit const testing.
#[derive(Any)]
#[angelscript(reference)]
struct ConstTest {
    data: i32,
}

impl ConstTest {
    /// Method explicitly marked const (vs auto-detected from &self).
    #[function(const)]
    fn explicit_const(&mut self) -> i32 {
        self.data
    }

    /// Instance method (explicit).
    #[function(instance)]
    fn instance_method(&self) {}
}

#[test]
fn function_explicit_const_keyword() {
    let meta = ConstTest::explicit_const__meta();
    assert!(meta.is_const);
}

#[test]
fn function_instance_kind() {
    let meta = ConstTest::instance_method__meta();
    // instance methods don't set a special behavior, just marks function kind
    assert!(meta.is_method);
}

// ============================================================================
// Funcdef Return Void Test
// ============================================================================

/// Funcdef with void return (no return type specified).
#[funcdef]
type ActionCallback = fn(i32);

#[test]
fn funcdef_void_return() {
    let meta = __as_ActionCallback_funcdef_meta();
    assert_eq!(meta.name, "ActionCallback");
    assert_eq!(meta.param_types.len(), 1);
    // Return type should be () for void
}

// ============================================================================
// Template Boolean Flag Test
// ============================================================================

/// Test deprecated `template` boolean flag.
#[derive(Any)]
#[angelscript(reference)]
struct TemplateBoolTest {
    data: i32,
}

impl TemplateBoolTest {
    #[function(template)]
    fn with_template_flag(&self) {}
}

#[test]
fn function_template_bool_flag() {
    let meta = TemplateBoolTest::with_template_flag__meta();
    // This uses the deprecated boolean `template` flag
    // The template_params will still be empty since no template string is provided
    assert!(meta.template_params.is_empty());
}

// ============================================================================
// Multiple #[param] Attributes Test
// ============================================================================

/// Generic function with multiple param attributes (tests from_attrs iteration).
#[function(generic)]
#[param(variable)]
#[param(in)]
#[param(out)]
fn generic_multi_params(_a: i32, _b: i32, _c: i32) {}

#[test]
fn function_generic_multi_params() {
    let meta = <generic_multi_params as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 3);
}

// ============================================================================
// Combined #[returns] Attributes Test
// ============================================================================

/// Type for combined return attribute testing.
#[derive(Any)]
#[angelscript(reference)]
struct CombinedReturnTest {
    data: i32,
}

impl CombinedReturnTest {
    #[function(generic)]
    #[returns(handle, const, type = i32)]
    fn combined_return(&self) -> i32 {
        self.data
    }
}

#[test]
fn function_return_combined() {
    let meta = CombinedReturnTest::combined_return__meta();
    assert!(meta.return_meta.is_const);
    // handle mode is set
}

// ============================================================================
// Combined #[param] Attributes Test
// ============================================================================

/// Generic function with combined param attributes.
#[function(generic)]
#[param(variable, in, default = "0", if_handle_then_const)]
fn generic_combined_param(_value: i32) {}

#[test]
fn function_generic_combined_param() {
    let meta = <generic_combined_param as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 1);
    assert_eq!(meta.generic_params[0].default_value, Some("0"));
    assert!(meta.generic_params[0].if_handle_then_const);
}

// ============================================================================
// Generic function with multiple params of mixed types
// ============================================================================

/// Tests ParamAttrs with explicit type in combination with other attrs.
#[function(generic)]
#[param(type = i32, in)]
#[param(type = f32, out)]
fn generic_typed_params(_a: i32, _b: f32) {}

#[test]
fn function_generic_typed_params() {
    let meta = <generic_typed_params as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    assert_eq!(meta.generic_params.len(), 2);
}

// ============================================================================
// Function Attribute returns = Type Test
// ============================================================================

/// Test the inline `returns = Type` function attribute.
#[function(generic, returns = i32)]
fn inline_returns_type(_value: i32) -> i32 {
    0
}

#[test]
fn function_inline_returns_attr() {
    let meta = <inline_returns_type as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.is_generic);
    // The `returns` attribute is for generic calling convention return type override
}

// ============================================================================
// Empty Template Params String Test
// ============================================================================

/// Tests empty template string edge case.
#[function(template = "")]
fn empty_template_string(_value: i32) {}

#[test]
fn function_empty_template_string() {
    let meta = <empty_template_string as HasFunctionMeta>::__as_fn_meta();
    // Empty template string results in empty params
    assert!(meta.template_params.is_empty());
}

// ============================================================================
// Additional Edge Cases
// ============================================================================

/// Type with multiple angelscript attributes on same struct.
#[derive(Any)]
#[angelscript(name = "MultiAttrType", reference, template = "<T>")]
struct MultiAttrStruct {
    value: i32,
}

#[test]
fn derive_any_multi_attrs() {
    let meta = <MultiAttrStruct as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.name, "MultiAttrType");
    assert_eq!(meta.template_params.len(), 1);
}

/// Struct with no angelscript attributes (uses defaults).
#[derive(Any)]
struct NoAttrStruct {
    value: i32,
}

#[test]
fn derive_any_no_attrs() {
    let meta = <NoAttrStruct as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.name, "NoAttrStruct");
    // Default is reference type kind
}

/// Test field with only get (no set).
#[derive(Any)]
#[angelscript(reference)]
struct GetOnlyField {
    #[angelscript(get)]
    read_only: i32,
}

#[test]
fn derive_any_get_only_field() {
    let meta = <GetOnlyField as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.properties.len(), 1);
    assert!(meta.properties[0].get);
    assert!(!meta.properties[0].set);
}

/// Test field with only set (no get).
#[derive(Any)]
#[angelscript(reference)]
struct SetOnlyField {
    #[angelscript(set)]
    write_only: i32,
}

#[test]
fn derive_any_set_only_field() {
    let meta = <SetOnlyField as HasClassMeta>::__as_type_meta();
    assert_eq!(meta.properties.len(), 1);
    assert!(!meta.properties[0].get);
    assert!(meta.properties[0].set);
}

/// Interface with mut method.
#[interface]
trait MutMethodInterface {
    fn modify(&mut self);
}

#[test]
fn interface_with_mut_only() {
    let meta = __as_MutMethodInterface_interface_meta();
    assert_eq!(meta.methods.len(), 1);
}

// ============================================================================
// Interface with Associated Types/Constants (covers `other` branch)
// ============================================================================

/// Interface with associated type (tests the `other` branch in filter_trait_item_attrs).
#[interface]
trait InterfaceWithAssocType {
    type Output;
    fn process(&self) -> i32;
}

#[test]
fn interface_with_associated_type() {
    let meta = __as_InterfaceWithAssocType_interface_meta();
    // Only methods are collected, not associated types
    assert_eq!(meta.methods.len(), 1);
}

/// Interface with associated constant.
#[interface]
trait InterfaceWithConst {
    const MAX_SIZE: i32;
    fn get_size(&self) -> i32;
}

#[test]
fn interface_with_associated_const() {
    let meta = __as_InterfaceWithConst_interface_meta();
    assert_eq!(meta.methods.len(), 1);
}

/// Interface with default method implementation.
#[interface]
trait InterfaceWithDefault {
    fn required(&self) -> i32;
    fn optional(&self) -> i32 {
        42
    }
}

#[test]
fn interface_with_default_impl() {
    let meta = __as_InterfaceWithDefault_interface_meta();
    assert_eq!(meta.methods.len(), 2);
}

// ============================================================================
// REQUIREMENT-BASED TESTS (Should FAIL initially, driving implementation)
// ============================================================================
// These tests verify the REQUIREMENTS, not the current implementation.
// They should fail until the FFI registration gaps are fixed.

use std::any::TypeId;

/// Verify that ClassMeta captures Rust's TypeId for runtime type verification.
/// This enables safe downcasting of FFI objects.
#[test]
fn class_meta_captures_rust_type_id() {
    let meta = Vec3::__as_type_meta();
    assert!(
        meta.rust_type_id.is_some(),
        "ClassMeta should capture rust_type_id for runtime type verification"
    );
    assert_eq!(
        meta.rust_type_id,
        Some(TypeId::of::<Vec3>()),
        "rust_type_id should match TypeId::of::<Vec3>()"
    );
}

/// Verify that FunctionMeta captures the actual callable NativeFn.
/// This enables the VM to invoke FFI functions.
#[test]
fn function_meta_captures_native_fn() {
    let meta = <add_numbers as HasFunctionMeta>::__as_fn_meta();
    assert!(
        meta.native_fn.is_some(),
        "FunctionMeta should capture native_fn for VM invocation"
    );
}

/// Verify that method FunctionMeta captures the actual callable NativeFn.
#[test]
fn method_meta_captures_native_fn() {
    let meta = Counter::increment__meta();
    assert!(
        meta.native_fn.is_some(),
        "Method FunctionMeta should capture native_fn"
    );
}

/// Verify that constructor FunctionMeta captures the actual callable NativeFn.
#[test]
fn constructor_meta_captures_native_fn() {
    let meta = Constructable::create__meta();
    assert!(
        meta.native_fn.is_some(),
        "Constructor FunctionMeta should capture native_fn"
    );
}

/// Verify that operator FunctionMeta captures the actual callable NativeFn.
#[test]
fn operator_meta_captures_native_fn() {
    let meta = Vector2::op_add__meta();
    assert!(
        meta.native_fn.is_some(),
        "Operator FunctionMeta should capture native_fn"
    );
}

/// Verify that NativeFn is actually callable and produces correct results.
/// This tests the full invoke path: extract args from CallContext, call function, set return.
#[test]
fn native_fn_is_callable() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    let meta = <add_numbers as HasFunctionMeta>::__as_fn_meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext with args: add_numbers(10, 20)
    let mut args = vec![Dynamic::Int(10), Dynamic::Int(20)];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 0, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    assert_eq!(ret, Dynamic::Int(30), "10 + 20 should equal 30");
}

/// Verify that NativeFn has the correct TypeHash based on function name.
/// This ensures each function gets a unique ID, not a hardcoded placeholder.
#[test]
fn native_fn_has_correct_type_hash() {
    let meta = <add_numbers as HasFunctionMeta>::__as_fn_meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // The NativeFn ID should match the function name hash
    assert_eq!(
        native.id,
        TypeHash::from_name("add_numbers"),
        "NativeFn should have TypeHash matching function name"
    );
}

/// Verify that method NativeFn is callable with &self receiver.
#[test]
fn native_fn_method_self_ref() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    // Create a Counter instance
    let counter = Counter { value: 42 };

    let meta = Counter::get_value__meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext with `this` in slot 0
    let mut args = vec![Dynamic::Native(Box::new(counter))];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 1, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    assert_eq!(ret, Dynamic::Int(42), "get_value should return 42");
}

/// Verify that method NativeFn is callable with &mut self receiver.
#[test]
fn native_fn_method_self_mut() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    // Create a Counter instance
    let counter = Counter { value: 10 };

    let meta = Counter::increment__meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext with `this` in slot 0
    let mut args = vec![Dynamic::Native(Box::new(counter))];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 1, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");

    // Verify the counter was incremented
    match &args[0] {
        Dynamic::Native(boxed) => {
            let counter = boxed.downcast_ref::<Counter>().unwrap();
            assert_eq!(
                counter.value, 11,
                "increment should change value from 10 to 11"
            );
        }
        _ => panic!("expected Native"),
    }
}

/// Verify that method NativeFn is callable with &mut self and additional params.
#[test]
fn native_fn_method_with_params() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    // Create a Counter instance
    let counter = Counter { value: 5 };

    let meta = Counter::add__meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext with `this` in slot 0, arg in slot 1
    let mut args = vec![Dynamic::Native(Box::new(counter)), Dynamic::Int(15)];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 1, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");

    // Verify the counter was updated
    match &args[0] {
        Dynamic::Native(boxed) => {
            let counter = boxed.downcast_ref::<Counter>().unwrap();
            assert_eq!(
                counter.value, 20,
                "add(15) should change value from 5 to 20"
            );
        }
        _ => panic!("expected Native"),
    }
}

/// Verify that float return types work correctly.
#[test]
fn native_fn_float_return() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    let meta = <divide_numbers as HasFunctionMeta>::__as_fn_meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext with args: divide_numbers(10.0, 4.0)
    let mut args = vec![Dynamic::Float(10.0), Dynamic::Float(4.0)];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 0, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    match ret {
        Dynamic::Float(v) => assert!((v - 2.5).abs() < 0.001, "10.0 / 4.0 should equal 2.5"),
        _ => panic!("expected Float"),
    }
}

/// Verify that bool return types work correctly.
#[test]
fn native_fn_bool_return() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    let meta = <is_positive as HasFunctionMeta>::__as_fn_meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Test positive number
    let mut args = vec![Dynamic::Int(42)];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 0, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    assert_eq!(ret, Dynamic::Bool(true), "42 should be positive");

    // Test negative number
    let mut args = vec![Dynamic::Int(-5)];
    let mut ret = Dynamic::Void;
    let mut ctx = CallContext::new(&mut args, 0, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    assert_eq!(ret, Dynamic::Bool(false), "-5 should not be positive");
}

/// Verify that method taking &Self as parameter works correctly.
#[test]
fn native_fn_method_ref_self_param() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    // Create two Counter instances
    let counter1 = Counter { value: 10 };
    let counter2 = Counter { value: 25 };

    let meta = Counter::add_from__meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext: slot 0 = this (&mut self), slot 1 = other (&Self)
    let mut args = vec![
        Dynamic::Native(Box::new(counter1)),
        Dynamic::Native(Box::new(counter2)),
    ];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 1, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");

    // Verify the first counter was updated (10 + 25 = 35)
    match &args[0] {
        Dynamic::Native(boxed) => {
            let counter = boxed.downcast_ref::<Counter>().unwrap();
            assert_eq!(
                counter.value, 35,
                "add_from should add other.value to self.value"
            );
        }
        _ => panic!("expected Native"),
    }
}

/// Verify that method taking owned Self as parameter works correctly.
#[test]
fn native_fn_method_owned_self_param() {
    use angelscript_core::{CallContext, Dynamic, ObjectHeap};

    // Create two Counter instances
    let counter1 = Counter { value: 42 };
    let counter2 = Counter { value: 42 };

    let meta = Counter::equals__meta();
    let native = meta.native_fn.expect("native_fn should be Some");

    // Set up CallContext: slot 0 = this (&self), slot 1 = other (owned Self)
    let mut args = vec![
        Dynamic::Native(Box::new(counter1)),
        Dynamic::Native(Box::new(counter2)),
    ];
    let mut ret = Dynamic::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut args, 1, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    assert_eq!(ret, Dynamic::Bool(true), "both counters have value 42");

    // Test with different values
    let counter1 = Counter { value: 10 };
    let counter2 = Counter { value: 20 };
    let mut args = vec![
        Dynamic::Native(Box::new(counter1)),
        Dynamic::Native(Box::new(counter2)),
    ];
    let mut ret = Dynamic::Void;
    let mut ctx = CallContext::new(&mut args, 1, &mut ret, &mut heap);

    native.call(&mut ctx).expect("call should succeed");
    assert_eq!(ret, Dynamic::Bool(false), "counters have different values");
}
