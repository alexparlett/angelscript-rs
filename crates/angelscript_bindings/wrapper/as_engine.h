#ifndef AS_ENGINE_H
#define AS_ENGINE_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif
#include "as_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Core functions
asIScriptEngine* asCreateScriptEngine(asUINT version);
const char* asGetLibraryVersion();
const char* asGetLibraryOptions();

// Engine reference counting
void asEngine_AddRef(asIScriptEngine *engine);
void asEngine_Release(asIScriptEngine *engine);
void asEngine_ShutDownAndRelease(asIScriptEngine *engine);

// Engine properties
int asEngine_SetEngineProperty(asIScriptEngine *engine, asEEngineProp property, asPWORD value);
asPWORD asEngine_GetEngineProperty(asIScriptEngine *engine, asEEngineProp property);

// Message callback
int asEngine_SetMessageCallback(asIScriptEngine *engine, asFUNCTION_t callback, void *obj, asDWORD callConv);
int asEngine_ClearMessageCallback(asIScriptEngine *engine);
int asEngine_WriteMessage(asIScriptEngine *engine, const char *section, int row, int col, int type, const char *message);

// JIT Compiler
asIJITCompiler* asEngine_GetJITCompiler(asIScriptEngine *engine);
int asEngine_SetJITCompiler(asIScriptEngine *engine, asIJITCompiler *compiler);

// Global functions
int asEngine_RegisterGlobalFunction(asIScriptEngine *engine, const char *declaration, asGENFUNC_t funcPointer, asDWORD callConv);
asUINT asEngine_GetGlobalFunctionCount(asIScriptEngine *engine);
asIScriptFunction* asEngine_GetGlobalFunctionByIndex(asIScriptEngine *engine, asUINT index);
asIScriptFunction* asEngine_GetGlobalFunctionByDecl(asIScriptEngine *engine, const char *decl);

// Global properties
int asEngine_RegisterGlobalProperty(asIScriptEngine *engine, const char *declaration, void *pointer);
asUINT asEngine_GetGlobalPropertyCount(asIScriptEngine *engine);
int asEngine_GetGlobalPropertyByIndex(asIScriptEngine *engine, asUINT index, const char **name, const char **nameSpace, int *typeId, asBOOL *isConst, const char **configGroup, void **pointer, asDWORD *accessMask);
int asEngine_GetGlobalPropertyIndexByName(asIScriptEngine *engine, const char *name);
int asEngine_GetGlobalPropertyIndexByDecl(asIScriptEngine *engine, const char *decl);

// Object types
int asEngine_RegisterObjectType(asIScriptEngine *engine, const char *name, int byteSize, asDWORD flags);
int asEngine_RegisterObjectProperty(asIScriptEngine *engine, const char *obj, const char *declaration, int byteOffset);
int asEngine_RegisterObjectMethod(asIScriptEngine *engine, const char *obj, const char *declaration, asGENFUNC_t funcPointer, asDWORD callConv);
int asEngine_RegisterObjectBehaviour(asIScriptEngine *engine, const char *obj, asEBehaviours behaviour, const char *declaration, asGENFUNC_t funcPointer, asDWORD callConv);

// Interfaces
int asEngine_RegisterInterface(asIScriptEngine *engine, const char *name);
int asEngine_RegisterInterfaceMethod(asIScriptEngine *engine, const char *intf, const char *declaration);

// String factory
int asEngine_GetStringFactoryReturnTypeId(asIScriptEngine *engine, asDWORD *flags);
int asEngine_RegisterStringFactory(asIScriptEngine *engine, const char *datatype, asIStringFactory *factory);

// Default array type
int asEngine_RegisterDefaultArrayType(asIScriptEngine *engine, const char *type);
int asEngine_GetDefaultArrayTypeId(asIScriptEngine *engine);

// Enums
int asEngine_RegisterEnum(asIScriptEngine *engine, const char *type);
int asEngine_RegisterEnumValue(asIScriptEngine *engine, const char *type, const char *name, int value);
asUINT asEngine_GetEnumCount(asIScriptEngine *engine);
asITypeInfo* asEngine_GetEnumByIndex(asIScriptEngine *engine, asUINT index);

// Funcdefs
int asEngine_RegisterFuncdef(asIScriptEngine *engine, const char *decl);
asUINT asEngine_GetFuncdefCount(asIScriptEngine *engine);
asITypeInfo* asEngine_GetFuncdefByIndex(asIScriptEngine *engine, asUINT index);

// Typedefs
int asEngine_RegisterTypedef(asIScriptEngine *engine, const char *type, const char *decl);
asUINT asEngine_GetTypedefCount(asIScriptEngine *engine);
asITypeInfo* asEngine_GetTypedefByIndex(asIScriptEngine *engine, asUINT index);

// Configuration groups
int asEngine_BeginConfigGroup(asIScriptEngine *engine, const char *groupName);
int asEngine_EndConfigGroup(asIScriptEngine *engine);
int asEngine_RemoveConfigGroup(asIScriptEngine *engine, const char *groupName);
asDWORD asEngine_SetDefaultAccessMask(asIScriptEngine *engine, asDWORD defaultMask);
int asEngine_SetDefaultNamespace(asIScriptEngine *engine, const char *nameSpace);
const char* asEngine_GetDefaultNamespace(asIScriptEngine *engine);

// Modules
asIScriptModule* asEngine_GetModule(asIScriptEngine *engine, const char *module, asEGMFlags flag);
int asEngine_DiscardModule(asIScriptEngine *engine, const char *module);
asUINT asEngine_GetModuleCount(asIScriptEngine *engine);
asIScriptModule* asEngine_GetModuleByIndex(asIScriptEngine *engine, asUINT index);

// Script object management
asIScriptContext* asEngine_CreateContext(asIScriptEngine *engine);
void* asEngine_CreateScriptObject(asIScriptEngine *engine, const asITypeInfo *type);
void* asEngine_CreateScriptObjectCopy(asIScriptEngine *engine, void *obj, const asITypeInfo *type);
void* asEngine_CreateUninitializedScriptObject(asIScriptEngine *engine, const asITypeInfo *type);
asIScriptFunction* asEngine_CreateDelegate(asIScriptEngine *engine, asIScriptFunction *func, void *obj);
int asEngine_AssignScriptObject(asIScriptEngine *engine, void *dstObj, void *srcObj, const asITypeInfo *type);
void asEngine_ReleaseScriptObject(asIScriptEngine *engine, void *obj, const asITypeInfo *type);
void asEngine_AddRefScriptObject(asIScriptEngine *engine, void *obj, const asITypeInfo *type);
int asEngine_RefCastObject(asIScriptEngine *engine, void *obj, asITypeInfo *fromType, asITypeInfo *toType, void **newPtr, asBOOL useOnlyImplicitCast);
asILockableSharedBool* asEngine_GetWeakRefFlagOfScriptObject(asIScriptEngine *engine, void *obj, const asITypeInfo *type);

// Context pooling
asIScriptContext* asEngine_RequestContext(asIScriptEngine *engine);
void asEngine_ReturnContext(asIScriptEngine *engine, asIScriptContext *ctx);
int asEngine_SetContextCallbacks(asIScriptEngine *engine, asREQUESTCONTEXTFUNC_t requestCtx, asRETURNCONTEXTFUNC_t returnCtx, void *param);

// Garbage collection
int asEngine_GarbageCollect(asIScriptEngine *engine, asDWORD flags);
void asEngine_GetGCStatistics(asIScriptEngine *engine, asUINT *currentSize, asUINT *totalDestroyed, asUINT *totalDetected, asUINT *newObjects, asUINT *totalNewDestroyed);
int asEngine_NotifyGarbageCollectorOfNewObject(asIScriptEngine *engine, void *obj, asITypeInfo *type);
int asEngine_GetObjectInGC(asIScriptEngine *engine, asUINT idx, asUINT *seqNbr, void **obj, asITypeInfo **type);
void asEngine_GCEnumCallback(asIScriptEngine *engine, void *reference);
void asEngine_ForwardGCEnumReferences(asIScriptEngine *engine, void *ref, asITypeInfo *type);
void asEngine_ForwardGCReleaseReferences(asIScriptEngine *engine, void *ref, asITypeInfo *type);
void asEngine_SetCircularRefDetectedCallback(asIScriptEngine *engine, asCIRCULARREFFUNC_t callback, void *param);

// Type identification
asITypeInfo* asEngine_GetTypeInfoByName(asIScriptEngine *engine, const char *name);
asITypeInfo* asEngine_GetTypeInfoByDecl(asIScriptEngine *engine, const char *decl);
int asEngine_GetTypeIdByDecl(asIScriptEngine *engine, const char *decl);
const char* asEngine_GetTypeDeclaration(asIScriptEngine *engine, int typeId, asBOOL includeNamespace);
int asEngine_GetSizeOfPrimitiveType(asIScriptEngine *engine, int typeId);
asITypeInfo* asEngine_GetTypeInfoById(asIScriptEngine *engine, int typeId);
asUINT asEngine_GetObjectTypeCount(asIScriptEngine *engine);
asITypeInfo* asEngine_GetObjectTypeByIndex(asIScriptEngine *engine, asUINT index);

// User data
void* asEngine_GetUserData(asIScriptEngine *engine, asPWORD type);
void* asEngine_SetUserData(asIScriptEngine *engine, void *data, asPWORD type);

int asEngine_GetLastFunctionId(asIScriptEngine *engine);
asIScriptFunction* asEngine_GetFunctionById(asIScriptEngine *engine, int funcId);

#ifdef __cplusplus
}
#endif

#endif // AS_ENGINE_H
