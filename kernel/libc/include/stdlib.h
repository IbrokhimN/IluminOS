#ifndef _KLIBC_STDLIB_H
#define _KLIBC_STDLIB_H

#include <stddef.h>

/* tinyrlibc */
void *malloc(size_t size);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);

int atoi(const char *s);
long strtol(const char *s, char **end, int base);
unsigned long strtoul(const char *s, char **end, int base);
long long strtoll(const char *s, char **end, int base);
unsigned long long strtoull(const char *s, char **end, int base);

int abs(int x);
void qsort(void *base, size_t nmemb, size_t size,
           int (*cmp)(const void *, const void *));
int rand(void);
void srand(unsigned int seed);
int rand_r(unsigned int *seedp);

__attribute__((noreturn)) void abort(void);
__attribute__((noreturn)) void exit(int code);

#endif
