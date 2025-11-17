#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineProperty {
    AllowUnsafeReferences,
    OptimizeBytecode,
    BuildWithoutLineCues,
    IncludeDebugInfo,
    TrackLocalScopes,
    StoreDocComments,
    MaxCallStackSize,
    InitContextStackSize,
    UseCharacterLiterals,
    AllowMultilineStrings,
    DisallowEmptyListElements,
    DisallowValueAssignForRefType,
    AlwaysImplDefaultConstruct,
    CompilerWarnings,
    DisallowGlobalVars,
    RequireEnumScope,
}

impl EngineProperty {
    pub fn default_value(&self) -> usize {
        match self {
            EngineProperty::AllowUnsafeReferences => 0,
            EngineProperty::OptimizeBytecode => 1,
            EngineProperty::BuildWithoutLineCues => 0,
            EngineProperty::IncludeDebugInfo => 0,
            EngineProperty::TrackLocalScopes => 0,
            EngineProperty::StoreDocComments => 0,
            EngineProperty::MaxCallStackSize => 10000,
            EngineProperty::InitContextStackSize => 1024,
            EngineProperty::UseCharacterLiterals => 0,
            EngineProperty::AllowMultilineStrings => 0,
            EngineProperty::DisallowEmptyListElements => 0,
            EngineProperty::DisallowValueAssignForRefType => 0,
            EngineProperty::AlwaysImplDefaultConstruct => 0,
            EngineProperty::CompilerWarnings => 1,
            EngineProperty::DisallowGlobalVars => 0,
            EngineProperty::RequireEnumScope => 0,
        }
    }
}
