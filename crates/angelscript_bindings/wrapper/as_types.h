#ifndef AS_TYPES_H
#define AS_TYPES_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Define asBOOL if it's not already defined by AngelScript
#ifndef asBOOL
typedef unsigned int asBOOL;
#endif

#ifndef asTRUE
#define asTRUE 1
#endif

#ifndef asFALSE
#define asFALSE 0
#endif

// Helper functions for bool conversion
static inline unsigned int as_bool(unsigned int value) {
    return value ? 1 : 0;
}

static inline unsigned int from_as_bool(unsigned int value) {
    return value != 0;
}

#ifdef __cplusplus
}
#endif

#endif // AS_TYPES_H
