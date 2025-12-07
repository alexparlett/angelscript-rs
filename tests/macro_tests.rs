//! Integration tests for AngelScript proc macros.

use angelscript::{Any, TypeHash, TypeKind, ClassMeta, Behavior};

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
