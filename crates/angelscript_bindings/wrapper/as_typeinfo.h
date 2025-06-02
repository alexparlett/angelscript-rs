#ifndef AS_TYPEINFO_H
#define AS_TYPEINFO_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif
#include "as_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Type info management
asIScriptEngine* asTypeInfo_GetEngine(asITypeInfo *ti);
const char* asTypeInfo_GetConfigGroup(asITypeInfo *ti);
asDWORD asTypeInfo_GetAccessMask(asITypeInfo *ti);
asIScriptModule* asTypeInfo_GetModule(asITypeInfo *ti);
int asTypeInfo_AddRef(asITypeInfo *ti);
int asTypeInfo_Release(asITypeInfo *ti);

// Type info
const char* asTypeInfo_GetName(asITypeInfo *ti);
const char* asTypeInfo_GetNamespace(asITypeInfo *ti);
asITypeInfo* asTypeInfo_GetBaseType(asITypeInfo *ti);
asBOOL asTypeInfo_DerivesFrom(asITypeInfo *ti, const asITypeInfo *objType);
asDWORD asTypeInfo_GetFlags(asITypeInfo *ti);
asUINT asTypeInfo_GetSize(asITypeInfo *ti);
int asTypeInfo_GetTypeId(asITypeInfo *ti);
int asTypeInfo_GetSubTypeId(asITypeInfo *ti, asUINT subTypeIndex);
asITypeInfo* asTypeInfo_GetSubType(asITypeInfo *ti, asUINT subTypeIndex);
asUINT asTypeInfo_GetSubTypeCount(asITypeInfo *ti);

// Interfaces
asUINT asTypeInfo_GetInterfaceCount(asITypeInfo *ti);
asITypeInfo* asTypeInfo_GetInterface(asITypeInfo *ti, asUINT index);
asBOOL asTypeInfo_Implements(asITypeInfo *ti, const asITypeInfo *objType);

// Factories
asUINT asTypeInfo_GetFactoryCount(asITypeInfo *ti);
asIScriptFunction* asTypeInfo_GetFactoryByIndex(asITypeInfo *ti, asUINT index);
asIScriptFunction* asTypeInfo_GetFactoryByDecl(asITypeInfo *ti, const char *decl);

// Methods
asUINT asTypeInfo_GetMethodCount(asITypeInfo *ti);
asIScriptFunction* asTypeInfo_GetMethodByIndex(asITypeInfo *ti, asUINT index, asBOOL getVirtual);
asIScriptFunction* asTypeInfo_GetMethodByName(asITypeInfo *ti, const char *name, asBOOL getVirtual);
asIScriptFunction* asTypeInfo_GetMethodByDecl(asITypeInfo *ti, const char *decl, asBOOL getVirtual);

// Properties
asUINT asTypeInfo_GetPropertyCount(asITypeInfo *ti);
int asTypeInfo_GetProperty(asITypeInfo *ti, asUINT index, const char **name, int *typeId, asBOOL *isPrivate, asBOOL *isProtected, int *offset, asBOOL *isReference, asDWORD *accessMask, int *compositeOffset, asBOOL *isCompositeIndirect);
const char* asTypeInfo_GetPropertyDeclaration(asITypeInfo *ti, asUINT index, asBOOL includeNamespace);

// Behaviours
asUINT asTypeInfo_GetBehaviourCount(asITypeInfo *ti);
asIScriptFunction* asTypeInfo_GetBehaviourByIndex(asITypeInfo *ti, asUINT index, asEBehaviours *outBehaviour);

// Child types
asUINT asTypeInfo_GetChildFuncdefCount(asITypeInfo *ti);
asITypeInfo* asTypeInfo_GetChildFuncdef(asITypeInfo *ti, asUINT index);
asITypeInfo* asTypeInfo_GetParentType(asITypeInfo *ti);

// Enums
asUINT asTypeInfo_GetEnumValueCount(asITypeInfo *ti);
const char* asTypeInfo_GetEnumValueByIndex(asITypeInfo *ti, asUINT index, int *outValue);

// Typedef
int asTypeInfo_GetTypedefTypeId(asITypeInfo *ti);

// Funcdef
asIScriptFunction* asTypeInfo_GetFuncdefSignature(asITypeInfo *ti);

// User data
void* asTypeInfo_GetUserData(asITypeInfo *ti, asPWORD type);
void* asTypeInfo_SetUserData(asITypeInfo *ti, void *data, asPWORD type);

#ifdef __cplusplus
}
#endif

#endif // AS_TYPEINFO_H
