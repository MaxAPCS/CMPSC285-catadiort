/*
 * rt.h: the ECE/CS 285 ray tracing interface (HW2, HW3 and HW4). The scene
 * files compile against exactly these declarations; do not change them.
 * Everything has C linkage (rt.rs mirrors this file). Angles are degrees;
 * world space is right-handed with +y up; there is no camera space. The
 * calls marked [HW3] exist now so that no header ever changes: a renderer
 * records what they say and ignores it until HW3. HW4 adds no calls. What
 * each call means under whitted, cook and kajiya is the table in the HW2
 * handout's section on the interface.
 */
#ifndef RT_H
#define RT_H

#include <stddef.h> /* size_t, NULL */

#ifdef __cplusplus
extern "C" {
#endif

typedef void RtVoid;
typedef float RtFloat;
typedef int RtInt;
typedef RtFloat RtColor[3];
typedef RtFloat RtPoint[3];
typedef RtFloat RtVector[3];
typedef RtFloat RtNormal[3];
typedef RtFloat RtMatrix[4][4];

/* material ------------------------------------------------------------- */

/* What a surface is made of, at one point. Scenes set it with rtMaterial;
 * a surface shader may rewrite any field at the hit point. Each field says
 * who reads it and its symbol in the HW2 shading formula (Whitted
 * Illumination handout); ka,
 * the ambient coefficient, is rtAmbient's, for the whole scene. */
typedef struct RtMaterial {
    RtColor albedo;       /* kd: diffuse reflectance per channel, 0 to 1 */
    RtColor specular;     /* ks: the reflectance head on: it weights the mirror
                             ray and the highlight (HW2), the glossy lobe (HW3),
                             and is Schlick's F0, metal or not (HW4) */
    RtFloat roughness;    /* 0 = mirror, 1 = fully rough; the lobe's width
                             is alpha = roughness^2 in every homework.
                             HW2: the Phong exponent n = 2 / alpha^2 - 2
                             (alpha clamped to [0.05, 1]) and how much of
                             the mirror ray survives (Whitted handout); HW3: the
                             glossy lobe's width; HW4: GGX's alpha */
    RtFloat metallic;     /* 0 = dielectric, 1 = metal. HW4 only; HW2 and HW3
                             ignore it */
    RtFloat transmission; /* kt: 0 = opaque, 1 = clear glass */
    RtFloat ior;          /* eta: index of refraction of the inside (1.5 = glass) */
    RtColor emission;     /* radiance emitted (seen directly, this bright).
                             Nonzero makes the draw call a light: a point
                             light at its center in HW2 (Whitted handout), its
                             surface sampled in HW3, with MIS in HW4 */
} RtMaterial;

/* shaders -------------------------------------------------------------- */

/* What a shader sees at one hit point. The renderer fills the inputs; a
 * surface shader edits m (and N, for bump or normal mapping); a cutout
 * shader only reads. */
typedef struct RtShadeContext {
    /* inputs */
    RtPoint P;            /* the hit point, world space */
    RtPoint Po;           /* the same point in the primitive's object space
                             (its coordinates before the CTM), for solid
                             procedural patterns that stick to the object */
    RtNormal Ng;          /* unit geometric normal, world space, on the vertex
                             normals' side, turned to face the incoming ray */
    RtNormal N;           /* unit shading normal: the interpolated vertex
                             normal, or Ng; on Ng's side */
    RtVector dPdu, dPdv;  /* world-space tangents dP/du and dP/dv,
                             for normal mapping; a renderer may leave them
                             zero until it needs them */
    RtFloat u, v;         /* texture coordinates: a mesh's st (t = 0 at the top
                             row of a texture), a sphere's longitude and
                             latitude (Intersection handout) */
    RtVector I;           /* unit direction of the ray that hit, toward P */
    RtFloat time;         /* the ray's time in [0, 1): the pixel sample's
                             (rtMotionBegin); 0 when nothing moves, and
                             always under whitted */
    RtInt depth;          /* 0 for camera rays, +1 per bounce */
    const void *params;   /* this shader's parameter block, or NULL */
    /* in and out */
    RtMaterial m;         /* in: the material bound when the primitive was
                             drawn; out: what the surface shader leaves */
} RtShadeContext;

/* A surface shader edits c->m (and may replace c->N). */
typedef void (*RtShaderFn)(RtShadeContext *c);

/* A cutout shader returns 0 where the surface does not exist: the ray goes
 * on as if nothing were there. Runs on every candidate hit of every ray,
 * shadow rays included, with P, Po, Ng, N, u, v, time, depth (the ray's:
 * 0 for a camera ray, +1 per bounce, -1 for a shadow ray, so a lamp can
 * hide from the camera and cast no shadow) and params filled;
 * both roots of a sphere are candidates, so a hole shows its inside. */
typedef RtInt (*RtCutoutFn)(RtShadeContext *c);

/* session, options ----------------------------------------------------- */

RtVoid rtBegin(void);                 /* resets everything to its defaults (the stub's rtBegin lists them) */
RtVoid rtEnd(void);
RtVoid rtFormat(RtInt xres, RtInt yres);
RtVoid rtDisplay(const char *name);   /* writes renders/<integrator>/<name>.png (sRGB), <integrator> being
                                         1_Whitted, 2_Cook or 3_Kajiya; with --pfm also the .pfm (linear) */
RtFloat rtSceneArg(RtInt index, RtFloat defaultValue); /* the index-th number after the scene's name on the
                                          command line (gauchoRT MoleculeMovie 8), defaultValue when there is
                                          none: a scene's own parameters, with their defaults in the scene */
RtVoid rtTonemap(const char *op, RtFloat exposure); /* how the PNG is made from linear radiance: times
                                         2^exposure (stops), then "none" (a clamp to [0, 1], the default),
                                         "reinhard", "aces" or "filmic" (a curve with a shoulder),
                                         then sRGB. Provided complete (image.cpp); the .pfm stays linear */
RtVoid rtPixelSamples(RtInt spp);     /* samples per pixel: cook and kajiya honor it, whitted renders one */
RtVoid rtMaxDepth(RtInt depth);       /* whitted, cook: deepest secondary ray; kajiya: longest path. Default 8 */
RtVoid rtAmbient(RtColor ka);         /* ka of the shading formula. whitted: ka * albedo; cook: the same, scaled
                                         by occlusion when rtAmbientOcclusion is on; kajiya: ignored, the
                                         environment lights the scene instead */
RtVoid rtBackground(RtColor radiance); /* what a ray that leaves the scene returns, camera and secondary rays
                                          alike (so mirrors show it). whitted, cook: lights nothing; kajiya: a
                                          uniform environment light of this radiance. Default black */
RtVoid rtEnvironment(const char *file, RtFloat scale, RtFloat rotateY); /* a latitude-longitude map (PFM or HDR)
                                          in place of rtBackground, scaled, turned about +y by rotateY degrees;
                                          NULL removes it. whitted: the background by direction, lights nothing;
                                          cook: also a light on the diffuse lobe, one importance-sampled
                                          direction per hit, on top of the ambient; kajiya: the environment
                                          light, importance sampled (HW4) */
RtVoid rtCamera(RtPoint eye, RtPoint lookat, RtVector up, RtFloat fovy); /* fovy: full vertical field of view */

/* transforms ----------------------------------------------------------- */

RtVoid rtTransformBegin(void);
RtVoid rtTransformEnd(void);
RtVoid rtTranslate(RtFloat dx, RtFloat dy, RtFloat dz);
RtVoid rtRotate(RtFloat angle, RtFloat x, RtFloat y, RtFloat z);
RtVoid rtScale(RtFloat sx, RtFloat sy, RtFloat sz);
RtVoid rtConcatTransform(RtMatrix m); /* row vectors, p' = p M, translation in the bottom row, as in HW1 */

/* [HW3, extra credit] Motion blur. The shutter is open over time 0 to 1.
 * Transform calls between rtMotionBegin(t) and rtMotionEnd() set the pose
 * at time t and nothing else; a primitive drawn after two or more such
 * blocks moves between its poses, interpolated between the two nearest
 * (translation, rotation and scale separately, so it stays rigid). Calls
 * outside a block apply at every time; the poses go with the CTM
 * (rtTransformBegin/End save and restore them); one pose or none means no
 * motion. rtCamera inside a block keys the camera the same way. A renderer
 * with motion blur gives every pixel sample a time in [0, 1) from the
 * sampler (one dimension after the lens, drawn only when something
 * moves); one without it, and whitted always, renders everything at time
 * 0, the first pose. Lights do not move. */
RtVoid rtMotionBegin(RtFloat time);
RtVoid rtMotionEnd(void);

/* lights --------------------------------------------------------------- */

/* All three go through the CTM at the time of the call. */
RtVoid rtPointLight(RtPoint position, RtColor intensity);          /* W/sr: a facing surface at distance d receives I / d^2 */
RtVoid rtDirectionalLight(RtVector direction, RtColor irradiance);  /* the direction the light travels; W/m^2 on a facing surface */
RtVoid rtSpotLight(RtPoint position, RtVector direction, RtColor intensity, /* a point light confined to a cone: full I / d^2 */
                   RtFloat coneAngle, RtFloat penumbraAngle, RtFloat maxDistance); /* within coneAngle - penumbraAngle degrees of
                                      direction, fading to nothing at coneAngle, and fading to nothing by maxDistance (0: no limit).
                                      A local fill, for lighting a scene by hand (Whitted handout) */
/* Area lights need no call: every rtSphere or rtTriangleMesh drawn with a
 * material of nonzero emission is one light, whatever its triangle count
 * (the Whitted Illumination handout for whitted, the HW3 and HW4 handouts
 * for the rest). */

/* attributes ----------------------------------------------------------- */

/* Attributes stay set until changed, as RiColor did. */
RtVoid rtMaterial(const RtMaterial *m);                              /* copied now */
RtVoid rtSurface(RtShaderFn fn, const void *params, size_t nbytes);  /* NULL unbinds: the bound material, unchanged */
RtVoid rtCutout(RtCutoutFn fn, const void *params, size_t nbytes);   /* NULL unbinds: no cutout, solid everywhere */

/* primitives ----------------------------------------------------------- */

/* A sphere of the given radius centered at the CTM's origin, under the
 * whole CTM: a non-uniform scale makes it an ellipsoid. Intersect it in
 * object space, through the inverse CTM (Intersection handout). */
RtVoid rtSphere(RtFloat radius);

/* ntris triangles over nverts vertices, indices three per triangle,
 * counterclockwise seen from outside. N (unit vertex normals), st (two per
 * vertex, t = 0 at the top row of a texture) and Cs (a color per vertex,
 * interpolated, replacing the material's albedo) may be NULL. The arrays
 * are copied through the CTM now; the caller may free them. obj.h's
 * ObjDraw() calls this per group of a model. */
RtVoid rtTriangleMesh(RtInt nverts, const RtPoint P[], const RtNormal N[], const RtFloat st[], const RtColor Cs[],
                      RtInt ntris, const RtInt indices[]);

/* textures ------------------------------------------------------------- */

/* Load an image: PNG, JPEG, BMP or TGA (8-bit; srgb = 1 decodes the color
 * channels to linear, never the alpha) or PFM or Radiance HDR (linear
 * float, srgb ignored). Returns a handle >= 0, or -1 with a message on
 * stderr. Tries the path as given, then under textures/. No slot limit. */
RtInt rtMakeTexture(const char *file, RtInt srgb);

/* Bilinear lookup with repeat wrap; s runs left to right, t top to bottom
 * (t = 0 is the top row, as in HW1). rgba gets alpha 1 when the image has
 * no alpha channel. */
RtVoid rtTexture(RtInt handle, RtFloat s, RtFloat t, RtFloat rgba[4]);

/* world ---------------------------------------------------------------- */

/* A world: the lights and primitives drawn between rtWorldBegin and
 * rtWorldEnd, which renders them. frame is the world's number in a movie
 * (0 for a still). rtWorldBegin returns 1 if the frame is to be drawn and
 * 0 if --frames leaves it out, so a movie skips the frame's draw calls:
 *     if (rtWorldBegin(f)) { ...draw... rtWorldEnd(); }
 * A scene that draws anyway is rendered anyway. */
RtInt rtWorldBegin(RtInt frame);
RtVoid rtWorldEnd(void);  /* build the acceleration structure, render, write the image, print the TIME line */

/* [HW3] declared now, implemented in HW3 ------------------------------- */

RtVoid rtLens(RtFloat radius, RtFloat focusDistance);  /* thin lens; radius 0 = pinhole (default). whitted ignores it */
RtVoid rtSampler(const char *name);  /* "independent" (default), "stratified", "halton", "sobol". whitted ignores it */
RtVoid rtAmbientOcclusion(RtFloat maxDistance);  /* cook only: > 0 scales the ambient term by the occlusion within
                                                    this distance; 0 (default) = plain ambient. whitted and kajiya ignore it */
RtVoid rtEnvironmentPortal(const RtPoint corners[4]);  /* the opening the environment is seen through: a quad in world
                                                    space (through the CTM), its corners in order around it; a cook
                                                    or kajiya that samples rtEnvironment through it (extra credit
                                                    from HW3) wastes no light samples on the walls. NULL
                                                    removes it; none (default) samples the whole map. whitted
                                                    ignores it */

/* HW4 adds no calls: it adds kajiya, the path tracer, which gives
 * rtBackground, rtEnvironment and emissive materials their full meaning. */

#ifdef __cplusplus
}
#endif

#include "shaders.h" /* the scenes include only rt.h but bind the shaders */

#endif /* RT_H */
