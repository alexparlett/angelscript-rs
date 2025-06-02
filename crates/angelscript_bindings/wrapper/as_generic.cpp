#include "as_generic.h"
#include <string>

extern "C" {
    unsigned int asIScriptGeneric_GetArgDWord(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgDWord(idx);
    }
    asQWORD asIScriptGeneric_GetArgQWord(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgQWord(idx);
    }
    float asIScriptGeneric_GetArgFloat(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgFloat(idx);
    }
    double asIScriptGeneric_GetArgDouble(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgDouble(idx);
    }
    void* asIScriptGeneric_GetArgAddress(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgAddress(idx);
    }
    void* asIScriptGeneric_GetArgObject(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgObject(idx);
    }
    const char*  asIScriptGeneric_GetArgString(asIScriptGeneric* g, asUINT idx) {
        std::string* a = static_cast<std::string*>(g->GetArgObject(idx));
        return a->c_str();
    }
    void* asIScriptGeneric_GetAddressOfReturnLocation(asIScriptGeneric* g) {
        return g->GetAddressOfReturnLocation();
    }
    void* asIScriptGeneric_GetAddressOfArg(asIScriptGeneric* g, asUINT idx) {
        return g->GetAddressOfArg(idx);
    }
    void asIScriptGeneric_SetReturnDWord(asIScriptGeneric* g, asUINT val) {
        g->SetReturnDWord(val);
    }
    void asIScriptGeneric_SetReturnQWord(asIScriptGeneric* g, asQWORD val) {
        g->SetReturnQWord(val);
    }
    void asIScriptGeneric_SetReturnFloat(asIScriptGeneric* g, float val) {
        g->SetReturnFloat(val);
    }
    void asIScriptGeneric_SetReturnDouble(asIScriptGeneric* g, double val) {
        g->SetReturnDouble(val);
    }
    void asIScriptGeneric_SetReturnAddress(asIScriptGeneric* g, void* addr) {
        g->SetReturnAddress(addr);
    }
    void asIScriptGeneric_SetReturnObject(asIScriptGeneric* g, void* obj) {
        g->SetReturnObject(obj);
    }
    void* asIScriptGeneric_GetObject(asIScriptGeneric* g) {
        return g->GetObject();
    }
    int asIScriptGeneric_GetObjectTypeId(asIScriptGeneric* g) {
        return g->GetObjectTypeId();
    }
    int asIScriptGeneric_GetArgTypeId(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgTypeId(idx);
    }
    asIScriptFunction* asIScriptGeneric_GetFunction(asIScriptGeneric* g) {
        if (!g) return nullptr;
        return const_cast<asIScriptFunction*>(g->GetFunction());
    }
    asIScriptEngine* asIScriptGeneric_GetEngine(asIScriptGeneric* g) {
        if (!g) return nullptr;
        return g->GetEngine();
    }

}