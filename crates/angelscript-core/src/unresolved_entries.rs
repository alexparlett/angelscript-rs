//! Unresolved entry types for Pass 1 output.
//!
//! These types represent declarations captured during the registration pass,
//! before type resolution. They are distinct from resolved entry types that
//! go into the registry.
//!
//! # Design Principles
//!
//! 1. **Separate types** - Unresolved entries are distinct types, not variants of resolved entries
//! 2. **All data captured** - Everything needed for completion is stored
//! 3. **Source spans preserved** - For error reporting during completion
//! 4. **No TypeHash** - No hashes computed until completion

use crate::{
    QualifiedName, Span, UnitId, UnresolvedParam, UnresolvedSignature, UnresolvedType, Visibility,
};

/// Unresolved class declaration from Pass 1.
///
/// Contains all information needed to create a resolved ClassEntry in Pass 2.
#[derive(Debug, Clone)]
pub struct UnresolvedClass {
    /// Qualified name (namespace + simple name).
    pub name: QualifiedName,

    /// Source location.
    pub span: Span,

    /// Unit this class was declared in.
    pub unit_id: UnitId,

    /// Inheritance list (base class, mixins, interfaces - not yet categorized).
    /// Classification happens during completion when we can look up each type.
    pub inheritance: Vec<UnresolvedInheritance>,

    /// Class modifiers.
    pub is_final: bool,
    pub is_abstract: bool,
    pub is_shared: bool,

    /// Methods declared in this class.
    pub methods: Vec<UnresolvedMethod>,

    /// Fields declared in this class.
    pub fields: Vec<UnresolvedField>,

    /// Virtual properties declared in this class.
    pub virtual_properties: Vec<UnresolvedVirtualProperty>,

    /// Nested funcdefs.
    pub funcdefs: Vec<UnresolvedFuncdef>,
}

impl UnresolvedClass {
    /// Create a new unresolved class.
    pub fn new(name: QualifiedName, span: Span, unit_id: UnitId) -> Self {
        Self {
            name,
            span,
            unit_id,
            inheritance: Vec::new(),
            is_final: false,
            is_abstract: false,
            is_shared: false,
            methods: Vec::new(),
            fields: Vec::new(),
            virtual_properties: Vec::new(),
            funcdefs: Vec::new(),
        }
    }

    /// Add inheritance item.
    pub fn with_inheritance(mut self, item: UnresolvedInheritance) -> Self {
        self.inheritance.push(item);
        self
    }

    /// Mark as final.
    pub fn with_final(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Mark as abstract.
    pub fn with_abstract(mut self) -> Self {
        self.is_abstract = true;
        self
    }

    /// Mark as shared.
    pub fn with_shared(mut self) -> Self {
        self.is_shared = true;
        self
    }

    /// Add a method.
    pub fn with_method(mut self, method: UnresolvedMethod) -> Self {
        self.methods.push(method);
        self
    }

    /// Add a field.
    pub fn with_field(mut self, field: UnresolvedField) -> Self {
        self.fields.push(field);
        self
    }

    /// Add a virtual property.
    pub fn with_virtual_property(mut self, prop: UnresolvedVirtualProperty) -> Self {
        self.virtual_properties.push(prop);
        self
    }

    /// Add a nested funcdef.
    pub fn with_funcdef(mut self, funcdef: UnresolvedFuncdef) -> Self {
        self.funcdefs.push(funcdef);
        self
    }
}

/// Unresolved inheritance reference.
///
/// We don't know if this is a base class, mixin, or interface until
/// we can look up the type during completion.
#[derive(Debug, Clone)]
pub struct UnresolvedInheritance {
    /// The type being inherited.
    pub type_ref: UnresolvedType,
    /// Source span for error reporting.
    pub span: Span,
}

impl UnresolvedInheritance {
    pub fn new(type_ref: UnresolvedType, span: Span) -> Self {
        Self { type_ref, span }
    }
}

/// Unresolved method declaration.
#[derive(Debug, Clone)]
pub struct UnresolvedMethod {
    /// Method name.
    pub name: String,
    /// Method signature.
    pub signature: UnresolvedSignature,
    /// Source span.
    pub span: Span,
    /// Visibility.
    pub visibility: Visibility,
    /// Method modifiers.
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
    pub is_abstract: bool,
    pub is_const: bool,
    /// Special method kind.
    pub kind: MethodKind,
    /// Whether the method has a body (false = declaration only or deleted).
    pub has_body: bool,
    /// Whether the method is deleted (= delete).
    pub is_deleted: bool,
}

/// Special method kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MethodKind {
    #[default]
    Regular,
    Constructor,
    CopyConstructor,
    Destructor,
    Factory,
}

impl UnresolvedMethod {
    pub fn new(name: impl Into<String>, signature: UnresolvedSignature, span: Span) -> Self {
        Self {
            name: name.into(),
            signature,
            span,
            visibility: Visibility::Public,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_abstract: false,
            is_const: false,
            kind: MethodKind::Regular,
            has_body: true,
            is_deleted: false,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_virtual(mut self) -> Self {
        self.is_virtual = true;
        self
    }

    pub fn with_override(mut self) -> Self {
        self.is_override = true;
        self
    }

    pub fn with_final(mut self) -> Self {
        self.is_final = true;
        self
    }

    pub fn with_abstract(mut self) -> Self {
        self.is_abstract = true;
        self.has_body = false;
        self
    }

    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    pub fn with_kind(mut self, kind: MethodKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_deleted(mut self) -> Self {
        self.is_deleted = true;
        self.has_body = false;
        self
    }

    pub fn is_constructor(&self) -> bool {
        matches!(
            self.kind,
            MethodKind::Constructor | MethodKind::CopyConstructor
        )
    }

    pub fn is_destructor(&self) -> bool {
        matches!(self.kind, MethodKind::Destructor)
    }
}

/// Unresolved field declaration.
#[derive(Debug, Clone)]
pub struct UnresolvedField {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: UnresolvedType,
    /// Source span.
    pub span: Span,
    /// Visibility.
    pub visibility: Visibility,
    /// Whether the field has an initializer.
    pub has_initializer: bool,
}

impl UnresolvedField {
    pub fn new(name: impl Into<String>, field_type: UnresolvedType, span: Span) -> Self {
        Self {
            name: name.into(),
            field_type,
            span,
            visibility: Visibility::Public,
            has_initializer: false,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_initializer(mut self) -> Self {
        self.has_initializer = true;
        self
    }
}

/// Unresolved virtual property declaration.
#[derive(Debug, Clone)]
pub struct UnresolvedVirtualProperty {
    /// Property name.
    pub name: String,
    /// Property type.
    pub property_type: UnresolvedType,
    /// Source span.
    pub span: Span,
    /// Visibility.
    pub visibility: Visibility,
    /// Getter info (if present).
    pub getter: Option<UnresolvedAccessor>,
    /// Setter info (if present).
    pub setter: Option<UnresolvedAccessor>,
}

impl UnresolvedVirtualProperty {
    pub fn new(name: impl Into<String>, property_type: UnresolvedType, span: Span) -> Self {
        Self {
            name: name.into(),
            property_type,
            span,
            visibility: Visibility::Public,
            getter: None,
            setter: None,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_getter(mut self, getter: UnresolvedAccessor) -> Self {
        self.getter = Some(getter);
        self
    }

    pub fn with_setter(mut self, setter: UnresolvedAccessor) -> Self {
        self.setter = Some(setter);
        self
    }
}

/// Unresolved property accessor.
#[derive(Debug, Clone)]
pub struct UnresolvedAccessor {
    /// Source span.
    pub span: Span,
    /// Whether the accessor is const.
    pub is_const: bool,
    /// Whether the accessor has a body.
    pub has_body: bool,
}

impl UnresolvedAccessor {
    pub fn new(span: Span) -> Self {
        Self {
            span,
            is_const: false,
            has_body: true,
        }
    }

    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    pub fn without_body(mut self) -> Self {
        self.has_body = false;
        self
    }
}

/// Unresolved mixin class declaration from Pass 1.
///
/// Mixins are similar to classes but cannot be instantiated directly.
/// Their members are copied into including classes.
#[derive(Debug, Clone)]
pub struct UnresolvedMixin {
    /// The class declaration (mixins use same structure).
    pub class: UnresolvedClass,
}

impl UnresolvedMixin {
    pub fn new(class: UnresolvedClass) -> Self {
        Self { class }
    }
}

/// Unresolved interface declaration from Pass 1.
#[derive(Debug, Clone)]
pub struct UnresolvedInterface {
    /// Qualified name.
    pub name: QualifiedName,
    /// Source span.
    pub span: Span,
    /// Unit this interface was declared in.
    pub unit_id: UnitId,
    /// Base interfaces.
    pub bases: Vec<UnresolvedInheritance>,
    /// Interface methods.
    pub methods: Vec<UnresolvedSignature>,
    /// Whether this is a shared interface.
    pub is_shared: bool,
}

impl UnresolvedInterface {
    pub fn new(name: QualifiedName, span: Span, unit_id: UnitId) -> Self {
        Self {
            name,
            span,
            unit_id,
            bases: Vec::new(),
            methods: Vec::new(),
            is_shared: false,
        }
    }

    pub fn with_base(mut self, base: UnresolvedInheritance) -> Self {
        self.bases.push(base);
        self
    }

    pub fn with_method(mut self, method: UnresolvedSignature) -> Self {
        self.methods.push(method);
        self
    }

    pub fn with_shared(mut self) -> Self {
        self.is_shared = true;
        self
    }
}

/// Unresolved funcdef (function type) declaration from Pass 1.
#[derive(Debug, Clone)]
pub struct UnresolvedFuncdef {
    /// Qualified name.
    pub name: QualifiedName,
    /// Source span.
    pub span: Span,
    /// Unit this funcdef was declared in.
    pub unit_id: UnitId,
    /// Parameter types.
    pub params: Vec<UnresolvedParam>,
    /// Return type.
    pub return_type: UnresolvedType,
    /// Whether this is a shared funcdef.
    pub is_shared: bool,
}

impl UnresolvedFuncdef {
    pub fn new(
        name: QualifiedName,
        span: Span,
        unit_id: UnitId,
        params: Vec<UnresolvedParam>,
        return_type: UnresolvedType,
    ) -> Self {
        Self {
            name,
            span,
            unit_id,
            params,
            return_type,
            is_shared: false,
        }
    }

    pub fn with_shared(mut self) -> Self {
        self.is_shared = true;
        self
    }
}

/// Unresolved enum declaration from Pass 1.
#[derive(Debug, Clone)]
pub struct UnresolvedEnum {
    /// Qualified name.
    pub name: QualifiedName,
    /// Source span.
    pub span: Span,
    /// Unit this enum was declared in.
    pub unit_id: UnitId,
    /// Enum values.
    pub values: Vec<UnresolvedEnumValue>,
    /// Whether this is a shared enum.
    pub is_shared: bool,
}

/// Unresolved enum value.
#[derive(Debug, Clone)]
pub struct UnresolvedEnumValue {
    /// Value name.
    pub name: String,
    /// Source span.
    pub span: Span,
    /// Explicit value (if provided).
    /// None = auto-increment from previous.
    pub explicit_value: Option<i64>,
}

impl UnresolvedEnum {
    pub fn new(name: QualifiedName, span: Span, unit_id: UnitId) -> Self {
        Self {
            name,
            span,
            unit_id,
            values: Vec::new(),
            is_shared: false,
        }
    }

    pub fn with_value(mut self, value: UnresolvedEnumValue) -> Self {
        self.values.push(value);
        self
    }

    pub fn with_shared(mut self) -> Self {
        self.is_shared = true;
        self
    }
}

impl UnresolvedEnumValue {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
            explicit_value: None,
        }
    }

    pub fn with_value(mut self, value: i64) -> Self {
        self.explicit_value = Some(value);
        self
    }
}

/// Unresolved global function declaration from Pass 1.
#[derive(Debug, Clone)]
pub struct UnresolvedFunction {
    /// Qualified name.
    pub name: QualifiedName,
    /// Source span.
    pub span: Span,
    /// Unit this function was declared in.
    pub unit_id: UnitId,
    /// Function signature.
    pub signature: UnresolvedSignature,
    /// Visibility.
    pub visibility: Visibility,
    /// Whether the function has a body.
    pub has_body: bool,
    /// Whether this is a shared function.
    pub is_shared: bool,
}

impl UnresolvedFunction {
    pub fn new(
        name: QualifiedName,
        span: Span,
        unit_id: UnitId,
        signature: UnresolvedSignature,
    ) -> Self {
        Self {
            name,
            span,
            unit_id,
            signature,
            visibility: Visibility::Public,
            has_body: true,
            is_shared: false,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_shared(mut self) -> Self {
        self.is_shared = true;
        self
    }

    pub fn declaration_only(mut self) -> Self {
        self.has_body = false;
        self
    }
}

/// Unresolved global variable declaration from Pass 1.
#[derive(Debug, Clone)]
pub struct UnresolvedGlobal {
    /// Qualified name.
    pub name: QualifiedName,
    /// Source span.
    pub span: Span,
    /// Unit this global was declared in.
    pub unit_id: UnitId,
    /// Variable type.
    pub var_type: UnresolvedType,
    /// Visibility.
    pub visibility: Visibility,
    /// Whether the variable has an initializer.
    pub has_initializer: bool,
    /// Whether this is a const global.
    pub is_const: bool,
    /// Whether this is a shared global.
    pub is_shared: bool,
}

impl UnresolvedGlobal {
    pub fn new(name: QualifiedName, span: Span, unit_id: UnitId, var_type: UnresolvedType) -> Self {
        Self {
            name,
            span,
            unit_id,
            var_type,
            visibility: Visibility::Public,
            has_initializer: false,
            is_const: false,
            is_shared: false,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_initializer(mut self) -> Self {
        self.has_initializer = true;
        self
    }

    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    pub fn with_shared(mut self) -> Self {
        self.is_shared = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_class_basic() {
        let name = QualifiedName::new("Player", vec!["Game".into()]);
        let class =
            UnresolvedClass::new(name.clone(), Span::default(), UnitId::new(0)).with_final();

        assert_eq!(class.name, name);
        assert!(class.is_final);
        assert!(!class.is_abstract);
    }

    #[test]
    fn unresolved_class_with_inheritance() {
        let name = QualifiedName::global("Player");
        let base = UnresolvedInheritance::new(UnresolvedType::simple("Entity"), Span::default());
        let iface =
            UnresolvedInheritance::new(UnresolvedType::simple("IDrawable"), Span::default());

        let class = UnresolvedClass::new(name, Span::default(), UnitId::new(0))
            .with_inheritance(base)
            .with_inheritance(iface);

        assert_eq!(class.inheritance.len(), 2);
    }

    #[test]
    fn unresolved_method_constructor() {
        let sig = UnresolvedSignature::new("Player", vec![], UnresolvedType::simple("void"));
        let method = UnresolvedMethod::new("Player", sig, Span::default())
            .with_kind(MethodKind::Constructor);

        assert!(method.is_constructor());
        assert!(!method.is_destructor());
    }

    #[test]
    fn unresolved_method_abstract() {
        let sig = UnresolvedSignature::new("draw", vec![], UnresolvedType::simple("void"));
        let method = UnresolvedMethod::new("draw", sig, Span::default()).with_abstract();

        assert!(method.is_abstract);
        assert!(!method.has_body);
    }

    #[test]
    fn unresolved_method_deleted() {
        let sig = UnresolvedSignature::new("copy", vec![], UnresolvedType::simple("void"));
        let method = UnresolvedMethod::new("copy", sig, Span::default()).with_deleted();

        assert!(method.is_deleted);
        assert!(!method.has_body);
    }

    #[test]
    fn unresolved_field() {
        let field = UnresolvedField::new("health", UnresolvedType::simple("int"), Span::default())
            .with_visibility(Visibility::Private)
            .with_initializer();

        assert_eq!(field.name, "health");
        assert_eq!(field.visibility, Visibility::Private);
        assert!(field.has_initializer);
    }

    #[test]
    fn unresolved_virtual_property() {
        let prop = UnresolvedVirtualProperty::new(
            "position",
            UnresolvedType::simple("vec3"),
            Span::default(),
        )
        .with_getter(UnresolvedAccessor::new(Span::default()).with_const())
        .with_setter(UnresolvedAccessor::new(Span::default()));

        assert_eq!(prop.name, "position");
        assert!(prop.getter.is_some());
        assert!(prop.getter.as_ref().unwrap().is_const);
        assert!(prop.setter.is_some());
    }

    #[test]
    fn unresolved_mixin() {
        let class = UnresolvedClass::new(
            QualifiedName::global("Serializable"),
            Span::default(),
            UnitId::new(0),
        );
        let mixin = UnresolvedMixin::new(class);

        assert_eq!(mixin.class.name.simple_name(), "Serializable");
    }

    #[test]
    fn unresolved_interface() {
        let name = QualifiedName::global("IDrawable");
        let method = UnresolvedSignature::new("draw", vec![], UnresolvedType::simple("void"));

        let iface = UnresolvedInterface::new(name.clone(), Span::default(), UnitId::new(0))
            .with_method(method)
            .with_shared();

        assert_eq!(iface.name, name);
        assert_eq!(iface.methods.len(), 1);
        assert!(iface.is_shared);
    }

    #[test]
    fn unresolved_interface_with_base() {
        let name = QualifiedName::global("IExtended");
        let base = UnresolvedInheritance::new(UnresolvedType::simple("IBase"), Span::default());

        let iface = UnresolvedInterface::new(name, Span::default(), UnitId::new(0)).with_base(base);

        assert_eq!(iface.bases.len(), 1);
    }

    #[test]
    fn unresolved_funcdef() {
        let name = QualifiedName::global("Callback");
        let params = vec![UnresolvedParam::new("arg", UnresolvedType::simple("int"))];
        let return_type = UnresolvedType::simple("void");

        let funcdef = UnresolvedFuncdef::new(
            name.clone(),
            Span::default(),
            UnitId::new(0),
            params,
            return_type,
        )
        .with_shared();

        assert_eq!(funcdef.name, name);
        assert_eq!(funcdef.params.len(), 1);
        assert!(funcdef.is_shared);
    }

    #[test]
    fn unresolved_enum() {
        let name = QualifiedName::global("Color");
        let red = UnresolvedEnumValue::new("Red", Span::default()).with_value(0);
        let green = UnresolvedEnumValue::new("Green", Span::default());

        let e = UnresolvedEnum::new(name.clone(), Span::default(), UnitId::new(0))
            .with_value(red)
            .with_value(green);

        assert_eq!(e.name, name);
        assert_eq!(e.values.len(), 2);
        assert_eq!(e.values[0].explicit_value, Some(0));
        assert_eq!(e.values[1].explicit_value, None);
    }

    #[test]
    fn unresolved_function() {
        let name = QualifiedName::new("update", vec!["Game".into()]);
        let sig = UnresolvedSignature::new(
            "update",
            vec![UnresolvedParam::new("dt", UnresolvedType::simple("float"))],
            UnresolvedType::simple("void"),
        );

        let func = UnresolvedFunction::new(name.clone(), Span::default(), UnitId::new(0), sig)
            .with_visibility(Visibility::Private)
            .with_shared();

        assert_eq!(func.name, name);
        assert_eq!(func.visibility, Visibility::Private);
        assert!(func.is_shared);
        assert!(func.has_body);
    }

    #[test]
    fn unresolved_function_declaration() {
        let name = QualifiedName::global("forward");
        let sig = UnresolvedSignature::new("forward", vec![], UnresolvedType::simple("void"));

        let func =
            UnresolvedFunction::new(name, Span::default(), UnitId::new(0), sig).declaration_only();

        assert!(!func.has_body);
    }

    #[test]
    fn unresolved_global() {
        let name = QualifiedName::global("MAX_PLAYERS");
        let var_type = UnresolvedType::simple("int");

        let global = UnresolvedGlobal::new(name.clone(), Span::default(), UnitId::new(0), var_type)
            .with_const()
            .with_initializer()
            .with_shared();

        assert_eq!(global.name, name);
        assert!(global.is_const);
        assert!(global.has_initializer);
        assert!(global.is_shared);
    }

    #[test]
    fn unresolved_global_mutable() {
        let name = QualifiedName::new("score", vec!["Game".into()]);
        let var_type = UnresolvedType::simple("int");

        let global = UnresolvedGlobal::new(name.clone(), Span::default(), UnitId::new(0), var_type)
            .with_visibility(Visibility::Protected);

        assert_eq!(global.name, name);
        assert!(!global.is_const);
        assert_eq!(global.visibility, Visibility::Protected);
    }
}
