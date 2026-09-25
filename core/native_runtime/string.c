/* Private experimental Valo runtime ABI. A null handle is the empty String.
 * Literal headers use UINT64_MAX references and are never freed. */
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

typedef struct ValoString {
    uint64_t references;
    uint64_t bytes;
    uint64_t scalars;
    unsigned char data[];
} ValoString;

static void fail(void) { abort(); }
static uint64_t live_allocations;
static int exit_check_registered;

static void check_exit(void) {
    const char *enabled = getenv("VALO_RUNTIME_ASSERT_CLEAN");
    if (enabled && enabled[0] == '1' && live_allocations != 0) fail();
}

static void allocation_created(void) {
    if (!exit_check_registered) {
        if (atexit(check_exit) != 0) fail();
        exit_check_registered = 1;
    }
    ++live_allocations;
}

void *__valo_string_clone(void *handle) {
    ValoString *value = handle;
    if (value && value->references != UINT64_MAX) {
        if (!value->references || value->references == UINT64_MAX - 1) fail();
        ++value->references;
    }
    return handle;
}

void __valo_string_release(void *handle) {
    ValoString *value = handle;
    if (value && value->references != UINT64_MAX) {
        if (!value->references) fail();
        if (--value->references == 0) {
            if (!live_allocations) fail();
            --live_allocations;
            free(value);
        }
    }
}

static uint64_t byte_count(const ValoString *value) { return value ? value->bytes : 0; }
static uint64_t scalar_count(const ValoString *value) { return value ? value->scalars : 0; }

void *__valo_string_concat_consume(void *left_handle, void *right_handle) {
    ValoString *left = left_handle, *right = right_handle;
    uint64_t a = byte_count(left), b = byte_count(right);
    if (a > SIZE_MAX - offsetof(ValoString, data) ||
        b > SIZE_MAX - offsetof(ValoString, data) - a) fail();
    ValoString *result = malloc(offsetof(ValoString, data) + (size_t)(a + b));
    if (!result) fail();
    allocation_created();
    result->references = 1;
    result->bytes = a + b;
    result->scalars = scalar_count(left) + scalar_count(right);
    if (a) memcpy(result->data, left->data, (size_t)a);
    if (b) memcpy(result->data + a, right->data, (size_t)b);
    __valo_string_release(left);
    __valo_string_release(right);
    return result;
}

int32_t __valo_string_compare_consume(void *left_handle, void *right_handle) {
    ValoString *left = left_handle, *right = right_handle;
    uint64_t a = byte_count(left), b = byte_count(right);
    size_t common = (size_t)(a < b ? a : b);
    int result = common ? memcmp(left->data, right->data, common) : 0;
    if (!result) result = (a > b) - (a < b);
    __valo_string_release(left);
    __valo_string_release(right);
    return (result > 0) - (result < 0);
}

int32_t __valo_string_len_consume(void *handle) {
    ValoString *value = handle;
    uint64_t length = scalar_count(value);
    __valo_string_release(handle);
    if (length > INT32_MAX) fail();
    return (int32_t)length;
}

static void *from_ascii(const char *text, size_t length) {
    if (length > SIZE_MAX - offsetof(ValoString, data)) fail();
    ValoString *result = malloc(offsetof(ValoString, data) + length);
    if (!result) fail();
    allocation_created();
    result->references = 1;
    result->bytes = length;
    result->scalars = length;
    if (length) memcpy(result->data, text, length);
    return result;
}

void *__valo_string_from_i64(int64_t number) {
    char buffer[64];
    int length = snprintf(buffer, sizeof buffer, "%lld", (long long)number);
    if (length < 0 || (size_t)length >= sizeof buffer) fail();
    return from_ascii(buffer, (size_t)length);
}

void *__valo_string_from_u64(uint64_t number) {
    char buffer[64];
    int length = snprintf(buffer, sizeof buffer, "%llu", (unsigned long long)number);
    if (length < 0 || (size_t)length >= sizeof buffer) fail();
    return from_ascii(buffer, (size_t)length);
}

void *__valo_string_from_fixed(double number, int32_t decimals) {
    char buffer[128];
    if (decimals < 1 || decimals > 6) fail();
    int length = snprintf(buffer, sizeof buffer, "%.*f", decimals, number);
    if (length < 0 || (size_t)length >= sizeof buffer) fail();
    return from_ascii(buffer, (size_t)length);
}

void *__valo_string_from_bool(int32_t value) {
    return value ? from_ascii("True", 4) : from_ascii("False", 5);
}
