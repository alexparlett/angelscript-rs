#ifndef AS_MODULE_H
#define AS_MODULE_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif
#include "as_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Module management
asIScriptEngine* asModule_GetEngine(asIScriptModule *m);
void asModule_SetName(asIScriptModule *m, const char *name);
const char* asModule_GetName(asIScriptModule *m);
void asModule_Discard(asIScriptModule *m);

// Script sections
int asModule_AddScriptSection(asIScriptModule *m, const char *name, const char *code, size_t codeLength, int lineOffset);

// Build
int asModule_Build(asIScriptModule *m);
int asModule_CompileFunction(asIScriptModule *m, const char *sectionName, const char *code, int lineOffset, asDWORD compileFlags, asIScriptFunction **outFunc);
int asModule_CompileGlobalVar(asIScriptModule *m, const char *sectionName, const char *code, int lineOffset);

// Namespaces
int asModule_SetDefaultNamespace(asIScriptModule *m, const char *nameSpace);
const char* asModule_GetDefaultNamespace(asIScriptModule *m);

// Functions
asUINT asModule_GetFunctionCount(asIScriptModule *m);
asIScriptFunction* asModule_GetFunctionByIndex(asIScriptModule *m, asUINT index);
asIScriptFunction* asModule_GetFunctionByDecl(asIScriptModule *m, const char *decl);
asIScriptFunction* asModule_GetFunctionByName(asIScriptModule *m, const char *name);
int asModule_RemoveFunction(asIScriptModule *m, asIScriptFunction *func);

// Global variables
int asModule_ResetGlobalVars(asIScriptModule *m, asIScriptContext *ctx);
asUINT asModule_GetGlobalVarCount(asIScriptModule *m);
int asModule_GetGlobalVarIndexByName(asIScriptModule *m, const char *name);
int asModule_GetGlobalVarIndexByDecl(asIScriptModule *m, const char *decl);
const char* asModule_GetGlobalVarDeclaration(asIScriptModule *m, asUINT index, bool includeNamespace);
int asModule_GetGlobalVar(asIScriptModule *m, asUINT index, const char **name, const char **nameSpace, int *typeId, bool *isConst);
void* asModule_GetAddressOfGlobalVar(asIScriptModule *m, asUINT index);
int asModule_RemoveGlobalVar(asIScriptModule *m, asUINT index);

// Type identification
asUINT asModule_GetObjectTypeCount(asIScriptModule *m);
asITypeInfo* asModule_GetObjectTypeByIndex(asIScriptModule *m, asUINT index);
int asModule_GetTypeIdByDecl(asIScriptModule *m, const char *decl);
asITypeInfo* asModule_GetTypeInfoByName(asIScriptModule *m, const char *name);
asITypeInfo* asModule_GetTypeInfoByDecl(asIScriptModule *m, const char *decl);

// Enums
asUINT asModule_GetEnumCount(asIScriptModule *m);
asITypeInfo* asModule_GetEnumByIndex(asIScriptModule *m, asUINT index);

// Typedefs
asUINT asModule_GetTypedefCount(asIScriptModule *m);
asITypeInfo* asModule_GetTypedefByIndex(asIScriptModule *m, asUINT index);

// Imports
asUINT asModule_GetImportedFunctionCount(asIScriptModule *m);
int asModule_GetImportedFunctionIndexByDecl(asIScriptModule *m, const char *decl);
const char* asModule_GetImportedFunctionDeclaration(asIScriptModule *m, asUINT importIndex);
const char* asModule_GetImportedFunctionSourceModule(asIScriptModule *m, asUINT importIndex);
int asModule_BindImportedFunction(asIScriptModule *m, asUINT importIndex, asIScriptFunction *func);
int asModule_UnbindImportedFunction(asIScriptModule *m, asUINT importIndex);
int asModule_BindAllImportedFunctions(asIScriptModule *m);
int asModule_UnbindAllImportedFunctions(asIScriptModule *m);

// Bytecode
int asModule_SaveByteCode(asIScriptModule *m, asIBinaryStream *out, bool stripDebugInfo);
int asModule_LoadByteCode(asIScriptModule *m, asIBinaryStream *in, bool *wasDebugInfoStripped);

// User data
void* asModule_GetUserData(asIScriptModule *m, asPWORD type);
void* asModule_SetUserData(asIScriptModule *m, void *data, asPWORD type);

#ifdef __cplusplus
}
#endif

#endif // AS_MODULE_H
