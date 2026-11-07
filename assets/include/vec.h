/*
 * vec.h: a plain three-component float vector with the operations a ray
 * tracer needs. Header-only; no SIMD, no padding. The renderer uses it
 * everywhere and the scene files may; the interface (rt.h) passes float[3]
 * arrays, and a Vec3 converts both ways. An array of Vec3 has the layout
 * of an array of RtPoint, but the renderer copies rather than casts.
 */
#ifndef VEC_H
#define VEC_H

#include <algorithm>
#include <cmath>

struct Vec3 {
    float x, y, z;

    Vec3() : x(0), y(0), z(0) {}
    Vec3(float x_, float y_, float z_) : x(x_), y(y_), z(z_) {}
    explicit Vec3(float s) : x(s), y(s), z(s) {}
    explicit Vec3(const float *a) : x(a[0]), y(a[1]), z(a[2]) {} /* from an RtPoint, RtColor, ... */

    /* To an RtPoint, RtColor, ... */
    void store(float *a) const
    {
        a[0] = x;
        a[1] = y;
        a[2] = z;
    }

    float operator[](int i) const { return i == 0 ? x : i == 1 ? y : z; }
    float &operator[](int i) { return i == 0 ? x : i == 1 ? y : z; }

    Vec3 operator-() const { return Vec3(-x, -y, -z); }
    Vec3 operator+(const Vec3 &b) const { return Vec3(x + b.x, y + b.y, z + b.z); }
    Vec3 operator-(const Vec3 &b) const { return Vec3(x - b.x, y - b.y, z - b.z); }
    Vec3 operator*(const Vec3 &b) const { return Vec3(x * b.x, y * b.y, z * b.z); } /* componentwise */
    Vec3 operator/(const Vec3 &b) const { return Vec3(x / b.x, y / b.y, z / b.z); }
    Vec3 operator*(float s) const { return Vec3(x * s, y * s, z * s); }
    Vec3 operator/(float s) const { return Vec3(x / s, y / s, z / s); }
    Vec3 &operator+=(const Vec3 &b) { return *this = *this + b; }
    Vec3 &operator-=(const Vec3 &b) { return *this = *this - b; }
    Vec3 &operator*=(const Vec3 &b) { return *this = *this * b; }
    Vec3 &operator*=(float s) { return *this = *this * s; }
    Vec3 &operator/=(float s) { return *this = *this / s; }
    bool operator==(const Vec3 &b) const { return x == b.x && y == b.y && z == b.z; }
    bool operator!=(const Vec3 &b) const { return !(*this == b); }
};

inline Vec3 operator*(float s, const Vec3 &v) { return v * s; }

inline float dot(const Vec3 &a, const Vec3 &b) { return a.x * b.x + a.y * b.y + a.z * b.z; }

inline Vec3 cross(const Vec3 &a, const Vec3 &b)
{
    return Vec3(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x);
}

inline float lengthSquared(const Vec3 &v) { return dot(v, v); }
inline float length(const Vec3 &v) { return std::sqrt(dot(v, v)); }

/* The unit vector along v; v itself if it has no length. */
inline Vec3 normalize(const Vec3 &v)
{
    float len = length(v);
    return len > 0 ? v / len : v;
}

inline Vec3 vmin(const Vec3 &a, const Vec3 &b)
{
    return Vec3(std::min(a.x, b.x), std::min(a.y, b.y), std::min(a.z, b.z));
}
inline Vec3 vmax(const Vec3 &a, const Vec3 &b)
{
    return Vec3(std::max(a.x, b.x), std::max(a.y, b.y), std::max(a.z, b.z));
}
inline Vec3 vabs(const Vec3 &v) { return Vec3(std::fabs(v.x), std::fabs(v.y), std::fabs(v.z)); }
inline float maxComponent(const Vec3 &v) { return std::max(v.x, std::max(v.y, v.z)); }
inline float minComponent(const Vec3 &v) { return std::min(v.x, std::min(v.y, v.z)); }

/* The index (0, 1, 2) of the largest component. */
inline int maxAxis(const Vec3 &v) { return v.x >= v.y ? (v.x >= v.z ? 0 : 2) : (v.y >= v.z ? 1 : 2); }

/* a + (b - a) t, componentwise */
inline Vec3 lerp(const Vec3 &a, const Vec3 &b, float t) { return a + (b - a) * t; }

inline Vec3 clamp01(const Vec3 &v)
{
    return Vec3(std::min(1.0f, std::max(0.0f, v.x)), std::min(1.0f, std::max(0.0f, v.y)),
                std::min(1.0f, std::max(0.0f, v.z)));
}

/* The mirror of v about the unit normal n, for v pointing toward the
 * surface: r = v - 2 (v . n) n. */
inline Vec3 reflect(const Vec3 &v, const Vec3 &n) { return v - n * (2 * dot(v, n)); }

#endif /* VEC_H */
