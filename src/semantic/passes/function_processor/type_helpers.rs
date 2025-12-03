//! Type helper utilities.
//!
//! This module contains helper methods for type resolution, type checking,
//! visibility access, and name building.

use crate::ast::{
    expr::Expr,
    types::{TypeBase, TypeExpr, TypeSuffix},
};
use crate::lexer::Span;
use crate::semantic::{
    DataType, FieldDef, SemanticErrorKind, TypeDef, TypeId, Visibility,
    BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE, STRING_TYPE, VOID_TYPE,
    INT8_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE,
    UINT8_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE,
};
use crate::semantic::types::type_def::FunctionId;

use super::{FunctionCompiler, SwitchCategory};

impl<'ast> FunctionCompiler<'ast> {
    pub(super) fn build_qualified_name(&self, name: &str) -> String {
        Self::build_qualified_name_from_path(&self.namespace_path, name)
    }

    /// Build a scoped name from a Scope (without intermediate Vec allocation)
    pub(super) fn build_scope_name(scope: &crate::ast::Scope<'ast>) -> String {
        if scope.segments.is_empty() {
            return String::new();
        }
        // Calculate capacity: sum of segment lengths + "::" separators
        let capacity = scope.segments.iter().map(|s| s.name.len()).sum::<usize>()
            + (scope.segments.len() - 1) * 2;
        let mut result = String::with_capacity(capacity);
        for (i, segment) in scope.segments.iter().enumerate() {
            if i > 0 {
                result.push_str("::");
            }
            result.push_str(segment.name);
        }
        result
    }

    /// Build a qualified name from namespace path (without intermediate Vec allocation)
    pub(super) fn build_qualified_name_from_path(namespace_path: &[String], name: &str) -> String {
        if namespace_path.is_empty() {
            return name.to_string();
        }
        let capacity = namespace_path.iter().map(|s| s.len()).sum::<usize>()
            + namespace_path.len() * 2 + name.len();
        let mut result = String::with_capacity(capacity);
        for (i, part) in namespace_path.iter().enumerate() {
            if i > 0 {
                result.push_str("::");
            }
            result.push_str(part);
        }
        result.push_str("::");
        result.push_str(name);
        result
    }

    /// Look up a function in imported namespaces.
    /// Returns all matching candidates from all imported namespaces (as owned Vec).
    pub(super) fn lookup_function_in_imports(&self, name: &str) -> Vec<FunctionId> {
        for ns in &self.imported_namespaces {
            let qualified = format!("{}::{}", ns, name);
            let candidates = self.registry.lookup_functions(&qualified);
            if !candidates.is_empty() {
                return candidates.to_vec();
            }
        }
        Vec::new()
    }

    /// Visits a block of statements.



    pub(super) fn resolve_type_expr(&mut self, type_expr: &TypeExpr<'ast>) -> Option<DataType> {
        // Resolve the base type, considering scope/namespace
        let base_type_id = self.resolve_base_type(&type_expr.base, type_expr.scope.as_ref(), type_expr.span)?;

        // Handle template types (e.g., array<int>)
        let type_id = if !type_expr.template_args.is_empty() {
            // Build template instance name like "array<int>" or "array<array<int>>"
            // For nested templates, we need to recursively resolve the inner type first
            // to get its registered name (which uses canonical type names like "int" not "int32")
            let base_name = self.registry.get_type(base_type_id).name();

            // Collect arg names and calculate capacity
            let mut arg_names: Vec<&str> = Vec::with_capacity(type_expr.template_args.len());
            for arg in type_expr.template_args {
                // Recursively resolve the template argument to get its canonical name
                if let Some(resolved) = self.resolve_type_expr(arg) {
                    let typedef = self.registry.get_type(resolved.type_id);
                    arg_names.push(typedef.name());
                } else {
                    return None; // Error already reported
                }
            }

            // Build template name without intermediate allocations
            let capacity = base_name.len() + 2 + arg_names.iter().map(|n| n.len()).sum::<usize>()
                + if arg_names.len() > 1 { (arg_names.len() - 1) * 2 } else { 0 };
            let mut template_name = String::with_capacity(capacity);
            template_name.push_str(base_name);
            template_name.push('<');
            for (i, name) in arg_names.iter().enumerate() {
                if i > 0 {
                    template_name.push_str(", ");
                }
                template_name.push_str(name);
            }
            template_name.push('>');

            // Look up the instantiated template type
            if let Some(id) = self.registry.lookup_type(&template_name) {
                id
            } else {
                self.error(
                    SemanticErrorKind::UndefinedType,
                    type_expr.span,
                    format!("undefined template type '{}' - may need explicit declaration", template_name),
                );
                return None;
            }
        } else {
            base_type_id
        };

        // Build DataType with modifiers
        let mut data_type = DataType::simple(type_id);

        // Check if this is an array template instance - arrays are always reference types (handles)
        let typedef = self.registry.get_type(type_id);
        if let TypeDef::TemplateInstance { template, .. } = typedef
            && *template == self.registry.array_template {
                // Arrays are reference types, so they're implicitly handles
                data_type.is_handle = true;
            }

        // Apply leading const
        if type_expr.is_const {
            if data_type.is_handle {
                // For handle types, leading const means handle to const
                data_type.is_handle_to_const = true;
            } else {
                data_type.is_const = true;
            }
        }

        // Apply suffixes (handle, array)
        for suffix in type_expr.suffixes {
            match suffix {
                TypeSuffix::Handle { is_const } => {
                    // If already a handle, this is a const modifier on the handle
                    if data_type.is_handle && *is_const {
                        data_type.is_const = true;
                    } else {
                        data_type.is_handle = true;
                        if *is_const {
                            // @ const = const handle
                            data_type.is_const = true;
                        }
                        // Leading const with handle = handle to const
                        if type_expr.is_const && !*is_const {
                            data_type.is_handle_to_const = true;
                            data_type.is_const = false; // Reset since const applies to target
                        }
                    }
                }
                TypeSuffix::Array => {
                    // Array suffix - the type should be looked up as array<base>
                    // This is a complex case that would need template instantiation
                    // For now, we handle it by noting arrays are always handles
                    data_type.is_handle = true;
                }
            }
        }

        Some(data_type)
    }

    /// Resolve a base type (primitive or named) to a TypeId, considering scope and namespaces.
    pub(super) fn resolve_base_type(
        &mut self,
        base: &TypeBase<'ast>,
        scope: Option<&crate::ast::Scope<'ast>>,
        span: Span,
    ) -> Option<TypeId> {
        use crate::ast::types::TypeBase;

        match base {
            TypeBase::Primitive(prim) => Some(self.primitive_to_type_id(*prim)),

            TypeBase::Named(ident) => {
                // Build the qualified name based on scope
                if let Some(scope) = scope {
                    // Scoped type: Namespace::Type
                    let type_name = self.build_scoped_name(scope, ident.name);
                    if let Some(type_id) = self.registry.lookup_type(&type_name) {
                        return Some(type_id);
                    }
                    self.error(
                        SemanticErrorKind::UndefinedType,
                        span,
                        format!("undefined type '{}'", type_name),
                    );
                    None
                } else {
                    // Try current namespace first, then ancestor namespaces, then global
                    // For namespace_path = ["Utils", "Colors"], try:
                    //   1. Utils::Colors::Color
                    //   2. Utils::Color
                    //   3. Color (global)
                    let qualified = self.build_qualified_name(ident.name);

                    // Look up in registry
                    if let Some(type_id) = self.registry.lookup_type(&qualified) {
                        return Some(type_id);
                    }

                    // Try progressively shorter namespace prefixes
                    if !self.namespace_path.is_empty() {
                        for prefix_len in (1..self.namespace_path.len()).rev() {
                            // Build ancestor qualified name without intermediate allocations
                            let ancestor_qualified = Self::build_qualified_name_from_path(
                                &self.namespace_path[..prefix_len],
                                ident.name,
                            );
                            if let Some(type_id) = self.registry.lookup_type(&ancestor_qualified) {
                                return Some(type_id);
                            }
                        }
                    }

                    // Try global scope
                    if let Some(type_id) = self.registry.lookup_type(ident.name) {
                        return Some(type_id);
                    }

                    // Try imported namespaces
                    let mut found_in_import: Option<TypeId> = None;
                    for ns in &self.imported_namespaces {
                        let imported_qualified = format!("{}::{}", ns, ident.name);
                        if let Some(type_id) = self.registry.lookup_type(&imported_qualified) {
                            if found_in_import.is_some() {
                                // Ambiguous - found in multiple imported namespaces
                                self.error(
                                    SemanticErrorKind::AmbiguousName,
                                    span,
                                    format!("ambiguous type '{}' found in multiple imported namespaces", ident.name),
                                );
                                return None;
                            }
                            found_in_import = Some(type_id);
                        }
                    }
                    if let Some(type_id) = found_in_import {
                        return Some(type_id);
                    }

                    // Not found anywhere
                    self.error(
                        SemanticErrorKind::UndefinedType,
                        span,
                        format!("undefined type '{}'", ident.name),
                    );
                    None
                }
            }

            TypeBase::Auto => {
                // Auto type should be handled by the caller before reaching here
                self.error(
                    SemanticErrorKind::UndefinedType,
                    span,
                    "auto type inference not valid in this context".to_string(),
                );
                None
            }

            TypeBase::Unknown => {
                self.error(
                    SemanticErrorKind::UndefinedType,
                    span,
                    "unknown type '?'".to_string(),
                );
                None
            }

            TypeBase::TemplateParam(_) => {
                // Template parameters (e.g., "class T" in "array<class T>") are placeholders
                // used in FFI template type declarations. They are not resolved to a TypeId;
                // instead, they are captured separately as template parameter names.
                // Returning None here allows concrete types in mixed declarations like
                // "stringmap<string, class T>" to be resolved normally.
                None
            }
        }
    }

    /// Map a primitive type to its TypeId
    #[inline]
    pub(super) fn primitive_to_type_id(&self, prim: crate::ast::types::PrimitiveType) -> TypeId {
        use crate::ast::types::PrimitiveType;
        match prim {
            PrimitiveType::Void => VOID_TYPE,
            PrimitiveType::Bool => BOOL_TYPE,
            PrimitiveType::Int => INT32_TYPE,
            PrimitiveType::Int8 => INT8_TYPE,
            PrimitiveType::Int16 => INT16_TYPE,
            PrimitiveType::Int64 => INT64_TYPE,
            PrimitiveType::UInt => UINT32_TYPE,
            PrimitiveType::UInt8 => UINT8_TYPE,
            PrimitiveType::UInt16 => UINT16_TYPE,
            PrimitiveType::UInt64 => UINT64_TYPE,
            PrimitiveType::Float => FLOAT_TYPE,
            PrimitiveType::Double => DOUBLE_TYPE,
        }
    }

    /// Build a scoped name from a Scope and a name (no intermediate Vec allocation)
    pub(super) fn build_scoped_name(&self, scope: &crate::ast::Scope<'ast>, name: &str) -> String {
        let scope_name = Self::build_scope_name(&scope);
        let mut result = String::with_capacity(scope_name.len() + 2 + name.len());
        result.push_str(&scope_name);
        result.push_str("::");
        result.push_str(name);
        result
    }

    /// Checks if a value can be assigned to a target type.
    ///
    /// Returns true if:
    /// - Types are identical, OR
    /// - An implicit conversion exists from value to target
    pub(super) fn is_assignable(&self, value: &DataType, target: &DataType) -> bool {
        if let Some(conversion) = value.can_convert_to(target, self.registry) {
            conversion.is_implicit
        } else {
            false
        }
    }

    /// Checks if a type is numeric (includes all integer types, floats, and enums).
    pub(super) fn is_numeric(&self, ty: &DataType) -> bool {
        if matches!(
            ty.type_id,
            INT8_TYPE | INT16_TYPE | INT32_TYPE | INT64_TYPE |
            UINT8_TYPE | UINT16_TYPE | UINT32_TYPE | UINT64_TYPE |
            FLOAT_TYPE | DOUBLE_TYPE
        ) {
            return true;
        }
        // Enum types are also numeric (int32 values)
        self.registry.get_type(ty.type_id).is_enum()
    }

    /// Checks if a type is an integer type (includes enums since they're int32 underneath).
    pub(super) fn is_integer(&self, ty: &DataType) -> bool {
        if matches!(
            ty.type_id,
            INT8_TYPE | INT16_TYPE | INT32_TYPE | INT64_TYPE |
            UINT8_TYPE | UINT16_TYPE | UINT32_TYPE | UINT64_TYPE
        ) {
            return true;
        }
        // Enum types are also integers (int32 values)
        self.registry.get_type(ty.type_id).is_enum()
    }

    /// Checks if a type can be used in bitwise operations (integers and bool).
    /// Bool is implicitly converted to 0 or 1 for bitwise ops.
    pub(super) fn is_bitwise_compatible(&self, ty: &DataType) -> bool {
        self.is_integer(ty) || ty.type_id == BOOL_TYPE
    }

    /// Checks if a type is compatible with switch statements (integer or enum).
    /// Determines the switch category for a type, or None if not switch-compatible.
    pub(super) fn get_switch_category(&self, ty: &DataType) -> Option<SwitchCategory> {
        // Handle types support type pattern matching
        if ty.is_handle {
            return Some(SwitchCategory::Handle);
        }

        // Integer types (int8-64, uint8-64)
        if self.is_integer(ty) {
            return Some(SwitchCategory::Integer);
        }

        // Enum types (treated as integers)
        let typedef = self.registry.get_type(ty.type_id);
        if typedef.is_enum() {
            return Some(SwitchCategory::Integer);
        }

        // Boolean
        if ty.type_id == BOOL_TYPE {
            return Some(SwitchCategory::Bool);
        }

        // Float/Double
        if ty.type_id == FLOAT_TYPE || ty.type_id == DOUBLE_TYPE {
            return Some(SwitchCategory::Float);
        }

        // String
        if ty.type_id == STRING_TYPE {
            return Some(SwitchCategory::String);
        }

        None
    }

    /// Try to resolve a case expression as a type pattern (class/interface name).
    /// Returns Some(TypeId) if the expression is an identifier that resolves to a class or interface.
    pub(super) fn try_resolve_type_pattern(&self, expr: &Expr) -> Option<TypeId> {
        // Only identifiers can be type patterns
        if let Expr::Ident(ident) = expr {
            // Look up as type name, not variable
            if let Some(type_id) = self.registry.lookup_type(ident.ident.name) {
                let typedef = self.registry.get_type(type_id);
                // Only classes and interfaces are valid type patterns
                if typedef.is_class() || typedef.is_interface() {
                    return Some(type_id);
                }
            }
        }
        None
    }

    /// Promotes two numeric types to their common type.
    pub(super) fn promote_numeric(&self, left: &DataType, right: &DataType) -> DataType {
        // Simplified promotion rules
        if left.type_id == DOUBLE_TYPE || right.type_id == DOUBLE_TYPE {
            DataType::simple(DOUBLE_TYPE)
        } else if left.type_id == FLOAT_TYPE || right.type_id == FLOAT_TYPE {
            DataType::simple(FLOAT_TYPE)
        } else if left.type_id == INT64_TYPE || right.type_id == INT64_TYPE {
            DataType::simple(INT64_TYPE)
        } else {
            DataType::simple(INT32_TYPE)
        }
    }

    /// Gets a human-readable name for a type.
    pub(super) fn type_name(&self, ty: &DataType) -> String {
        let type_def = self.registry.get_type(ty.type_id);
        type_def.name().to_string()
    }

    /// Checks if access to a member with the given visibility is allowed from the current context.
    ///
    /// Returns true if access is allowed, false if it would be a visibility violation.
    ///
    /// Access rules:
    /// - `Public`: Always accessible
    /// - `Private`: Only accessible within the same class
    /// - `Protected`: Accessible within the same class or derived classes
    pub(super) fn check_visibility_access(&self, visibility: Visibility, member_class: TypeId) -> bool {
        match visibility {
            Visibility::Public => true,
            Visibility::Private => {
                // Private: only accessible if we're compiling code within the same class
                self.current_class == Some(member_class)
            }
            Visibility::Protected => {
                // Protected: accessible within the class or any derived class
                match self.current_class {
                    None => false,
                    Some(current_class_id) => {
                        // Same class - always allowed
                        if current_class_id == member_class {
                            return true;
                        }
                        // Check if current class is derived from member_class
                        self.registry.is_subclass_of(current_class_id, member_class)
                    }
                }
            }
        }
    }

    /// Finds a field by name in the class hierarchy (including inherited fields).
    ///
    /// Returns Some((field_index, field_def, defining_class_id)) if found,
    /// where field_index is the position within the defining class's fields,
    /// and defining_class_id is the TypeId of the class that defines the field.
    ///
    /// Searches the immediate class first, then walks up the inheritance chain.
    pub(super) fn find_field_in_hierarchy(
        &self,
        class_id: TypeId,
        field_name: &str,
    ) -> Option<(usize, FieldDef, TypeId)> {
        let mut current_class_id = Some(class_id);

        while let Some(cid) = current_class_id {
            let typedef = self.registry.get_type(cid);
            match typedef {
                TypeDef::Class { fields, base_class, .. } => {
                    // Check fields in this class
                    for (idx, field) in fields.iter().enumerate() {
                        if field.name == field_name {
                            return Some((idx, field.clone(), cid));
                        }
                    }
                    // Move to base class
                    current_class_id = *base_class;
                }
                _ => break,
            }
        }
        None
    }

    /// Reports an access violation error with detailed message.
    pub(super) fn report_access_violation(
        &mut self,
        visibility: Visibility,
        member_name: &str,
        member_class_name: &str,
        span: Span,
    ) {
        let visibility_str = match visibility {
            Visibility::Public => "public",
            Visibility::Private => "private",
            Visibility::Protected => "protected",
        };
        self.error(
            SemanticErrorKind::AccessViolation,
            span,
            format!(
                "cannot access {} member '{}' of class '{}'",
                visibility_str, member_name, member_class_name
            ),
        );
    }

    pub(super) fn get_base_class_by_name(&self, class_id: TypeId, name: &str) -> Option<TypeId> {
        let class_def = self.registry.get_type(class_id);
        if let TypeDef::Class { base_class, .. } = class_def {
            if let Some(base_id) = base_class {
                let base_def = self.registry.get_type(*base_id);
                // Check if the base class name matches (short name only)
                if base_def.name() == name {
                    return Some(*base_id);
                }
                // Recursively check further up the chain
                return self.get_base_class_by_name(*base_id, name);
            }
        }
        None
    }
}
