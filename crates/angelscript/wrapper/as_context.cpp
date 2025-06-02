#include "as_context.h"

extern "C" {

// Context management
asIScriptEngine* asContext_GetEngine(asIScriptContext *c) {
    if (!c) return nullptr;
    return static_cast<::asIScriptContext*>(c)->GetEngine();
}

int asContext_AddRef(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->AddRef();
}

int asContext_Release(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Release();
}

// Execution
asEContextState asContext_GetState(asIScriptContext *c) {
    if (!c) return asEXECUTION_ERROR;  // Replace this with an appropriate state, if necessary.
    return static_cast<asEContextState>(static_cast<::asIScriptContext*>(c)->GetState());
}

int asContext_Prepare(asIScriptContext *c, asIScriptFunction *func) {
    if (!c || !func) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Prepare(static_cast<::asIScriptFunction*>(func));
}

int asContext_Unprepare(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Unprepare();
}

int asContext_Execute(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Execute();
}

int asContext_Abort(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Abort();
}

int asContext_Suspend(asIScriptContext *c) {
    if (!c) return asINVALID_ARG;
    return static_cast<::asIScriptContext*>(c)->Suspend();
}

asEContextState asContext_GetStateOfExecution(asIScriptContext *c) {
    if (!c) return asEXECUTION_ERROR;  // Replace this with an appropriate state, if necessary.
    return static_cast<asEContextState>(static_cast<::asIScriptContext*>(c)->GetState());
}

} // extern "C"
