#include "as_generic.h"
#include <string>

extern "C" {
    asWORD asScriptGeneric_GetArgWord(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgWord(idx);
    }
    asBYTE asScriptGeneric_GetArgByte(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgByte(idx);
    }
    unsigned int asScriptGeneric_GetArgDWord(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgDWord(idx);
    }
    asQWORD asScriptGeneric_GetArgQWord(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgQWord(idx);
    }
    float asScriptGeneric_GetArgFloat(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgFloat(idx);
    }
    double asScriptGeneric_GetArgDouble(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgDouble(idx);
    }
    void* asScriptGeneric_GetArgAddress(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgAddress(idx);
    }
    void* asScriptGeneric_GetArgObject(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgObject(idx);
    }
    void* asScriptGeneric_GetAddressOfReturnLocation(asIScriptGeneric* g) {
        return g->GetAddressOfReturnLocation();
    }
    void* asScriptGeneric_GetAddressOfArg(asIScriptGeneric* g, asUINT idx) {
        return g->GetAddressOfArg(idx);
    }
    void asScriptGeneric_SetReturnDWord(asIScriptGeneric* g, asUINT val) {
        g->SetReturnDWord(val);
    }
    void asScriptGeneric_SetReturnQWord(asIScriptGeneric* g, asQWORD val) {
        g->SetReturnQWord(val);
    }
    void asScriptGeneric_SetReturnFloat(asIScriptGeneric* g, float val) {
        g->SetReturnFloat(val);
    }
    void asScriptGeneric_SetReturnDouble(asIScriptGeneric* g, double val) {
        g->SetReturnDouble(val);
    }
    void asScriptGeneric_SetReturnAddress(asIScriptGeneric* g, void* addr) {
        g->SetReturnAddress(addr);
    }
    void asScriptGeneric_SetReturnObject(asIScriptGeneric* g, void* obj) {
        g->SetReturnObject(obj);
    }
    void asScriptGeneric_SetReturnByte(asIScriptGeneric* g, asBYTE val) {
        g->SetReturnByte(val);
    }
    void* asScriptGeneric_GetObject(asIScriptGeneric* g) {
        return g->GetObject();
    }
    int asScriptGeneric_GetObjectTypeId(asIScriptGeneric* g) {
        return g->GetObjectTypeId();
    }
    int asScriptGeneric_GetArgTypeId(asIScriptGeneric* g, asUINT idx) {
        return g->GetArgTypeId(idx);
    }
    asIScriptFunction* asScriptGeneric_GetFunction(asIScriptGeneric* g) {
        if (!g) return nullptr;
        return const_cast<asIScriptFunction*>(g->GetFunction());
    }
    asIScriptEngine* asScriptGeneric_GetEngine(asIScriptGeneric* g) {
        if (!g) return nullptr;
        return g->GetEngine();
    }

}