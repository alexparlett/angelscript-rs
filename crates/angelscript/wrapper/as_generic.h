#ifndef AS_GENERIC_H
#define AS_GENERIC_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

// Argument getters
unsigned int asIScriptGeneric_GetArgDWord(asIScriptGeneric* g, asUINT idx);
asQWORD      asIScriptGeneric_GetArgQWord(asIScriptGeneric* g, asUINT idx);
float         asIScriptGeneric_GetArgFloat(asIScriptGeneric* g, asUINT idx);
double       asIScriptGeneric_GetArgDouble(asIScriptGeneric* g, asUINT idx);
void*        asIScriptGeneric_GetArgAddress(asIScriptGeneric* g, asUINT idx);
void*        asIScriptGeneric_GetArgObject(asIScriptGeneric* g, asUINT idx);
const char*  asIScriptGeneric_GetArgString(asIScriptGeneric* g, asUINT idx);

// Utils getters
asIScriptFunction* asIScriptGeneric_GetFunction(asIScriptGeneric* g);
asIScriptEngine*   asIScriptGeneric_GetEngine(asIScriptGeneric* g); // often useful for user

// Return value setters
void asIScriptGeneric_SetReturnDWord(asIScriptGeneric* g, asUINT val);
void asIScriptGeneric_SetReturnQWord(asIScriptGeneric* g, asQWORD val);
void asIScriptGeneric_SetReturnFloat(asIScriptGeneric* g, float val);
void asIScriptGeneric_SetReturnDouble(asIScriptGeneric* g, double val);
void asIScriptGeneric_SetReturnAddress(asIScriptGeneric* g, void* addr);
void asIScriptGeneric_SetReturnObject(asIScriptGeneric* g, void* obj);

// Misc helpers
void* asIScriptGeneric_GetObject(asIScriptGeneric* g);
int   asIScriptGeneric_GetObjectTypeId(asIScriptGeneric* g);
int   asIScriptGeneric_GetArgTypeId(asIScriptGeneric* g, asUINT idx);

#ifdef __cplusplus
}
#endif

#endif // AS_GENERIC_H