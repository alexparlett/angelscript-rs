#include "as_stringfactory.h"

extern "C" {
    const void* asStringFactory_GetStringConstant(asIStringFactory* s, const char* data, asUINT length) {
        if (!s) return nullptr;
        return  static_cast<::asIStringFactory*>(s)->GetStringConstant(data, length);
    }

    int asStringFactory_ReleaseStringConstant(asIStringFactory* s, const void* str) {
        if (!s) return asINVALID_ARG;
        return  static_cast<::asIStringFactory*>(s)->ReleaseStringConstant(str);
    }

    int asStringFactory_GetRawStringData(asIStringFactory* s, const void* str, char* data, asUINT* length) {
        if (!s) return asINVALID_ARG;
        return  static_cast<::asIStringFactory*>(s)->GetRawStringData(str, data, length);
    }
}