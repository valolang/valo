/* Private, single-threaded Valo Class reference ABI.  The generated LLVM
 * payload starts with this header; LLVM's target layout determines all field
 * offsets after it.  A null handle is Nothing. */
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>

typedef struct ValoObject {
    uint64_t references;
    void (*drop_fields)(void *);
} ValoObject;

static uint64_t live_objects;
static int exit_check_registered;

static void fail(void) { abort(); }

static void check_exit(void) {
    const char *enabled = getenv("VALO_RUNTIME_ASSERT_CLEAN");
    if (enabled && enabled[0] == '1' && live_objects != 0) fail();
}

void *__valo_object_alloc(uint64_t size, void (*drop_fields)(void *)) {
    if (size < sizeof(ValoObject) || size > SIZE_MAX) fail();
    ValoObject *object = calloc(1, (size_t)size);
    if (!object) fail();
    if (!exit_check_registered) {
        if (atexit(check_exit) != 0) fail();
        exit_check_registered = 1;
    }
    ++live_objects;
    object->references = 1;
    object->drop_fields = drop_fields;
    return object;
}

void *__valo_object_retain(void *handle) {
    ValoObject *object = handle;
    if (object) {
        if (!object->references || object->references == UINT64_MAX) fail();
        ++object->references;
    }
    return handle;
}

void __valo_object_release(void *handle) {
    ValoObject *object = handle;
    if (object) {
        if (!object->references) fail();
        if (--object->references == 0) {
            if (object->drop_fields) object->drop_fields(object);
            if (!live_objects) fail();
            --live_objects;
            free(object);
        }
    }
}
