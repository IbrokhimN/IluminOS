#ifndef _KLIBC_CTYPE_H
#define _KLIBC_CTYPE_H

/* tinyrlibc */
int isalpha(int c);
int isdigit(int c);
int isspace(int c);
int isupper(int c);

/* src/libc/ext.rs */
int isalnum(int c);
int islower(int c);
int isxdigit(int c);
int ispunct(int c);
int iscntrl(int c);
int isprint(int c);
int toupper(int c);
int tolower(int c);

#endif
