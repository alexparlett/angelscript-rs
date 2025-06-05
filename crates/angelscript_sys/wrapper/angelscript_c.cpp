#include "angelscript_c.h"

#ifdef __cplusplus
extern "C" {
#endif

asSFuncPtr asGenericFunction(asGENFUNC_t func) {
    return asFUNCTION(func);
}
asSFuncPtr asFunction(asFUNCTION_t func) {
    return asFUNCTION(func);
}
asSFuncPtr asMessageInfoFunction(asMESSAGEINFOFUNC_t func) {
    return asFUNCTION(func);
}

asSFuncPtr asScriptContextFunction(asSCRIPTCONTEXTFUNC_t func) {
    return asFUNCTION(func);
}

#ifdef __cplusplus
}
#endif