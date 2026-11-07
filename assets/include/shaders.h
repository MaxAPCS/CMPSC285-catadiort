/*
 * shaders.h: the four required shaders, their parameter structs, and the
 * noise function they and the scenes share.
 *
 * Scenes bind these by name, so keep the names and structs exactly as they
 * are. A shader reads its parameters by casting c->params to its struct.
 * rt.h includes this file at its end, so a scene that includes rt.h has
 * the shaders too.
 */
#ifndef SHADERS_H
#define SHADERS_H

#include "rt.h"

#ifdef __cplusplus
extern "C" {
#endif

/* TEXTURED: albedo from an image. albedoTex is an rtMakeTexture handle
 * (-1: leave the albedo alone). alphaTex is a second handle whose red
 * channel is the coverage mask, or -1 for the albedo image's own alpha;
 * only ALPHA_CUTOUT reads it. ObjDraw fills this from map_Kd and map_d. */
typedef struct {
    RtInt albedoTex;
    RtInt alphaTex;
} TexturedParams;

/* CHECKER: sizex checks across u and sizey across v, alternating two whole
 * materials, a (the check at u = v = 0) and b. */
typedef struct {
    RtInt sizex, sizey;
    RtMaterial a, b;
} CheckerParams;

/* ALPHA_CUTOUT: solid where the mask is >= threshold. Reads TexturedParams:
 * alphaTex's red channel if alphaTex >= 0, else albedoTex's alpha. */
typedef struct {
    TexturedParams tex;
    RtFloat threshold;    /* 0.5 is the usual value */
} CutoutParams;

void  TEXTURED(RtShadeContext *c);       /* surface, TexturedParams */
void  CHECKER(RtShadeContext *c);        /* surface, CheckerParams */
RtInt ALPHA_CUTOUT(RtShadeContext *c);   /* cutout, CutoutParams */
RtInt CHECKER_CUTOUT(RtShadeContext *c); /* cutout, CheckerParams: solid on the a checks, a hole on the b checks */

double perlinNoise(double x, double y, double z); /* provided complete (noise.cpp): Perlin's improved noise, as in HW1 */

#ifdef __cplusplus
}
#endif

#endif /* SHADERS_H */
