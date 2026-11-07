/*
 * ECE/CS 285 – HW2 Test Scene: CornellBox
 * The Cornell box: a white room with a red and a green wall and two
 * white blocks, all matte, lit by an emissive quad below the ceiling, no
 * ambient. Under whitted the quad is one point light at its center,
 * hard shadows; under cook its area is sampled, soft shadows; under
 * kajiya the light bounces, a lit ceiling and color
 * bleeding. 720 x 720, cropped to the room, 1024 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/CornellBox.png
 */

#include "rt.h"
#include "sceneutil.h"

/* The room both Cornell boxes share: 2 m on a side, (-1, 0, -1) to
 * (1, 2, 1), open toward +z, matte walls with normals pointing in (red
 * left, green right, white elsewhere); the light a 0.75 m Quad 1 mm below
 * the ceiling, facing down, radiance 64 / 9, so Le A = 4 W/sr. */
static void CornellRoom(void)
{
    const float Le = 64.0f / 9.0f;
    RtMaterial light = EMITTER(Le, Le, Le);
    RtMaterial white = MATTE(0.75f, 0.75f, 0.75f);
    RtMaterial red = MATTE(0.65f, 0.06f, 0.05f);
    RtMaterial green = MATTE(0.12f, 0.45f, 0.1f);

    rtMaterial(&light);
    Quad(Vec3(-0.375f, 1.999f, 0.375f), Vec3(0.75f, 0, 0), Vec3(0, 0, -0.75f)); /* facing down */

    rtMaterial(&white);
    Quad(Vec3(-1, 0, -1), Vec3(2, 0, 0), Vec3(0, 0, 2));   /* floor, facing up */
    Quad(Vec3(-1, 2, 1), Vec3(2, 0, 0), Vec3(0, 0, -2));   /* ceiling, facing down */
    Quad(Vec3(-1, 2, -1), Vec3(2, 0, 0), Vec3(0, -2, 0));  /* back wall, facing +z */
    rtMaterial(&red);
    Quad(Vec3(-1, 2, 1), Vec3(0, 0, -2), Vec3(0, -2, 0));  /* left wall, facing +x */
    rtMaterial(&green);
    Quad(Vec3(1, 2, -1), Vec3(0, 0, 2), Vec3(0, -2, 0));   /* right wall, facing -x */
}

void CornellBox(void)
{
    RtPoint eye = {0, 1, 3.9f}, lookat = {0, 1, 0};
    RtVector up = {0, 1, 0};
    RtColor ka = {0, 0, 0}; /* no ambient: only the light lights the room */
    RtMaterial white = MATTE(0.75f, 0.75f, 0.75f);

    rtBegin();
        rtFormat(720, 720); /* square, cropped to the room: fovy 38 at 2.9 m sees its 2 m opening */
        rtDisplay("1_TestScenes/CornellBox");
        rtCamera(eye, lookat, up, 38);
        rtPixelSamples(1024); /* [HW3] cook and kajiya; whitted renders one sample */
        rtSampler("stratified");
        rtAmbient(ka);
        rtWorldBegin(0);
            CornellRoom();

            /* The two blocks, 0.6 m square, standing on the floor: the
             * tall one (1.2 m) at the back left turned 17 degrees, the
             * short one (0.6 m) at the front right turned -18. */
            rtMaterial(&white);
            rtTransformBegin();
                rtTranslate(-0.33f, 0, -0.32f);
                rtRotate(17, 0, 1, 0);
                Box(Vec3(-0.3f, 0, -0.3f), Vec3(0.3f, 1.2f, 0.3f));
            rtTransformEnd();
            rtTransformBegin();
                rtTranslate(0.37f, 0, 0.4f);
                rtRotate(-18, 0, 1, 0);
                Box(Vec3(-0.3f, 0, -0.3f), Vec3(0.3f, 0.6f, 0.3f));
            rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}

/*
 * ECE/CS 285 – HW2 Test Scene: CornellBoxSpheres
 * The room of CornellBox with a glass and a mirror sphere in place of the
 * blocks: reflection and refraction in a closed room, the light and the
 * walls seen in the mirror ball. Under cook the shadows soften; under
 * kajiya the walls bleed onto the floor and the glass ball focuses the
 * light into a caustic. 720 x 720, 1024 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/CornellBoxSpheres.png
 */
void CornellBoxSpheres(void)
{
    RtPoint eye = {0, 1, 3.9f}, lookat = {0, 1, 0};
    RtVector up = {0, 1, 0};
    RtColor ka = {0, 0, 0}; /* no ambient, as in CornellBox */
    RtMaterial leftBall = GLASS, rightBall = MIRROR;

    rtBegin();
        rtFormat(720, 720); /* square, cropped to the room, as CornellBox */
        rtDisplay("1_TestScenes/CornellBoxSpheres");
        rtCamera(eye, lookat, up, 38);
        rtPixelSamples(1024); /* [HW3] cook and kajiya; whitted renders one sample */
        rtSampler("stratified");
        rtAmbient(ka);
        rtWorldBegin(0);
            CornellRoom();

            rtMaterial(&leftBall);
            rtTransformBegin(); rtTranslate(-0.45f, 0.35f, 0.3f); rtSphere(0.35f); rtTransformEnd();
            rtMaterial(&rightBall);
            rtTransformBegin(); rtTranslate(0.45f, 0.35f, -0.3f); rtSphere(0.35f); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
