/* Private native Variant box. A box is immutable and reference counted;
 * generated LLVM supplies a unique per-type tag and a field-aware destructor.
 * Copying a Variant shares the box, while unboxing clones its payload according
 * to the payload type's managed-copy contract. */
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct ValoDynamic {
    uint64_t references;
    const void *type_tag;
    void (*drop_value)(void *);
    /* Current native value subset requires at most eight-byte alignment. */
    uint64_t alignment;
    unsigned char data[];
} ValoDynamic;

static uint64_t live_values;
static int check_registered;

static void fail(void) { abort(); }
static void check_exit(void) {
    const char *enabled = getenv("VALO_RUNTIME_ASSERT_CLEAN");
    if (enabled && enabled[0] == '1' && live_values) fail();
}

void *__valo_dynamic_alloc(uint64_t bytes, const void *type_tag,
                           void (*drop_value)(void *)) {
    if (!type_tag || bytes > SIZE_MAX - offsetof(ValoDynamic, data)) fail();
    ValoDynamic *box = calloc(1, offsetof(ValoDynamic, data) + (size_t)bytes);
    if (!box) fail();
    if (!check_registered) {
        if (atexit(check_exit) != 0) fail();
        check_registered = 1;
    }
    ++live_values;
    box->references = 1;
    box->type_tag = type_tag;
    box->drop_value = drop_value;
    return box;
}

void *__valo_dynamic_data(void *handle) {
    if (!handle) fail();
    return ((ValoDynamic *)handle)->data;
}

const void *__valo_dynamic_tag(void *handle) {
    if (!handle) fail();
    return ((ValoDynamic *)handle)->type_tag;
}

void *__valo_dynamic_retain(void *handle) {
    ValoDynamic *box = handle;
    if (box) {
        if (!box->references || box->references == UINT64_MAX) fail();
        ++box->references;
    }
    return handle;
}

void __valo_dynamic_release(void *handle) {
    ValoDynamic *box = handle;
    if (box) {
        if (!box->references) fail();
        if (--box->references == 0) {
            if (box->drop_value) box->drop_value(box->data);
            if (!live_values) fail();
            --live_values;
            free(box);
        }
    }
}
