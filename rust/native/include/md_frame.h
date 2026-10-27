/* Núcleo de quadro do MultiDroid. Roda no processo hospedeiro (o binário
   Rust), não dentro do Android. Não é o NDK do Android: não há JNI nem APK.
   Ver docs/arquitetura/ADR-001-nucleo-nativo-e-android-studio.md. */

#ifndef MD_FRAME_H
#define MD_FRAME_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) && defined(MD_FRAME_BUILD_SHARED)
#define MD_API __declspec(dllexport)
#elif defined(MD_FRAME_BUILD_SHARED)
#define MD_API __attribute__((visibility("default")))
#else
#define MD_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef enum MdHost {
    MD_HOST_UNKNOWN = 0,
    /* Apple Silicon: NEON neste corte. Metal no app fica para depois. */
    MD_HOST_APPLE = 1,
    /* O guest no Windows segue em WHPX. Aqui o quadro usa SSE2. */
    MD_HOST_WINDOWS = 2,
    /* KVM (emulador) ou Redroid. NEON ou SSE2, conforme a CPU. */
    MD_HOST_LINUX = 3
} MdHost;

MD_API MdHost md_host(void);

/* Copia `bytes` de RGBA e força o alfa em 255. `dst` e `src` podem ser o
   mesmo ponteiro. `bytes` que não fecha pixel é ignorado no fim.
   Devolve os bytes escritos (múltiplo de 4), ou 0 se o ponteiro for nulo. */
MD_API size_t md_frame_opaque(uint8_t *dst, const uint8_t *src, size_t bytes);

/* Nanossegundos do último md_frame_opaque nesta thread. */
MD_API uint64_t md_frame_last_blit_ns(void);

/* Uma thread só para o blit. O chamador espera o fim: o mmap do emulador
   é um buffer só, então a cópia tem de terminar antes do próximo quadro. */
typedef struct MdPump MdPump;

MD_API MdPump *md_pump_new(void);
MD_API void md_pump_free(MdPump *pump);

/* Escreve o quadro opaco em `dst` (pelo menos `bytes`, múltiplo de 4).
   `dst` e `src` vivem até a função voltar. Devolve 1 se escreveu. */
MD_API int md_pump_blit(MdPump *pump, uint8_t *dst, const uint8_t *src, size_t bytes,
                        uint64_t *out_ns);

/* 1 se quem chama é a thread do blit. */
MD_API int md_pump_caller_is_worker(const MdPump *pump);

#ifdef __cplusplus
}
#endif

#endif
