//! Integration tests for AngelScript proc macros.

use angelscript::{Any, TypeHash, TypeKind, ClassMeta, Behavior, FunctionMeta, Operator, InterfaceMeta, FuncdefMeta};
use angelscript::{function, interface, funcdef};

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
