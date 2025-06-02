#include "as_function.h"

extern "C" {

// Function management
asIScriptEngine* asFunction_GetEngine(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetEngine();
}

int asFunction_AddRef(asIScriptFunction *f) {
    if (!f) return asINVALID_ARG;
    return static_cast<::asIScriptFunction*>(f)->AddRef();
}

int asFunction_Release(asIScriptFunction *f) {
    if (!f) return asINVALID_ARG;
    return static_cast<::asIScriptFunction*>(f)->Release();
}

// Function info
int asFunction_GetId(asIScriptFunction *f) {
    if (!f) return 0;
    return static_cast<::asIScriptFunction*>(f)->GetId();
}

asEFuncType asFunction_GetFuncType(asIScriptFunction *f) {
    if (!f) return asFUNC_DUMMY;
    return static_cast<asEFuncType>(static_cast<::asIScriptFunction*>(f)->GetFuncType());
}

const char* asFunction_GetModuleName(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetModuleName();
}

asIScriptModule* asFunction_GetModule(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetModule();
}

const char* asFunction_GetScriptSectionName(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetScriptSectionName();
}

const char* asFunction_GetConfigGroup(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetConfigGroup();
}

asDWORD asFunction_GetAccessMask(asIScriptFunction *f) {
    if (!f) return 0;
    return static_cast<::asIScriptFunction*>(f)->GetAccessMask();
}

void* asFunction_GetAuxiliary(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetAuxiliary();
}

// Function signature
asITypeInfo* asFunction_GetObjectType(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetObjectType();
}

const char* asFunction_GetObjectName(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetObjectName();
}

const char* asFunction_GetName(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetName();
}

const char* asFunction_GetNamespace(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetNamespace();
}

const char* asFunction_GetDeclaration(asIScriptFunction *f, asBOOL includeObjectName, asBOOL includeNamespace, asBOOL includeParamNames) {
    if (!f) return nullptr;
    bool inclObjName = includeObjectName ? true : false;
    bool inclNs = includeNamespace ? true : false;
    bool inclParamNames = includeParamNames ? true : false;
    return static_cast<::asIScriptFunction*>(f)->GetDeclaration(inclObjName, inclNs, inclParamNames);
}

asBOOL asFunction_IsReadOnly(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsReadOnly() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsPrivate(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsPrivate() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsProtected(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsProtected() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsFinal(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsFinal() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsOverride(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsOverride() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsShared(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsShared() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsExplicit(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsExplicit() ? asTRUE : asFALSE;
}

asBOOL asFunction_IsProperty(asIScriptFunction *f) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsProperty() ? asTRUE : asFALSE;
}

// Parameters
asUINT asFunction_GetParamCount(asIScriptFunction *f) {
    if (!f) return 0;
    return static_cast<::asIScriptFunction*>(f)->GetParamCount();
}

int asFunction_GetParam(asIScriptFunction *f, asUINT index, int *typeId, asDWORD *flags, const char **name, const char **defaultArg) {
    if (!f) return asINVALID_ARG;
    return static_cast<::asIScriptFunction*>(f)->GetParam(index, typeId, flags, name, defaultArg);
}

// Return type
int asFunction_GetReturnTypeId(asIScriptFunction *f, asDWORD *flags) {
    if (!f) return 0;
    return static_cast<::asIScriptFunction*>(f)->GetReturnTypeId(flags);
}

// Type id for function pointers
int asFunction_GetTypeId(asIScriptFunction *f) {
    if (!f) return 0;
    return static_cast<::asIScriptFunction*>(f)->GetTypeId();
}

asBOOL asFunction_IsCompatibleWithTypeId(asIScriptFunction *f, int typeId) {
    if (!f) return asFALSE;
    return static_cast<::asIScriptFunction*>(f)->IsCompatibleWithTypeId(typeId) ? asTRUE : asFALSE;
}

// Delegates
void* asFunction_GetDelegateObject(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetDelegateObject();
}

asITypeInfo* asFunction_GetDelegateObjectType(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetDelegateObjectType();
}

asIScriptFunction* asFunction_GetDelegateFunction(asIScriptFunction *f) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetDelegateFunction();
}

// Debug info
asUINT asFunction_GetVarCount(asIScriptFunction *f) {
    if (!f) return 0;
    return static_cast<::asIScriptFunction*>(f)->GetVarCount();
}

int asFunction_GetVar(asIScriptFunction *f, asUINT index, const char **name, int *typeId) {
    if (!f) return asINVALID_ARG;
    return static_cast<::asIScriptFunction*>(f)->GetVar(index, name, typeId);
}

const char* asFunction_GetVarDecl(asIScriptFunction *f, asUINT index, asBOOL includeNamespace) {
    if (!f) return nullptr;
    bool inclNs = includeNamespace ? true : false;
    return static_cast<::asIScriptFunction*>(f)->GetVarDecl(index, inclNs);
}

int asFunction_FindNextLineWithCode(asIScriptFunction *f, int line) {
    if (!f) return -1;
    return static_cast<::asIScriptFunction*>(f)->FindNextLineWithCode(line);
}

// For JIT compilation
asDWORD* asFunction_GetByteCode(asIScriptFunction *f, asUINT *length) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetByteCode(length);
}

// User data
void* asFunction_GetUserData(asIScriptFunction *f, asPWORD type) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->GetUserData(type);
}

void* asFunction_SetUserData(asIScriptFunction *f, void *data, asPWORD type) {
    if (!f) return nullptr;
    return static_cast<::asIScriptFunction*>(f)->SetUserData(data, type);
}

} // extern "C"
