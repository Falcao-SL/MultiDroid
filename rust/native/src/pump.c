#include "md_frame.h"

#include <stdlib.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <pthread.h>
#endif

struct MdPump {
#ifdef _WIN32
    CRITICAL_SECTION mu;
    CONDITION_VARIABLE work_cv;
    CONDITION_VARIABLE done_cv;
    HANDLE thread;
    DWORD worker_id;
#else
    pthread_mutex_t mu;
    pthread_cond_t work_cv;
    pthread_cond_t done_cv;
    pthread_t thread;
#endif
    int started;
    int stop;
    int job;
    int failed;
    uint8_t *dst;
    const uint8_t *src;
    size_t bytes;
    uint64_t blit_ns;
};

#ifdef _WIN32
static DWORD WINAPI md_worker(LPVOID arg) {
    MdPump *p = arg;
    EnterCriticalSection(&p->mu);
    p->worker_id = GetCurrentThreadId();
    p->started = 1;
    WakeConditionVariable(&p->done_cv);
    for (;;) {
        while (!p->stop && !p->job) {
            SleepConditionVariableCS(&p->work_cv, &p->mu, INFINITE);
        }
        if (p->stop && !p->job) {
            break;
        }
        {
            uint8_t *dst = p->dst;
            const uint8_t *src = p->src;
            size_t n = p->bytes;
            size_t wrote;
            LeaveCriticalSection(&p->mu);
            wrote = md_frame_opaque(dst, src, n);
            EnterCriticalSection(&p->mu);
            p->blit_ns = md_frame_last_blit_ns();
            p->failed = wrote != n;
            p->job = 0;
            WakeConditionVariable(&p->done_cv);
        }
        if (p->stop) {
            break;
        }
    }
    LeaveCriticalSection(&p->mu);
    return 0;
}
#else
static void *md_worker(void *arg) {
    MdPump *p = arg;
    pthread_mutex_lock(&p->mu);
    p->started = 1;
    pthread_cond_signal(&p->done_cv);
    for (;;) {
        while (!p->stop && !p->job) {
            pthread_cond_wait(&p->work_cv, &p->mu);
        }
        if (p->stop && !p->job) {
            break;
        }
        {
            uint8_t *dst = p->dst;
            const uint8_t *src = p->src;
            size_t n = p->bytes;
            size_t wrote;
            pthread_mutex_unlock(&p->mu);
            wrote = md_frame_opaque(dst, src, n);
            pthread_mutex_lock(&p->mu);
            p->blit_ns = md_frame_last_blit_ns();
            p->failed = wrote != n;
            p->job = 0;
            pthread_cond_signal(&p->done_cv);
        }
        if (p->stop) {
            break;
        }
    }
    pthread_mutex_unlock(&p->mu);
    return NULL;
}
#endif

MdPump *md_pump_new(void) {
    MdPump *p = calloc(1, sizeof(*p));
    if (p == NULL) {
        return NULL;
    }
#ifdef _WIN32
    InitializeCriticalSection(&p->mu);
    InitializeConditionVariable(&p->work_cv);
    InitializeConditionVariable(&p->done_cv);
    EnterCriticalSection(&p->mu);
    p->thread = CreateThread(NULL, 0, md_worker, p, 0, NULL);
    if (p->thread == NULL) {
        LeaveCriticalSection(&p->mu);
        DeleteCriticalSection(&p->mu);
        free(p);
        return NULL;
    }
    while (!p->started) {
        SleepConditionVariableCS(&p->done_cv, &p->mu, INFINITE);
    }
    LeaveCriticalSection(&p->mu);
#else
    if (pthread_mutex_init(&p->mu, NULL) != 0) {
        free(p);
        return NULL;
    }
    if (pthread_cond_init(&p->work_cv, NULL) != 0 || pthread_cond_init(&p->done_cv, NULL) != 0) {
        pthread_mutex_destroy(&p->mu);
        free(p);
        return NULL;
    }
    pthread_mutex_lock(&p->mu);
    if (pthread_create(&p->thread, NULL, md_worker, p) != 0) {
        pthread_mutex_unlock(&p->mu);
        pthread_cond_destroy(&p->work_cv);
        pthread_cond_destroy(&p->done_cv);
        pthread_mutex_destroy(&p->mu);
        free(p);
        return NULL;
    }
    while (!p->started) {
        pthread_cond_wait(&p->done_cv, &p->mu);
    }
    pthread_mutex_unlock(&p->mu);
#endif
    return p;
}

void md_pump_free(MdPump *p) {
    if (p == NULL) {
        return;
    }
#ifdef _WIN32
    EnterCriticalSection(&p->mu);
    p->stop = 1;
    WakeConditionVariable(&p->work_cv);
    LeaveCriticalSection(&p->mu);
    WaitForSingleObject(p->thread, INFINITE);
    CloseHandle(p->thread);
    DeleteCriticalSection(&p->mu);
#else
    pthread_mutex_lock(&p->mu);
    p->stop = 1;
    pthread_cond_signal(&p->work_cv);
    pthread_mutex_unlock(&p->mu);
    pthread_join(p->thread, NULL);
    pthread_cond_destroy(&p->work_cv);
    pthread_cond_destroy(&p->done_cv);
    pthread_mutex_destroy(&p->mu);
#endif
    free(p);
}

int md_pump_blit(MdPump *p, uint8_t *dst, const uint8_t *src, size_t bytes, uint64_t *out_ns) {
    if (p == NULL || dst == NULL || src == NULL || (bytes % 4) != 0) {
        return 0;
    }
#ifdef _WIN32
    EnterCriticalSection(&p->mu);
#else
    pthread_mutex_lock(&p->mu);
#endif
    p->dst = dst;
    p->src = src;
    p->bytes = bytes;
    p->failed = 0;
    p->job = 1;
#ifdef _WIN32
    WakeConditionVariable(&p->work_cv);
    while (p->job) {
        SleepConditionVariableCS(&p->done_cv, &p->mu, INFINITE);
    }
    if (out_ns != NULL) {
        *out_ns = p->blit_ns;
    }
    {
        int ok = !p->failed;
        LeaveCriticalSection(&p->mu);
        return ok;
    }
#else
    pthread_cond_signal(&p->work_cv);
    while (p->job) {
        pthread_cond_wait(&p->done_cv, &p->mu);
    }
    if (out_ns != NULL) {
        *out_ns = p->blit_ns;
    }
    {
        int ok = !p->failed;
        pthread_mutex_unlock(&p->mu);
        return ok;
    }
#endif
}

int md_pump_caller_is_worker(const MdPump *p) {
    if (p == NULL) {
        return 0;
    }
#ifdef _WIN32
    return GetCurrentThreadId() == p->worker_id;
#else
    return pthread_equal(pthread_self(), p->thread);
#endif
}
