#include "md_frame.h"

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <time.h>
#endif

#if defined(__aarch64__)
#include <arm_neon.h>
#elif defined(__x86_64__) || defined(_M_X64)
#include <emmintrin.h>
#endif

#if defined(_MSC_VER)
static __declspec(thread) uint64_t md_last_ns;
#else
static _Thread_local uint64_t md_last_ns;
#endif

#if defined(__aarch64__) || defined(__x86_64__) || defined(_M_X64)
static const uint8_t k_alpha[16] = {
    0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255,
};
#endif

static uint64_t md_now(void) {
#if defined(_WIN32)
    static LARGE_INTEGER freq;
    LARGE_INTEGER c;
    if (freq.QuadPart == 0) {
        QueryPerformanceFrequency(&freq);
    }
    QueryPerformanceCounter(&c);
    return (uint64_t)c.QuadPart * 1000000000ull / (uint64_t)freq.QuadPart;
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
#endif
}

MdHost md_host(void) {
#if defined(MD_BUILD_FOR_APPLE)
    return MD_HOST_APPLE;
#elif defined(MD_BUILD_FOR_WINDOWS)
    return MD_HOST_WINDOWS;
#elif defined(MD_BUILD_FOR_LINUX)
    return MD_HOST_LINUX;
#else
    return MD_HOST_UNKNOWN;
#endif
}

size_t md_frame_opaque(uint8_t *dst, const uint8_t *src, size_t bytes) {
    size_t n;
    size_t i;
    uint64_t t0;

    if (dst == NULL || src == NULL) {
        return 0;
    }
    n = bytes & ~(size_t)3;
    t0 = md_now();
    i = 0;
#if defined(__aarch64__)
    {
        uint8x16_t alpha = vld1q_u8(k_alpha);
        for (; i + 16 <= n; i += 16) {
            uint8x16_t p = vld1q_u8(src + i);
            vst1q_u8(dst + i, vorrq_u8(p, alpha));
        }
    }
#elif defined(__x86_64__) || defined(_M_X64)
    {
        __m128i alpha = _mm_loadu_si128((const __m128i *)k_alpha);
        for (; i + 16 <= n; i += 16) {
            __m128i p = _mm_loadu_si128((const __m128i *)(src + i));
            _mm_storeu_si128((__m128i *)(dst + i), _mm_or_si128(p, alpha));
        }
    }
#endif
    for (; i < n; i += 4) {
        dst[i] = src[i];
        dst[i + 1] = src[i + 1];
        dst[i + 2] = src[i + 2];
        dst[i + 3] = 255;
    }
    md_last_ns = md_now() - t0;
    return n;
}

uint64_t md_frame_last_blit_ns(void) { return md_last_ns; }
