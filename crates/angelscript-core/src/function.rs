use crate::types::{AccessModifier, DataType, QualifiedName};
use std::fmt;

/// Unique identifier for a compiled function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FunctionId({})", self.0)
    }
}

/// What kind of function this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionKind {
    /// Compiled from script source.
    Script,
    /// Registered from host application (native).
    System,
    /// Interface method declaration (no body).
    Interface,
    /// Virtual method entry in a vtable.
    Virtual,
    /// Function definition (function pointer type).
    Funcdef,
    /// Delegate binding (object + function).
    Delegate,
}

impl fmt::Display for FunctionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionKind::Script => write!(f, "script"),
            FunctionKind::System => write!(f, "system"),
            FunctionKind::Interface => write!(f, "interface"),
            FunctionKind::Virtual => write!(f, "virtual"),
            FunctionKind::Funcdef => write!(f, "funcdef"),
            FunctionKind::Delegate => write!(f, "delegate"),
        }
    }
}

/// Bitflags for function traits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FunctionTraits(u16);

impl FunctionTraits {
    pub const CONSTRUCTOR: u16 = 1 << 0;
    pub const DESTRUCTOR: u16 = 1 << 1;
    pub const CONST: u16 = 1 << 2;
    pub const PRIVATE: u16 = 1 << 3;
    pub const PROTECTED: u16 = 1 << 4;
    pub const FINAL: u16 = 1 << 5;
    pub const OVERRIDE: u16 = 1 << 6;
    pub const SHARED: u16 = 1 << 7;
    pub const EXPLICIT: u16 = 1 << 8;
    pub const PROPERTY: u16 = 1 << 9;
    pub const ABSTRACT: u16 = 1 << 10;
    pub const DELETED: u16 = 1 << 11;

    pub fn new() -> Self {
        FunctionTraits(0)
    }

    pub fn set(mut self, flag: u16) -> Self {
        self.0 |= flag;
        self
    }

    pub fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    pub fn is_constructor(self) -> bool {
        self.has(Self::CONSTRUCTOR)
    }

    pub fn is_destructor(self) -> bool {
        self.has(Self::DESTRUCTOR)
    }

    pub fn is_const(self) -> bool {
        self.has(Self::CONST)
    }

    pub fn is_private(self) -> bool {
        self.has(Self::PRIVATE)
    }

    pub fn is_protected(self) -> bool {
        self.has(Self::PROTECTED)
    }

    pub fn is_final(self) -> bool {
        self.has(Self::FINAL)
    }

    pub fn is_override(self) -> bool {
        self.has(Self::OVERRIDE)
    }

    pub fn is_shared(self) -> bool {
        self.has(Self::SHARED)
    }

    pub fn is_explicit(self) -> bool {
        self.has(Self::EXPLICIT)
    }

    pub fn is_property(self) -> bool {
        self.has(Self::PROPERTY)
    }

    pub fn is_abstract(self) -> bool {
        self.has(Self::ABSTRACT)
    }

    pub fn is_deleted(self) -> bool {
        self.has(Self::DELETED)
    }

    /// Get the access modifier implied by these traits.
    pub fn access_modifier(self) -> AccessModifier {
        if self.is_private() {
            AccessModifier::Private
        } else if self.is_protected() {
            AccessModifier::Protected
        } else {
            AccessModifier::Public
        }
    }
}

/// Direction of a function parameter reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamDirection {
    In,
    Out,
    InOut,
}

/// Information about a single function parameter.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name (may be empty for system functions).
    pub name: String,
    /// Parameter type.
    pub data_type: DataType,
    /// Reference direction, if any.
    pub direction: Option<ParamDirection>,
    /// Whether this parameter has a default value.
    pub has_default: bool,
}

impl ParamInfo {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        ParamInfo {
            name: name.into(),
            data_type,
            direction: None,
            has_default: false,
        }
    }

    pub fn with_direction(mut self, dir: ParamDirection) -> Self {
        self.direction = Some(dir);
        self
    }

    pub fn with_default(mut self) -> Self {
        self.has_default = true;
        self
    }
}

/// Complete function signature.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Fully qualified function name.
    pub name: QualifiedName,
    /// Return type.
    pub return_type: DataType,
    /// Parameter list.
    pub params: Vec<ParamInfo>,
    /// Function traits.
    pub traits: FunctionTraits,
}

impl FunctionSignature {
    pub fn new(name: QualifiedName, return_type: DataType, params: Vec<ParamInfo>) -> Self {
        FunctionSignature {
            name,
            return_type,
            params,
            traits: FunctionTraits::new(),
        }
    }

    pub fn with_traits(mut self, traits: FunctionTraits) -> Self {
        self.traits = traits;
        self
    }

    /// Number of required parameters (excluding those with defaults).
    pub fn required_param_count(&self) -> usize {
        self.params.iter().filter(|p| !p.has_default).count()
    }

    /// Total parameter count.
    pub fn param_count(&self) -> usize {
        self.params.len()
    }
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}(", self.return_type, self.name)?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param.data_type)?;
            if !param.name.is_empty() {
                write!(f, " {}", param.name)?;
            }
        }
        write!(f, ")")?;
        if self.traits.is_const() {
            write!(f, " const")?;
        }
        Ok(())
    }
}

/// Information about an object property.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property name.
    pub name: String,
    /// Property type.
    pub data_type: DataType,
    /// Byte offset within the object.
    pub byte_offset: usize,
    /// Access modifier.
    pub access: AccessModifier,
    /// Whether this property was inherited from a base class.
    pub is_inherited: bool,
}

impl PropertyInfo {
    pub fn new(name: impl Into<String>, data_type: DataType, byte_offset: usize) -> Self {
        PropertyInfo {
            name: name.into(),
            data_type,
            byte_offset,
            access: AccessModifier::Public,
            is_inherited: false,
        }
    }

    pub fn with_access(mut self, access: AccessModifier) -> Self {
        self.access = access;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrimitiveType;

    #[test]
    fn function_traits() {
        let traits = FunctionTraits::new()
            .set(FunctionTraits::CONST)
            .set(FunctionTraits::PRIVATE);
        assert!(traits.is_const());
        assert!(traits.is_private());
        assert!(!traits.is_final());
        assert_eq!(traits.access_modifier(), AccessModifier::Private);
    }

    #[test]
    fn function_signature_display() {
        let sig = FunctionSignature::new(
            QualifiedName::global("add"),
            DataType::primitive(PrimitiveType::Int32),
            vec![
                ParamInfo::new("a", DataType::primitive(PrimitiveType::Int32)),
                ParamInfo::new("b", DataType::primitive(PrimitiveType::Int32)),
            ],
        );
        let s = format!("{}", sig);
        assert!(s.contains("int add(int a, int b)"));
    }

    #[test]
    fn function_signature_required_params() {
        let sig = FunctionSignature::new(
            QualifiedName::global("foo"),
            DataType::void(),
            vec![
                ParamInfo::new("a", DataType::primitive(PrimitiveType::Int32)),
                ParamInfo::new("b", DataType::primitive(PrimitiveType::Int32)).with_default(),
            ],
        );
        assert_eq!(sig.required_param_count(), 1);
        assert_eq!(sig.param_count(), 2);
    }
}
