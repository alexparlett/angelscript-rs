#ifndef ANGELSCRIPT_C_H
#define ANGELSCRIPT_C_H

// Include AngelScript to get all the type definitions
#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*asMESSAGEINFOFUNC_t)(const asSMessageInfo *msg, void *param);
typedef void (*asSCRIPTCONTEXTFUNC_t)(asIScriptContext *msg, void *param);

// Core functions that need C linkage
asSFuncPtr asGenericFunction(asGENFUNC_t func);
asSFuncPtr asFunction(asFUNCTION_t func);
asSFuncPtr asMessageInfoFunction(asMESSAGEINFOFUNC_t func);
asSFuncPtr asScriptContextFunction(asSCRIPTCONTEXTFUNC_t func);

// (const asSMessageInfo *msg, void *param)
#ifdef __cplusplus
}
#endif

#endif // ANGELSCRIPT_C_H
