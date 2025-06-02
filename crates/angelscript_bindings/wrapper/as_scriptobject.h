#ifndef AS_SCRIPTOBJECT_H
#define AS_SCRIPTOBJECT_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif
#include "as_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Object management
asIScriptEngine* asScriptObject_GetEngine(asIScriptObject *s);
int asScriptObject_AddRef(asIScriptObject *s);
int asScriptObject_Release(asIScriptObject *s);
asILockableSharedBool* asScriptObject_GetWeakRefFlag(asIScriptObject *s);

// Type info
asITypeInfo* asScriptObject_GetObjectType(asIScriptObject *s);

// Properties
asUINT asScriptObject_GetPropertyCount(asIScriptObject *s);
int asScriptObject_GetPropertyTypeId(asIScriptObject *s, asUINT prop);
const char* asScriptObject_GetPropertyName(asIScriptObject *s, asUINT prop);
void* asScriptObject_GetAddressOfProperty(asIScriptObject *s, asUINT prop);

// Object copying
int asScriptObject_CopyFrom(asIScriptObject *s, const asIScriptObject *other);

// User data
void* asScriptObject_GetUserData(asIScriptObject *s, asPWORD type);
void* asScriptObject_SetUserData(asIScriptObject *s, void *data, asPWORD type);

#ifdef __cplusplus
}
#endif

#endif // AS_SCRIPTOBJECT_H
