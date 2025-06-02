#ifndef AS_STRINGFACTORY_H
#define AS_STRINGFACTORY_H

#ifndef ANGELSCRIPT_H
// Avoid having to inform include path if header is already include before
#include <angelscript.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

const void* asStringFactory_GetStringConstant(asIStringFactory* factory, const char *data, asUINT length);
int asStringFactory_ReleaseStringConstant(asIStringFactory* factory, const void *str);
int asStringFactory_GetRawStringData(asIStringFactory* factory, const void *str, char *data, asUINT *length);

#ifdef __cplusplus
}
#endif

#endif // AS_STRINGFACTORY_H