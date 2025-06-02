#include "as_context.h"

extern "C" {

// Context management
asIScriptEngine* asContext_GetEngine(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetEngine();
}

int asContext_AddRef(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->AddRef();
}

int asContext_Release(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Release();
}

// Execution
asEContextState asContext_GetState(asIScriptContext *c) {
    if (!c) return asEXECUTION_ERROR;
    return static_cast<asEContextState>(static_cast<::asIScriptContext*>(c)->GetState());
}

int asContext_Prepare(asIScriptContext *c, asIScriptFunction *func) {
    if (!c || !func) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Prepare(static_cast<::asIScriptFunction*>(func));
}

int asContext_Unprepare(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Unprepare();
}

int asContext_Execute(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Execute();
}

int asContext_Abort(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Abort();
}

int asContext_Suspend(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Suspend();
}

asEContextState asContext_GetStateOfExecution(asIScriptContext *c) {
    if (!c) return asEXECUTION_ERROR;
    return static_cast<asEContextState>(static_cast<::asIScriptContext*>(c)->GetState());
}

// State management
int asContext_PushState(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->PushState();
}

int asContext_PopState(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->PopState();
}

asBOOL asContext_IsNested(asIScriptContext *c, asUINT *nestCount) {
    if (!c || !nestCount) return asFALSE;
    asUINT count = 0;
    bool isNested = static_cast<::asIScriptContext*>(c)->IsNested(&count);
    *nestCount = count;
    return isNested ? asTRUE : asFALSE;
}

// Object pointer for calling class methods
int asContext_SetObject(asIScriptContext *c, void *obj) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetObject(obj);
}

// Arguments
int asContext_SetArgByte(asIScriptContext *c, asUINT arg, asBYTE value) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgByte(arg, value);
}
int asContext_SetArgWord(asIScriptContext *c, asUINT arg, asWORD value) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgWord(arg, value);
}
int asContext_SetArgDWord(asIScriptContext *c, asUINT arg, asDWORD value) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgDWord(arg, value);
}
int asContext_SetArgQWord(asIScriptContext *c, asUINT arg, asQWORD value) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgQWord(arg, value);
}
int asContext_SetArgFloat(asIScriptContext *c, asUINT arg, float value) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgFloat(arg, value);
}
int asContext_SetArgDouble(asIScriptContext *c, asUINT arg, double value) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgDouble(arg, value);
}
int asContext_SetArgAddress(asIScriptContext *c, asUINT arg, void *addr) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgAddress(arg, addr);
}
int asContext_SetArgObject(asIScriptContext *c, asUINT arg, void *obj) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgObject(arg, obj);
}
int asContext_SetArgVarType(asIScriptContext *c, asUINT arg, void *ptr, int typeId) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetArgVarType(arg, ptr, typeId);
}
void* asContext_GetAddressOfArg(asIScriptContext *c, asUINT arg) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetAddressOfArg(arg);
}

// Return value
asBYTE asContext_GetReturnByte(asIScriptContext *c) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetReturnByte();
}
asWORD asContext_GetReturnWord(asIScriptContext *c) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetReturnWord();
}
asDWORD asContext_GetReturnDWord(asIScriptContext *c) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetReturnDWord();
}
asQWORD asContext_GetReturnQWord(asIScriptContext *c) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetReturnQWord();
}
float asContext_GetReturnFloat(asIScriptContext *c) {
    if (!c) return 0.0f;
    return static_cast<::asIScriptContext*>(c)->GetReturnFloat();
}
double asContext_GetReturnDouble(asIScriptContext *c) {
    if (!c) return 0.0;
    return static_cast<::asIScriptContext*>(c)->GetReturnDouble();
}
void* asContext_GetReturnAddress(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetReturnAddress();
}
void* asContext_GetReturnObject(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetReturnObject();
}
void* asContext_GetAddressOfReturnValue(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetAddressOfReturnValue();
}

// Exception handling
int asContext_SetException(asIScriptContext *c, const char *string) {
    if (!c || !string) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetException(string);
}
int asContext_GetExceptionLineNumber(asIScriptContext *c, int *column, const char **sectionName) {
    if (!c) return 0;
    int col = 0;
    const char *sect = nullptr;
    int line = static_cast<::asIScriptContext*>(c)->GetExceptionLineNumber(&col, &sect);
    if (column) *column = col;
    if (sectionName) *sectionName = sect;
    return line;
}
asIScriptFunction* asContext_GetExceptionFunction(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetExceptionFunction();
}
const char* asContext_GetExceptionString(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetExceptionString();
}
int asContext_SetExceptionCallback(asIScriptContext *c, asFUNCTION_t callback, void *obj, int callConv) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetExceptionCallback(asFUNCTION(callback), obj, callConv);
}
void asContext_ClearExceptionCallback(asIScriptContext *c) {
    if (!c) return;
    static_cast<::asIScriptContext*>(c)->ClearExceptionCallback();
}

// Line callback
int asContext_SetLineCallback(asIScriptContext *c, asFUNCTION_t callback, void *obj, int callConv) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->SetLineCallback(asFUNCTION(callback), obj, callConv);
}

void asContext_ClearLineCallback(asIScriptContext *c) {
    if (!c) return;
    static_cast<::asIScriptContext*>(c)->ClearLineCallback();
}

// Debugging
asUINT asContext_GetCallstackSize(asIScriptContext *c) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetCallstackSize();
}
asIScriptFunction* asContext_GetFunction(asIScriptContext *c, asUINT stackLevel) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetFunction(stackLevel);
}
int asContext_GetLineNumber(asIScriptContext *c, asUINT stackLevel, int *column, const char **sectionName) {
    if (!c) return 0;
    int col = 0;
    const char *sect = nullptr;
    int line = static_cast<::asIScriptContext*>(c)->GetLineNumber(stackLevel, &col, &sect);
    if (column) *column = col;
    if (sectionName) *sectionName = sect;
    return line;
}

// Variables
int asContext_GetVarCount(asIScriptContext *c, asUINT stackLevel) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetVarCount(stackLevel);
}
const char* asContext_GetVarDeclaration(asIScriptContext *c, asUINT varIndex, asUINT stackLevel, asBOOL includeNamespace) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetVarDeclaration(varIndex, stackLevel, includeNamespace != 0);
}
void* asContext_GetAddressOfVar(asIScriptContext *c, asUINT varIndex, asUINT stackLevel) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetAddressOfVar(varIndex, stackLevel);
}
asBOOL asContext_IsVarInScope(asIScriptContext *c, asUINT varIndex, asUINT stackLevel) {
    if (!c) return asFALSE;
    return static_cast<::asIScriptContext*>(c)->IsVarInScope(varIndex, stackLevel) ? asTRUE : asFALSE;
}

// This pointer
int asContext_GetThisTypeId(asIScriptContext *c, asUINT stackLevel) {
    if (!c) return 0;
    return static_cast<::asIScriptContext*>(c)->GetThisTypeId(stackLevel);
}
void* asContext_GetThisPointer(asIScriptContext *c, asUINT stackLevel) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetThisPointer(stackLevel);
}

// System function
asIScriptFunction* asContext_GetSystemFunction(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetSystemFunction();
}

// User data
void* asContext_GetUserData(asIScriptContext *c, asPWORD type) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetUserData(type);
}
void* asContext_SetUserData(asIScriptContext *c, void *data, asPWORD type) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->SetUserData(data, type);
}

} // extern "C"