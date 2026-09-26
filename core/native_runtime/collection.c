/* Private ordered Collection runtime. Elements are tagged native Variant boxes,
 * never interpreter Values or untyped payload pointers. Public indexing is
 * 1-based. Keys and named positions remain a compiler-diagnosed extension. */
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct ValoDynamic ValoDynamic;
extern void *__valo_dynamic_retain(void *);
extern void __valo_dynamic_release(void *);

typedef struct ValoCollection {
    uint64_t references;
    uint64_t length;
    uint64_t capacity;
    ValoDynamic **items;
} ValoCollection;

static uint64_t live_collections;
static int check_registered;
static void fail(void) { abort(); }
static void check_exit(void) {
    const char *enabled = getenv("VALO_RUNTIME_ASSERT_CLEAN");
    if (enabled && enabled[0] == '1' && live_collections) fail();
}

void *__valo_collection_new(void) {
    ValoCollection *collection = calloc(1, sizeof(*collection));
    if (!collection) fail();
    if (!check_registered) {
        if (atexit(check_exit) != 0) fail();
        check_registered = 1;
    }
    ++live_collections;
    collection->references = 1;
    return collection;
}

void *__valo_collection_retain(void *handle) {
    ValoCollection *collection = handle;
    if (collection) {
        if (!collection->references || collection->references == UINT64_MAX) fail();
        ++collection->references;
    }
    return handle;
}

void __valo_collection_release(void *handle) {
    ValoCollection *collection = handle;
    if (collection) {
        if (!collection->references) fail();
        if (--collection->references == 0) {
            for (uint64_t i = collection->length; i > 0; --i)
                __valo_dynamic_release(collection->items[i - 1]);
            free(collection->items);
            if (!live_collections) fail();
            --live_collections;
            free(collection);
        }
    }
}

static void grow(ValoCollection *collection) {
    if (collection->length < collection->capacity) return;
    uint64_t capacity = collection->capacity ? collection->capacity * 2 : 8;
    if (capacity <= collection->capacity || capacity > SIZE_MAX / sizeof(*collection->items)) fail();
    ValoDynamic **items = realloc(collection->items, (size_t)capacity * sizeof(*items));
    if (!items) fail();
    collection->items = items;
    collection->capacity = capacity;
}

/* before == 0 appends; otherwise inserts before that 1-based item.
 * The element's owned share transfers to Collection on success. */
void __valo_collection_add_consume(void *handle, void *element, int64_t before) {
    ValoCollection *collection = handle;
    if (!collection || !element) fail();
    uint64_t at = before == 0 ? collection->length : (uint64_t)(before - 1);
    if (before < 0 || (before != 0 && (uint64_t)before > collection->length + 1)) fail();
    grow(collection);
    memmove(collection->items + at + 1, collection->items + at,
            (size_t)(collection->length - at) * sizeof(*collection->items));
    collection->items[at] = element;
    ++collection->length;
}

int32_t __valo_collection_count(void *handle) {
    ValoCollection *collection = handle;
    if (!collection || collection->length > INT32_MAX) fail();
    return (int32_t)collection->length;
}

void *__valo_collection_item(void *handle, int64_t index) {
    ValoCollection *collection = handle;
    if (!collection || index < 1 || (uint64_t)index > collection->length) fail();
    return __valo_dynamic_retain(collection->items[index - 1]);
}

void __valo_collection_remove(void *handle, int64_t index) {
    ValoCollection *collection = handle;
    if (!collection || index < 1 || (uint64_t)index > collection->length) fail();
    uint64_t at = (uint64_t)index - 1;
    __valo_dynamic_release(collection->items[at]);
    memmove(collection->items + at, collection->items + at + 1,
            (size_t)(collection->length - at - 1) * sizeof(*collection->items));
    --collection->length;
}

void *__valo_collection_snapshot(void *handle) {
    ValoCollection *source = handle;
    if (!source) fail();
    ValoCollection *copy = __valo_collection_new();
    for (uint64_t i = 0; i < source->length; ++i)
        __valo_collection_add_consume(copy, __valo_dynamic_retain(source->items[i]), 0);
    return copy;
}
