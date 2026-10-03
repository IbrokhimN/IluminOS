#ifndef _KLIBC_STDIO_H
#define _KLIBC_STDIO_H

#include <stdarg.h>
#include <stddef.h>

int printf(const char *fmt, ...);
int vprintf(const char *fmt, va_list ap);
int sprintf(char *buf, const char *fmt, ...);
int snprintf(char *buf, size_t size, const char *fmt, ...);
int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
int putchar(int c);
int puts(const char *s);

#endif
