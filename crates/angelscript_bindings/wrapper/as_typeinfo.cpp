#include "as_typeinfo.h"

extern "C" {

// Type info management
asIScriptEngine* asTypeInfo_GetEngine(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetEngine();
}

const char* asTypeInfo_GetConfigGroup(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetConfigGroup();
}

asDWORD asTypeInfo_GetAccessMask(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetAccessMask();
}

asIScriptModule* asTypeInfo_GetModule(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetModule();
}

int asTypeInfo_AddRef(asITypeInfo *ti) {
    if (!ti) return asINVALID_ARG;
    return static_cast<::asITypeInfo*>(ti)->AddRef();
}

int asTypeInfo_Release(asITypeInfo *ti) {
    if (!ti) return asINVALID_ARG;
    return static_cast<::asITypeInfo*>(ti)->Release();
}

// Type info
const char* asTypeInfo_GetName(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetName();
}

const char* asTypeInfo_GetNamespace(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetNamespace();
}

asITypeInfo* asTypeInfo_GetBaseType(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetBaseType();
}

asBOOL asTypeInfo_DerivesFrom(asITypeInfo *ti, const asITypeInfo *objType) {
    if (!ti || !objType) return asFALSE;
    return static_cast<::asITypeInfo*>(ti)->DerivesFrom(static_cast<const ::asITypeInfo*>(objType)) ? asTRUE : asFALSE;
}

asDWORD asTypeInfo_GetFlags(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetFlags();
}

asUINT asTypeInfo_GetSize(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetSize();
}

int asTypeInfo_GetTypeId(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetTypeId();
}

int asTypeInfo_GetSubTypeId(asITypeInfo *ti, asUINT subTypeIndex) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetSubTypeId(subTypeIndex);
}

asITypeInfo* asTypeInfo_GetSubType(asITypeInfo *ti, asUINT subTypeIndex) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetSubType(subTypeIndex);
}

asUINT asTypeInfo_GetSubTypeCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetSubTypeCount();
}

// Interfaces
asUINT asTypeInfo_GetInterfaceCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetInterfaceCount();
}

asITypeInfo* asTypeInfo_GetInterface(asITypeInfo *ti, asUINT index) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetInterface(index);
}

asBOOL asTypeInfo_Implements(asITypeInfo *ti, const asITypeInfo *objType) {
    if (!ti || !objType) return asFALSE;
    return static_cast<::asITypeInfo*>(ti)->Implements(static_cast<const ::asITypeInfo*>(objType)) ? asTRUE : asFALSE;
}

// Factories
asUINT asTypeInfo_GetFactoryCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetFactoryCount();
}

asIScriptFunction* asTypeInfo_GetFactoryByIndex(asITypeInfo *ti, asUINT index) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetFactoryByIndex(index);
}

asIScriptFunction* asTypeInfo_GetFactoryByDecl(asITypeInfo *ti, const char *decl) {
    if (!ti || !decl) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetFactoryByDecl(decl);
}

// Methods
// Methods
asUINT asTypeInfo_GetMethodCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetMethodCount();
}

asIScriptFunction* asTypeInfo_GetMethodByIndex(asITypeInfo *ti, asUINT index, asBOOL getVirtual) {
    if (!ti) return nullptr;
    bool getVirt = getVirtual ? true : false;
    return static_cast<::asITypeInfo*>(ti)->GetMethodByIndex(index, getVirt);
}

asIScriptFunction* asTypeInfo_GetMethodByName(asITypeInfo *ti, const char *name, asBOOL getVirtual) {
    if (!ti || !name) return nullptr;
    bool getVirt = getVirtual ? true : false;
    return static_cast<::asITypeInfo*>(ti)->GetMethodByName(name, getVirt);
}

asIScriptFunction* asTypeInfo_GetMethodByDecl(asITypeInfo *ti, const char *decl, asBOOL getVirtual) {
    if (!ti || !decl) return nullptr;
    bool getVirt = getVirtual ? true : false;
    return static_cast<::asITypeInfo*>(ti)->GetMethodByDecl(decl, getVirt);
}

// Properties
asUINT asTypeInfo_GetPropertyCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetPropertyCount();
}

int asTypeInfo_GetProperty(asITypeInfo *ti, asUINT index, const char **name, int *typeId, asBOOL *isPrivate, asBOOL *isProtected, int *offset, asBOOL *isReference, asDWORD *accessMask, int *compositeOffset, asBOOL *isCompositeIndirect) {
    if (!ti) return asINVALID_ARG;

    bool privFlag, protFlag, refFlag, compIndFlag;
    int result = static_cast<::asITypeInfo*>(ti)->GetProperty(index, name, typeId, &privFlag, &protFlag, offset, &refFlag, accessMask, compositeOffset, &compIndFlag);

    if (isPrivate) *isPrivate = privFlag ? asTRUE : asFALSE;
    if (isProtected) *isProtected = protFlag ? asTRUE : asFALSE;
    if (isReference) *isReference = refFlag ? asTRUE : asFALSE;
    if (isCompositeIndirect) *isCompositeIndirect = compIndFlag ? asTRUE : asFALSE;

    return result;
}

const char* asTypeInfo_GetPropertyDeclaration(asITypeInfo *ti, asUINT index, asBOOL includeNamespace) {
    if (!ti) return nullptr;
    bool inclNs = includeNamespace ? true : false;
    return static_cast<::asITypeInfo*>(ti)->GetPropertyDeclaration(index, inclNs);
}

// Behaviours
asUINT asTypeInfo_GetBehaviourCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetBehaviourCount();
}

asIScriptFunction* asTypeInfo_GetBehaviourByIndex(asITypeInfo *ti, asUINT index, asEBehaviours *outBehaviour) {
    if (!ti) return nullptr;
    ::asEBehaviours behaviour;
    asIScriptFunction* func = static_cast<::asITypeInfo*>(ti)->GetBehaviourByIndex(index, &behaviour);
    if (outBehaviour) *outBehaviour = static_cast<asEBehaviours>(behaviour);
    return func;
}

// Child types
asUINT asTypeInfo_GetChildFuncdefCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetChildFuncdefCount();
}

asITypeInfo* asTypeInfo_GetChildFuncdef(asITypeInfo *ti, asUINT index) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetChildFuncdef(index);
}

asITypeInfo* asTypeInfo_GetParentType(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetParentType();
}

// Enums
asUINT asTypeInfo_GetEnumValueCount(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetEnumValueCount();
}

const char* asTypeInfo_GetEnumValueByIndex(asITypeInfo *ti, asUINT index, int *outValue) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetEnumValueByIndex(index, outValue);
}

// Typedef
int asTypeInfo_GetTypedefTypeId(asITypeInfo *ti) {
    if (!ti) return 0;
    return static_cast<::asITypeInfo*>(ti)->GetTypedefTypeId();
}

// Funcdef
asIScriptFunction* asTypeInfo_GetFuncdefSignature(asITypeInfo *ti) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetFuncdefSignature();
}

// User data
void* asTypeInfo_GetUserData(asITypeInfo *ti, asPWORD type) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->GetUserData(type);
}

void* asTypeInfo_SetUserData(asITypeInfo *ti, void *data, asPWORD type) {
    if (!ti) return nullptr;
    return static_cast<::asITypeInfo*>(ti)->SetUserData(data, type);
}

} // extern "C"
