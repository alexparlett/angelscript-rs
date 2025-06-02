#include "as_engine.h"

extern "C" {

// Engine reference counting
void asEngine_AddRef(asIScriptEngine *engine) {
    if (engine) {
        static_cast<::asIScriptEngine*>(engine)->AddRef();
    }
}

void asEngine_Release(asIScriptEngine *engine) {
    if (engine) {
        static_cast<::asIScriptEngine*>(engine)->Release();
    }
}

void asEngine_ShutDownAndRelease(asIScriptEngine *engine) {
    if (engine) {
        static_cast<::asIScriptEngine*>(engine)->ShutDownAndRelease();
    }
}

// Engine properties
int asEngine_SetEngineProperty(asIScriptEngine *engine, asEEngineProp property, asPWORD value) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->SetEngineProperty(static_cast<::asEEngineProp>(property), value);
}

asPWORD asEngine_GetEngineProperty(asIScriptEngine *engine, asEEngineProp property) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetEngineProperty(static_cast<::asEEngineProp>(property));
}

// Message callback
int asEngine_SetMessageCallback(asIScriptEngine *engine, asFUNCTION_t callback, void *obj, asDWORD callConv) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->SetMessageCallback(asFUNCTION(callback), obj, callConv);
}

int asEngine_ClearMessageCallback(asIScriptEngine *engine) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->ClearMessageCallback();
}

int asEngine_WriteMessage(asIScriptEngine *engine, const char *section, int row, int col, int type, const char *message) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->WriteMessage(section, row, col, static_cast<::asEMsgType>(type), message);
}

// JIT Compiler
asIJITCompiler* asEngine_GetJITCompiler(asIScriptEngine *engine) {
    if (!engine) return nullptr;
    return reinterpret_cast<asIJITCompiler*>(static_cast<::asIScriptEngine*>(engine)->GetJITCompiler());
}

int asEngine_SetJITCompiler(asIScriptEngine *engine, asIJITCompiler *compiler) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->SetJITCompiler(reinterpret_cast<::asIJITCompilerAbstract*>(compiler));
}

// Global functions
int asEngine_RegisterGlobalFunction(asIScriptEngine *engine, const char *declaration, asGENFUNC_t funcPointer, asDWORD callConv) {
    if (!engine || !declaration || !funcPointer) return asINVALID_ARG;
    // C++: cast back to the right type for asFUNCTION
    return engine->RegisterGlobalFunction(
        declaration,
        asFUNCTION(funcPointer),
        callConv
    );
}

asUINT asEngine_GetGlobalFunctionCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetGlobalFunctionCount();
}

asIScriptFunction* asEngine_GetGlobalFunctionByIndex(asIScriptEngine *engine, asUINT index) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetGlobalFunctionByIndex(index);
}

asIScriptFunction* asEngine_GetGlobalFunctionByDecl(asIScriptEngine *engine, const char *decl) {
    if (!engine || !decl) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetGlobalFunctionByDecl(decl);
}

// Global properties
int asEngine_RegisterGlobalProperty(asIScriptEngine *engine, const char *declaration, void *pointer) {
    if (!engine || !declaration || !pointer) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterGlobalProperty(declaration, pointer);
}

asUINT asEngine_GetGlobalPropertyCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetGlobalPropertyCount();
}

int asEngine_GetGlobalPropertyByIndex(asIScriptEngine *engine, asUINT index, const char **name, const char **nameSpace, int *typeId, asBOOL *isConst, const char **configGroup, void **pointer, asDWORD *accessMask) {
    if (!engine) return asINVALID_ARG;
    bool constFlag;
    int result = static_cast<::asIScriptEngine*>(engine)->GetGlobalPropertyByIndex(index, name, nameSpace, typeId, &constFlag, configGroup, pointer, accessMask);
    if (isConst) *isConst = constFlag ? asTRUE : asFALSE;
    return result;
}

int asEngine_GetGlobalPropertyIndexByName(asIScriptEngine *engine, const char *name) {
    if (!engine || !name) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetGlobalPropertyIndexByName(name);
}

int asEngine_GetGlobalPropertyIndexByDecl(asIScriptEngine *engine, const char *decl) {
    if (!engine || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetGlobalPropertyIndexByDecl(decl);
}

// Object types
int asEngine_RegisterObjectType(asIScriptEngine *engine, const char *name, int byteSize, asDWORD flags) {
    if (!engine || !name) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterObjectType(name, byteSize, flags);
}

int asEngine_RegisterObjectProperty(asIScriptEngine *engine, const char *obj, const char *declaration, int byteOffset) {
    if (!engine || !obj || !declaration) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterObjectProperty(obj, declaration, byteOffset);
}

int asEngine_RegisterObjectMethod(asIScriptEngine *engine, const char *obj, const char *declaration, asGENFUNC_t funcPointer, asDWORD callConv) {
    if (!engine || !obj || !declaration || !funcPointer) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterObjectMethod(obj, declaration, asFUNCTION(funcPointer), callConv);
}

int asEngine_RegisterObjectBehaviour(asIScriptEngine *engine, const char *obj, asEBehaviours behaviour, const char *declaration, asGENFUNC_t funcPointer, asDWORD callConv) {
    if (!engine || !obj || !declaration || !funcPointer) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterObjectBehaviour(obj, static_cast<::asEBehaviours>(behaviour), declaration, asFUNCTION(funcPointer), callConv);
}

// Interfaces
int asEngine_RegisterInterface(asIScriptEngine *engine, const char *name) {
    if (!engine || !name) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterInterface(name);
}

int asEngine_RegisterInterfaceMethod(asIScriptEngine *engine, const char *intf, const char *declaration) {
    if (!engine || !intf || !declaration) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterInterfaceMethod(intf, declaration);
}

int asEngine_GetStringFactoryReturnTypeId(asIScriptEngine *engine, asDWORD *flags) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetStringFactoryReturnTypeId(flags);
}

// Default array type
int asEngine_RegisterStringFactory(asIScriptEngine *engine, const char *datatype, asIStringFactory *factory) {
    if (!engine || !datatype || !factory) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterStringFactory(datatype, factory);
}

// Default array type
int asEngine_RegisterDefaultArrayType(asIScriptEngine *engine, const char *type) {
    if (!engine || !type) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterDefaultArrayType(type);
}

int asEngine_GetDefaultArrayTypeId(asIScriptEngine *engine) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetDefaultArrayTypeId();
}

// Enums
int asEngine_RegisterEnum(asIScriptEngine *engine, const char *type) {
    if (!engine || !type) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterEnum(type);
}

int asEngine_RegisterEnumValue(asIScriptEngine *engine, const char *type, const char *name, int value) {
    if (!engine || !type || !name) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterEnumValue(type, name, value);
}

asUINT asEngine_GetEnumCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetEnumCount();
}

asITypeInfo* asEngine_GetEnumByIndex(asIScriptEngine *engine, asUINT index) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetEnumByIndex(index);
}

// Funcdefs
int asEngine_RegisterFuncdef(asIScriptEngine *engine, const char *decl) {
    if (!engine || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterFuncdef(decl);
}

asUINT asEngine_GetFuncdefCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetFuncdefCount();
}

asITypeInfo* asEngine_GetFuncdefByIndex(asIScriptEngine *engine, asUINT index) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetFuncdefByIndex(index);
}

// Typedefs
int asEngine_RegisterTypedef(asIScriptEngine *engine, const char *type, const char *decl) {
    if (!engine || !type || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RegisterTypedef(type, decl);
}

asUINT asEngine_GetTypedefCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetTypedefCount();
}

asITypeInfo* asEngine_GetTypedefByIndex(asIScriptEngine *engine, asUINT index) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetTypedefByIndex(index);
}

// Configuration groups
int asEngine_BeginConfigGroup(asIScriptEngine *engine, const char *groupName) {
    if (!engine || !groupName) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->BeginConfigGroup(groupName);
}

int asEngine_EndConfigGroup(asIScriptEngine *engine) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->EndConfigGroup();
}

int asEngine_RemoveConfigGroup(asIScriptEngine *engine, const char *groupName) {
    if (!engine || !groupName) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->RemoveConfigGroup(groupName);
}

asDWORD asEngine_SetDefaultAccessMask(asIScriptEngine *engine, asDWORD defaultMask) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->SetDefaultAccessMask(defaultMask);
}

int asEngine_SetDefaultNamespace(asIScriptEngine *engine, const char *nameSpace) {
    if (!engine || !nameSpace) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->SetDefaultNamespace(nameSpace);
}

const char* asEngine_GetDefaultNamespace(asIScriptEngine *engine) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetDefaultNamespace();
}

// Modules
asIScriptModule* asEngine_GetModule(asIScriptEngine *engine, const char *module, asEGMFlags flag) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetModule(module, static_cast<::asEGMFlags>(flag));
}

int asEngine_DiscardModule(asIScriptEngine *engine, const char *module) {
    if (!engine || !module) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->DiscardModule(module);
}

asUINT asEngine_GetModuleCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetModuleCount();
}

asIScriptModule* asEngine_GetModuleByIndex(asIScriptEngine *engine, asUINT index) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetModuleByIndex(index);
}

// Script object management
asIScriptContext* asEngine_CreateContext(asIScriptEngine *engine) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->CreateContext();
}

void* asEngine_CreateScriptObject(asIScriptEngine *engine, const asITypeInfo *type) {
    if (!engine || !type) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->CreateScriptObject(static_cast<const ::asITypeInfo*>(type));
}

void* asEngine_CreateScriptObjectCopy(asIScriptEngine *engine, void *obj, const asITypeInfo *type) {
    if (!engine || !obj || !type) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->CreateScriptObjectCopy(obj, static_cast<const ::asITypeInfo*>(type));
}

void* asEngine_CreateUninitializedScriptObject(asIScriptEngine *engine, const asITypeInfo *type) {
    if (!engine || !type) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->CreateUninitializedScriptObject(static_cast<const ::asITypeInfo*>(type));
}

asIScriptFunction* asEngine_CreateDelegate(asIScriptEngine *engine, asIScriptFunction *func, void *obj) {
    if (!engine || !func || !obj) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->CreateDelegate(static_cast<::asIScriptFunction*>(func), obj);
}

int asEngine_AssignScriptObject(asIScriptEngine *engine, void *dstObj, void *srcObj, const asITypeInfo *type) {
    if (!engine || !dstObj || !srcObj || !type) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->AssignScriptObject(dstObj, srcObj, static_cast<const ::asITypeInfo*>(type));
}

void asEngine_ReleaseScriptObject(asIScriptEngine *engine, void *obj, const asITypeInfo *type) {
    if (!engine || !obj || !type) return;
    static_cast<::asIScriptEngine*>(engine)->ReleaseScriptObject(obj, static_cast<const ::asITypeInfo*>(type));
}

void asEngine_AddRefScriptObject(asIScriptEngine *engine, void *obj, const asITypeInfo *type) {
    if (!engine || !obj || !type) return;
    static_cast<::asIScriptEngine*>(engine)->AddRefScriptObject(obj, static_cast<const ::asITypeInfo*>(type));
}

int asEngine_RefCastObject(asIScriptEngine *engine, void *obj, asITypeInfo *fromType, asITypeInfo *toType, void **newPtr, asBOOL useOnlyImplicitCast) {
    if (!engine || !obj || !fromType || !toType || !newPtr) return asINVALID_ARG;
    bool implicitCast = useOnlyImplicitCast ? true : false;
    return static_cast<::asIScriptEngine*>(engine)->RefCastObject(obj, static_cast<::asITypeInfo*>(fromType), static_cast<::asITypeInfo*>(toType), newPtr, implicitCast);
}

asILockableSharedBool* asEngine_GetWeakRefFlagOfScriptObject(asIScriptEngine *engine, void *obj, const asITypeInfo *type) {
    if (!engine || !obj || !type) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetWeakRefFlagOfScriptObject(obj, static_cast<const ::asITypeInfo*>(type));
}

// Context pooling
asIScriptContext* asEngine_RequestContext(asIScriptEngine *engine) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->RequestContext();
}

void asEngine_ReturnContext(asIScriptEngine *engine, asIScriptContext *ctx) {
    if (!engine || !ctx) return;
    static_cast<::asIScriptEngine*>(engine)->ReturnContext(static_cast<::asIScriptContext*>(ctx));
}

int asEngine_SetContextCallbacks(asIScriptEngine *engine, asREQUESTCONTEXTFUNC_t requestCtx, asRETURNCONTEXTFUNC_t returnCtx, void *param) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->SetContextCallbacks(
        reinterpret_cast<::asREQUESTCONTEXTFUNC_t>(requestCtx),
        reinterpret_cast<::asRETURNCONTEXTFUNC_t>(returnCtx),
        param
    );
}

// Garbage collection
int asEngine_GarbageCollect(asIScriptEngine *engine, asDWORD flags) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GarbageCollect(flags);
}

void asEngine_GetGCStatistics(asIScriptEngine *engine, asUINT *currentSize, asUINT *totalDestroyed, asUINT *totalDetected, asUINT *newObjects, asUINT *totalNewDestroyed) {
    if (!engine) return;
    static_cast<::asIScriptEngine*>(engine)->GetGCStatistics(currentSize, totalDestroyed, totalDetected, newObjects, totalNewDestroyed);
}

int asEngine_NotifyGarbageCollectorOfNewObject(asIScriptEngine *engine, void *obj, asITypeInfo *type) {
    if (!engine || !obj || !type) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->NotifyGarbageCollectorOfNewObject(obj, static_cast<::asITypeInfo*>(type));
}

int asEngine_GetObjectInGC(asIScriptEngine *engine, asUINT idx, asUINT *seqNbr, void **obj, asITypeInfo **type) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetObjectInGC(idx, seqNbr, obj, reinterpret_cast<::asITypeInfo**>(type));
}

void asEngine_GCEnumCallback(asIScriptEngine *engine, void *reference) {
    if (!engine || !reference) return;
    static_cast<::asIScriptEngine*>(engine)->GCEnumCallback(reference);
}

void asEngine_ForwardGCEnumReferences(asIScriptEngine *engine, void *ref, asITypeInfo *type) {
    if (!engine || !ref || !type) return;
    static_cast<::asIScriptEngine*>(engine)->ForwardGCEnumReferences(ref, static_cast<::asITypeInfo*>(type));
}

void asEngine_ForwardGCReleaseReferences(asIScriptEngine *engine, void *ref, asITypeInfo *type) {
    if (!engine || !ref || !type) return;
    static_cast<::asIScriptEngine*>(engine)->ForwardGCReleaseReferences(ref, static_cast<::asITypeInfo*>(type));
}

void asEngine_SetCircularRefDetectedCallback(asIScriptEngine *engine, asCIRCULARREFFUNC_t callback, void *param) {
    if (!engine) return;
    static_cast<::asIScriptEngine*>(engine)->SetCircularRefDetectedCallback(
        reinterpret_cast<::asCIRCULARREFFUNC_t>(callback),
        param
    );
}

// Type identification
asITypeInfo* asEngine_GetTypeInfoByName(asIScriptEngine *engine, const char *name) {
    if (!engine || !name) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetTypeInfoByName(name);
}

asITypeInfo* asEngine_GetTypeInfoByDecl(asIScriptEngine *engine, const char *decl) {
    if (!engine || !decl) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetTypeInfoByDecl(decl);
}

int asEngine_GetTypeIdByDecl(asIScriptEngine *engine, const char *decl) {
    if (!engine || !decl) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetTypeIdByDecl(decl);
}

const char* asEngine_GetTypeDeclaration(asIScriptEngine *engine, int typeId, asBOOL includeNamespace) {
    if (!engine) return nullptr;
    bool includeNs = includeNamespace ? true : false;
    return static_cast<::asIScriptEngine*>(engine)->GetTypeDeclaration(typeId, includeNs);
}

int asEngine_GetSizeOfPrimitiveType(asIScriptEngine *engine, int typeId) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetSizeOfPrimitiveType(typeId);
}

asITypeInfo* asEngine_GetTypeInfoById(asIScriptEngine *engine, int typeId) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetTypeInfoById(typeId);
}

asUINT asEngine_GetObjectTypeCount(asIScriptEngine *engine) {
    if (!engine) return 0;
    return static_cast<::asIScriptEngine*>(engine)->GetObjectTypeCount();
}

asITypeInfo* asEngine_GetObjectTypeByIndex(asIScriptEngine *engine, asUINT index) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetObjectTypeByIndex(index);
}

// User data
void* asEngine_GetUserData(asIScriptEngine *engine, asPWORD type) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetUserData(type);
}

void* asEngine_SetUserData(asIScriptEngine *engine, void *data, asPWORD type) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->SetUserData(data, type);
}

int asEngine_GetLastFunctionId(asIScriptEngine *engine) {
    if (!engine) return asINVALID_ARG;
    return static_cast<::asIScriptEngine*>(engine)->GetLastFunctionId();
}

asIScriptFunction* asEngine_GetFunctionById(asIScriptEngine *engine, int funcId) {
    if (!engine) return nullptr;
    return static_cast<::asIScriptEngine*>(engine)->GetFunctionById(funcId);
}

} // extern "C"
