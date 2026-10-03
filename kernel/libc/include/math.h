#ifndef _KLIBC_MATH_H
#define _KLIBC_MATH_H

/* libm, фича c-math */
double sin(double x);   double cos(double x);   double tan(double x);
double asin(double x);  double acos(double x);  double atan(double x);
double sinh(double x);  double cosh(double x);  double tanh(double x);
double exp(double x);   double exp2(double x);
double log(double x);   double log2(double x);  double log10(double x);
double sqrt(double x);  double cbrt(double x);
double floor(double x); double ceil(double x);
double trunc(double x); double round(double x); double fabs(double x);
double atan2(double y, double x);
double pow(double x, double y);
double fmod(double x, double y);
double hypot(double x, double y);
double fmin(double x, double y);
double fmax(double x, double y);

float sinf(float x);  float cosf(float x);  float tanf(float x);
float expf(float x);  float logf(float x);  float sqrtf(float x);
float floorf(float x); float ceilf(float x); float fabsf(float x);
float powf(float x, float y);
float atan2f(float y, float x);
float fmodf(float x, float y);

#endif
