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
asWORD       asScriptGeneric_GetArgWord(asIScriptGeneric* g, asUINT idx);
asBYTE       asScriptGeneric_GetArgByte(asIScriptGeneric* g, asUINT idx);
asUINT       asScriptGeneric_GetArgDWord(asIScriptGeneric* g, asUINT idx);
asQWORD      asScriptGeneric_GetArgQWord(asIScriptGeneric* g, asUINT idx);
float         asScriptGeneric_GetArgFloat(asIScriptGeneric* g, asUINT idx);
double       asScriptGeneric_GetArgDouble(asIScriptGeneric* g, asUINT idx);
void*        asScriptGeneric_GetArgAddress(asIScriptGeneric* g, asUINT idx);
void*        asScriptGeneric_GetArgObject(asIScriptGeneric* g, asUINT idx);
void*        asScriptGeneric_GetAddressOfReturnLocation(asIScriptGeneric* g);
void*        asScriptGeneric_GetAddressOfArg(asIScriptGeneric* g, asUINT idx);
// Utils getters
asIScriptFunction* asScriptGeneric_GetFunction(asIScriptGeneric* g);
asIScriptEngine*   asScriptGeneric_GetEngine(asIScriptGeneric* g); // often useful for user

// Return value setters
void asScriptGeneric_SetReturnByte(asIScriptGeneric* g, asBYTE val);
void asScriptGeneric_SetReturnDWord(asIScriptGeneric* g, asUINT val);
void asScriptGeneric_SetReturnQWord(asIScriptGeneric* g, asQWORD val);
void asScriptGeneric_SetReturnFloat(asIScriptGeneric* g, float val);
void asScriptGeneric_SetReturnDouble(asIScriptGeneric* g, double val);
void asScriptGeneric_SetReturnAddress(asIScriptGeneric* g, void* addr);
void asScriptGeneric_SetReturnObject(asIScriptGeneric* g, void* obj);

// Misc helpers
void* asScriptGeneric_GetObject(asIScriptGeneric* g);
int   asScriptGeneric_GetObjectTypeId(asIScriptGeneric* g);
int   asScriptGeneric_GetArgTypeId(asIScriptGeneric* g, asUINT idx);

#ifdef __cplusplus
}
#endif

#endif // AS_GENERIC_H