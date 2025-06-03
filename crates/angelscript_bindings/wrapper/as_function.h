#ifndef AS_FUNCTION_H
#define AS_FUNCTION_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif
#include "as_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Function management
asIScriptEngine* asFunction_GetEngine(asIScriptFunction *f);
int asFunction_AddRef(asIScriptFunction *f);
int asFunction_Release(asIScriptFunction *f);

// Function info
int asFunction_GetId(asIScriptFunction *f);
asEFuncType asFunction_GetFuncType(asIScriptFunction *f);
const char* asFunction_GetModuleName(asIScriptFunction *f);
asIScriptModule* asFunction_GetModule(asIScriptFunction *f);
const char* asFunction_GetScriptSectionName(asIScriptFunction *f);
const char* asFunction_GetConfigGroup(asIScriptFunction *f);
asDWORD asFunction_GetAccessMask(asIScriptFunction *f);
void* asFunction_GetAuxiliary(asIScriptFunction *f);

// Function signature
asITypeInfo* asFunction_GetObjectType(asIScriptFunction *f);
const char* asFunction_GetObjectName(asIScriptFunction *f);
const char* asFunction_GetName(asIScriptFunction *f);
const char* asFunction_GetNamespace(asIScriptFunction *f);
const char* asFunction_GetDeclaration(asIScriptFunction *f, bool includeObjectName, bool includeNamespace, bool includeParamNames);
bool asFunction_IsReadOnly(asIScriptFunction *f);
bool asFunction_IsPrivate(asIScriptFunction *f);
bool asFunction_IsProtected(asIScriptFunction *f);
bool asFunction_IsFinal(asIScriptFunction *f);
bool asFunction_IsOverride(asIScriptFunction *f);
bool asFunction_IsShared(asIScriptFunction *f);
bool asFunction_IsExplicit(asIScriptFunction *f);
bool asFunction_IsProperty(asIScriptFunction *f);

// Parameters
asUINT asFunction_GetParamCount(asIScriptFunction *f);
int asFunction_GetParam(asIScriptFunction *f, asUINT index, int *typeId, asDWORD *flags, const char **name, const char **defaultArg);

// Return type
int asFunction_GetReturnTypeId(asIScriptFunction *f, asDWORD *flags);

// Type id for function pointers
int asFunction_GetTypeId(asIScriptFunction *f);
bool asFunction_IsCompatibleWithTypeId(asIScriptFunction *f, int typeId);

// Delegates
void* asFunction_GetDelegateObject(asIScriptFunction *f);
asITypeInfo* asFunction_GetDelegateObjectType(asIScriptFunction *f);
asIScriptFunction* asFunction_GetDelegateFunction(asIScriptFunction *f);

// Debug info
asUINT asFunction_GetVarCount(asIScriptFunction *f);
int asFunction_GetVar(asIScriptFunction *f, asUINT index, const char **name, int *typeId);
const char* asFunction_GetVarDecl(asIScriptFunction *f, asUINT index, bool includeNamespace);
int asFunction_FindNextLineWithCode(asIScriptFunction *f, int line);

// For JIT compilation
asDWORD* asFunction_GetByteCode(asIScriptFunction *f, asUINT *length);

// User data
void* asFunction_GetUserData(asIScriptFunction *f, asPWORD type);
void* asFunction_SetUserData(asIScriptFunction *f, void *data, asPWORD type);

#ifdef __cplusplus
}
#endif

#endif // AS_FUNCTION_H
