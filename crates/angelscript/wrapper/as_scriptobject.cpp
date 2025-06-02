#include "as_scriptobject.h"

extern "C" {

// Object management
asIScriptEngine* asScriptObject_GetEngine(asIScriptObject *s) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->GetEngine();
}

int asScriptObject_AddRef(asIScriptObject *s) {
    if (!s) return asINVALID_ARG;
    return static_cast<::asIScriptObject*>(s)->AddRef();
}

int asScriptObject_Release(asIScriptObject *s) {
    if (!s) return asINVALID_ARG;
    return static_cast<::asIScriptObject*>(s)->Release();
}

asILockableSharedBool* asScriptObject_GetWeakRefFlag(asIScriptObject *s) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->GetWeakRefFlag();
}

// Type info
asITypeInfo* asScriptObject_GetObjectType(asIScriptObject *s) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->GetObjectType();
}

// Properties
asUINT asScriptObject_GetPropertyCount(asIScriptObject *s) {
    if (!s) return 0;
    return static_cast<::asIScriptObject*>(s)->GetPropertyCount();
}

int asScriptObject_GetPropertyTypeId(asIScriptObject *s, asUINT prop) {
    if (!s) return 0;
    return static_cast<::asIScriptObject*>(s)->GetPropertyTypeId(prop);
}

const char* asScriptObject_GetPropertyName(asIScriptObject *s, asUINT prop) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->GetPropertyName(prop);
}

void* asScriptObject_GetAddressOfProperty(asIScriptObject *s, asUINT prop) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->GetAddressOfProperty(prop);
}

// Object copying
int asScriptObject_CopyFrom(asIScriptObject *s, const asIScriptObject *other) {
    if (!s || !other) return asINVALID_ARG;
    return static_cast<::asIScriptObject*>(s)->CopyFrom(static_cast<const ::asIScriptObject*>(other));
}

// User data
void* asScriptObject_GetUserData(asIScriptObject *s, asPWORD type) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->GetUserData(type);
}

void* asScriptObject_SetUserData(asIScriptObject *s, void *data, asPWORD type) {
    if (!s) return nullptr;
    return static_cast<::asIScriptObject*>(s)->SetUserData(data, type);
}

} // extern "C"
