#ifndef AS_CONTEXT_H
#define AS_CONTEXT_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif
#include "as_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Context management
asIScriptEngine* asContext_GetEngine(asIScriptContext *c);
int asContext_AddRef(asIScriptContext *c);
int asContext_Release(asIScriptContext *c);

// Execution
asEContextState asContext_GetState(asIScriptContext *c);
int asContext_Prepare(asIScriptContext *c, asIScriptFunction *func);
int asContext_Unprepare(asIScriptContext *c);
int asContext_Execute(asIScriptContext *c);
int asContext_Abort(asIScriptContext *c);
int asContext_Suspend(asIScriptContext *c);
asEContextState asContext_GetStateOfExecution(asIScriptContext *c);

// State management
int asContext_PushState(asIScriptContext *c);
int asContext_PopState(asIScriptContext *c);
asBOOL asContext_IsNested(asIScriptContext *c, asUINT *nestCount);

// Object pointer for calling class methods
int asContext_SetObject(asIScriptContext *c, void *obj);

// Arguments
int asContext_SetArgByte(asIScriptContext *c, asUINT arg, asBYTE value);
int asContext_SetArgWord(asIScriptContext *c, asUINT arg, asWORD value);
int asContext_SetArgDWord(asIScriptContext *c, asUINT arg, asDWORD value);
int asContext_SetArgQWord(asIScriptContext *c, asUINT arg, asQWORD value);
int asContext_SetArgFloat(asIScriptContext *c, asUINT arg, float value);
int asContext_SetArgDouble(asIScriptContext *c, asUINT arg, double value);
int asContext_SetArgAddress(asIScriptContext *c, asUINT arg, void *addr);
int asContext_SetArgObject(asIScriptContext *c, asUINT arg, void *obj);
int asContext_SetArgVarType(asIScriptContext *c, asUINT arg, void *ptr, int typeId);
void* asContext_GetAddressOfArg(asIScriptContext *c, asUINT arg);

// Return value
asBYTE asContext_GetReturnByte(asIScriptContext *c);
asWORD asContext_GetReturnWord(asIScriptContext *c);
asDWORD asContext_GetReturnDWord(asIScriptContext *c);
asQWORD asContext_GetReturnQWord(asIScriptContext *c);
float asContext_GetReturnFloat(asIScriptContext *c);
double asContext_GetReturnDouble(asIScriptContext *c);
void* asContext_GetReturnAddress(asIScriptContext *c);
void* asContext_GetReturnObject(asIScriptContext *c);
void* asContext_GetAddressOfReturnValue(asIScriptContext *c);

// Exception handling
int asContext_SetException(asIScriptContext *c, const char *string);
int asContext_GetExceptionLineNumber(asIScriptContext *c, int *column, const char **sectionName);
asIScriptFunction* asContext_GetExceptionFunction(asIScriptContext *c);
const char* asContext_GetExceptionString(asIScriptContext *c);
int asContext_SetExceptionCallback(asIScriptContext *c, asFUNCTION_t callback, void *obj, int callConv);
void asContext_ClearExceptionCallback(asIScriptContext *c);

// Line callback
int asContext_SetLineCallback(asIScriptContext *c, asFUNCTION_t callback, void *obj, int callConv);
void asContext_ClearLineCallback(asIScriptContext *c);

// Debugging
asUINT asContext_GetCallstackSize(asIScriptContext *c);
asIScriptFunction* asContext_GetFunction(asIScriptContext *c, asUINT stackLevel);
int asContext_GetLineNumber(asIScriptContext *c, asUINT stackLevel, int *column, const char **sectionName);

// Variables
int asContext_GetVarCount(asIScriptContext *c, asUINT stackLevel);
const char* asContext_GetVarDeclaration(asIScriptContext *c, asUINT varIndex, asUINT stackLevel, asBOOL includeNamespace);
void* asContext_GetAddressOfVar(asIScriptContext *c, asUINT varIndex, asUINT stackLevel);
asBOOL asContext_IsVarInScope(asIScriptContext *c, asUINT varIndex, asUINT stackLevel);

// This pointer
int asContext_GetThisTypeId(asIScriptContext *c, asUINT stackLevel);
void* asContext_GetThisPointer(asIScriptContext *c, asUINT stackLevel);

// System function
asIScriptFunction* asContext_GetSystemFunction(asIScriptContext *c);

// User data
void* asContext_GetUserData(asIScriptContext *c, asPWORD type);
void* asContext_SetUserData(asIScriptContext *c, void *data, asPWORD type);

#ifdef __cplusplus
}
#endif

#endif // AS_CONTEXT_H
