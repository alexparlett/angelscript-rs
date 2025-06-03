#include "as_module.h"

extern "C" {

// Module management
asIScriptEngine* asModule_GetEngine(asIScriptModule *m) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetEngine();
}

void asModule_SetName(asIScriptModule *m, const char *name) {
    if (!m || !name) return;
    static_cast<::asIScriptModule*>(m)->SetName(name);
}

const char* asModule_GetName(asIScriptModule *m) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetName();
}

void asModule_Discard(asIScriptModule *m) {
    if (!m) return;
    static_cast<::asIScriptModule*>(m)->Discard();
}

// Script sections
int asModule_AddScriptSection(asIScriptModule *m, const char *name, const char *code, size_t codeLength, int lineOffset) {
    if (!m || !name || !code) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->AddScriptSection(name, code, codeLength, lineOffset);
}

// Build
int asModule_Build(asIScriptModule *m) {
    if (!m) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->Build();
}

int asModule_CompileFunction(asIScriptModule *m, const char *sectionName, const char *code, int lineOffset, asDWORD compileFlags, asIScriptFunction **outFunc) {
    if (!m || !sectionName || !code || !outFunc) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->CompileFunction(sectionName, code, lineOffset, compileFlags, reinterpret_cast<::asIScriptFunction**>(outFunc));
}

int asModule_CompileGlobalVar(asIScriptModule *m, const char *sectionName, const char *code, int lineOffset) {
    if (!m || !sectionName || !code) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->CompileGlobalVar(sectionName, code, lineOffset);
}

// Namespaces
int asModule_SetDefaultNamespace(asIScriptModule *m, const char *nameSpace) {
    if (!m || !nameSpace) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->SetDefaultNamespace(nameSpace);
}

const char* asModule_GetDefaultNamespace(asIScriptModule *m) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetDefaultNamespace();
}

// Functions
asUINT asModule_GetFunctionCount(asIScriptModule *m) {
    if (!m) return 0;
    return static_cast<::asIScriptModule*>(m)->GetFunctionCount();
}

asIScriptFunction* asModule_GetFunctionByIndex(asIScriptModule *m, asUINT index) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetFunctionByIndex(index);
}

asIScriptFunction* asModule_GetFunctionByDecl(asIScriptModule *m, const char *decl) {
    if (!m || !decl) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetFunctionByDecl(decl);
}

asIScriptFunction* asModule_GetFunctionByName(asIScriptModule *m, const char *name) {
    if (!m || !name) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetFunctionByName(name);
}

int asModule_RemoveFunction(asIScriptModule *m, asIScriptFunction *func) {
    if (!m || !func) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->RemoveFunction(static_cast<::asIScriptFunction*>(func));
}

// Global variables
int asModule_ResetGlobalVars(asIScriptModule *m, asIScriptContext *ctx) {
    if (!m) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->ResetGlobalVars(static_cast<::asIScriptContext*>(ctx));
}

asUINT asModule_GetGlobalVarCount(asIScriptModule *m) {
    if (!m) return 0;
    return static_cast<::asIScriptModule*>(m)->GetGlobalVarCount();
}

int asModule_GetGlobalVarIndexByName(asIScriptModule *m, const char *name) {
    if (!m || !name) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->GetGlobalVarIndexByName(name);
}

int asModule_GetGlobalVarIndexByDecl(asIScriptModule *m, const char *decl) {
    if (!m || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->GetGlobalVarIndexByDecl(decl);
}

const char* asModule_GetGlobalVarDeclaration(asIScriptModule *m, asUINT index, bool includeNamespace) {
    if (!m) return nullptr;
    bool includeNs = includeNamespace ? true : false;
    return static_cast<::asIScriptModule*>(m)->GetGlobalVarDeclaration(index, includeNs);
}

int asModule_GetGlobalVar(asIScriptModule *m, asUINT index, const char **name, const char **nameSpace, int *typeId, bool *isConst) {
    if (!m) return asINVALID_ARG;
    bool constFlag;
    int result = static_cast<::asIScriptModule*>(m)->GetGlobalVar(index, name, nameSpace, typeId, &constFlag);
    if (isConst) *isConst = constFlag ? true : false;
    return result;
}

void* asModule_GetAddressOfGlobalVar(asIScriptModule *m, asUINT index) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetAddressOfGlobalVar(index);
}

int asModule_RemoveGlobalVar(asIScriptModule *m, asUINT index) {
    if (!m) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->RemoveGlobalVar(index);
}

// Type identification
asUINT asModule_GetObjectTypeCount(asIScriptModule *m) {
    if (!m) return 0;
    return static_cast<::asIScriptModule*>(m)->GetObjectTypeCount();
}

asITypeInfo* asModule_GetObjectTypeByIndex(asIScriptModule *m, asUINT index) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetObjectTypeByIndex(index);
}

int asModule_GetTypeIdByDecl(asIScriptModule *m, const char *decl) {
    if (!m || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->GetTypeIdByDecl(decl);
}

asITypeInfo* asModule_GetTypeInfoByName(asIScriptModule *m, const char *name) {
    if (!m || !name) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetTypeInfoByName(name);
}

asITypeInfo* asModule_GetTypeInfoByDecl(asIScriptModule *m, const char *decl) {
    if (!m || !decl) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetTypeInfoByDecl(decl);
}

// Enums
asUINT asModule_GetEnumCount(asIScriptModule *m) {
    if (!m) return 0;
    return static_cast<::asIScriptModule*>(m)->GetEnumCount();
}

asITypeInfo* asModule_GetEnumByIndex(asIScriptModule *m, asUINT index) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetEnumByIndex(index);
}

// Typedefs
asUINT asModule_GetTypedefCount(asIScriptModule *m) {
    if (!m) return 0;
    return static_cast<::asIScriptModule*>(m)->GetTypedefCount();
}

asITypeInfo* asModule_GetTypedefByIndex(asIScriptModule *m, asUINT index) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetTypedefByIndex(index);
}

// Imports
asUINT asModule_GetImportedFunctionCount(asIScriptModule *m) {
    if (!m) return 0;
    return static_cast<::asIScriptModule*>(m)->GetImportedFunctionCount();
}

int asModule_GetImportedFunctionIndexByDecl(asIScriptModule *m, const char *decl) {
    if (!m || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->GetImportedFunctionIndexByDecl(decl);
}

const char* asModule_GetImportedFunctionDeclaration(asIScriptModule *m, asUINT importIndex) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetImportedFunctionDeclaration(importIndex);
}

const char* asModule_GetImportedFunctionSourceModule(asIScriptModule *m, asUINT importIndex) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetImportedFunctionSourceModule(importIndex);
}

int asModule_BindImportedFunction(asIScriptModule *m, asUINT importIndex, asIScriptFunction *func) {
    if (!m || !func) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->BindImportedFunction(importIndex, static_cast<::asIScriptFunction*>(func));
}

int asModule_UnbindImportedFunction(asIScriptModule *m, asUINT importIndex) {
    if (!m) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->UnbindImportedFunction(importIndex);
}

int asModule_BindAllImportedFunctions(asIScriptModule *m) {
    if (!m) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->BindAllImportedFunctions();
}

int asModule_UnbindAllImportedFunctions(asIScriptModule *m) {
    if (!m) return asINVALID_ARG;
    return static_cast<::asIScriptModule*>(m)->UnbindAllImportedFunctions();
}

// Bytecode
int asModule_SaveByteCode(asIScriptModule *m, asIBinaryStream *out, bool stripDebugInfo) {
    if (!m || !out) return asINVALID_ARG;
    bool strip = stripDebugInfo ? true : false;
    return static_cast<::asIScriptModule*>(m)->SaveByteCode(static_cast<::asIBinaryStream*>(out), strip);
}

int asModule_LoadByteCode(asIScriptModule *m, asIBinaryStream *in, bool *wasDebugInfoStripped) {
    if (!m || !in) return asINVALID_ARG;
    bool stripped;
    int result = static_cast<::asIScriptModule*>(m)->LoadByteCode(static_cast<::asIBinaryStream*>(in), &stripped);
    if (wasDebugInfoStripped) *wasDebugInfoStripped = stripped ? true : false;
    return result;
}

// User data
void* asModule_GetUserData(asIScriptModule *m, asPWORD type) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->GetUserData(type);
}

void* asModule_SetUserData(asIScriptModule *m, void *data, asPWORD type) {
    if (!m) return nullptr;
    return static_cast<::asIScriptModule*>(m)->SetUserData(data, type);
}

} // extern "C"
