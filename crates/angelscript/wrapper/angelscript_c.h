#ifndef ANGELSCRIPT_C_H
#define ANGELSCRIPT_C_H

// Include AngelScript to get all the type definitions
#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif

// Include our minimal types
#include "as_types.h"

// Include our function declarations
#include "as_engine.h"
#include "as_module.h"
#include "as_context.h"
#include "as_function.h"
#include "as_typeinfo.h"
#include "as_scriptobject.h"
#include "as_generic.h"
#include "as_stringfactory.h"

#ifdef __cplusplus
extern "C" {
#endif

// Core functions that need C linkage
asIScriptEngine* asCreateScriptEngine(asUINT version);
const char* asGetLibraryVersion();
const char* asGetLibraryOptions();

#ifdef __cplusplus
}
#endif

#endif // ANGELSCRIPT_C_H
