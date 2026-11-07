/*
 * obj.h: a small Wavefront OBJ and MTL loader for the scene files, and
 * ObjDraw(), which hands a loaded model to the renderer group by group
 * (HW1's loader with materials added). Not part of the rt interface: the
 * renderer never calls it, and it is C++ because the scene files are.
 * obj_load() reads a model, ObjDraw() binds each group's material and
 * maps and draws it with rtTriangleMesh, ObjDrawInstances() draws the
 * placements listed in a model's instances.txt. A model's binary twin,
 * <name>.mesh (what the kit ships: the loader's output written once,
 * little-endian), is read by the same obj_load() by its extension.
 */
#ifndef OBJ_H
#define OBJ_H

#include "rt.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <tuple>
#include <vector>

typedef struct {
    char name[64];       /* from newmtl */
    RtColor Kd, Ks, Ke;  /* diffuse, specular, emission */
    RtFloat Ns;          /* specular exponent */
    RtFloat d;           /* opacity (Tr is read as 1 - d) */
    RtFloat Ni;          /* index of refraction */
    RtFloat Pm;          /* metallic, the MTL PBR extension's Pm: 0 (the default) to 1 */
    char map_Kd[256];    /* diffuse map path, relative to the OBJ's folder; "" if none */
    char map_d[256];     /* alpha mask path; "" if none */
    char map_Ke[256];    /* emission, metalness, roughness and normal maps: kept
                            for a scene's own shader, ObjDraw does not load them */
    char map_Pm[256];
    char map_Pr[256];
    char map_norm[256];  /* "norm": a tangent-space normal map */
    RtInt kdHandle;      /* texture handles once ObjDraw loaded them: -2 not yet, -1 none or failed */
    RtInt dHandle;
} ObjMaterial;

typedef struct {
    char name[64];     /* from g */
    char material[64]; /* from usemtl, "" if none */
    RtInt materialIndex; /* into ObjMesh::materials, or -1 */
    RtInt npolys;
    RtInt *nverts;     /* npolys entries */
    RtInt *verts;      /* sum(nverts) entries */
    RtInt nvertices;
    RtPoint *P;
    RtNormal *N;       /* NULL unless every corner in the group has a vn */
    RtFloat *st;       /* 2 * nvertices, NULL unless every corner has a vt */
    RtColor *Cs;       /* NULL unless every corner's v line carries a color */
} ObjGroup;

typedef struct {
    int ngroups;
    ObjGroup *groups;
    int nmaterials;
    ObjMaterial *materials;
    RtInt ntriangles; /* after fanning, over all groups */
} ObjMesh;

enum {
    OBJ_KEEP_MATERIAL = 1, /* ObjDraw: keep the scene's material and shaders for every group */
    OBJ_NO_TEXTURES = 2,   /* ObjDraw: set the materials but bind no maps */
    OBJ_COAT_MODE = 4      /* ObjDraw: the model was exported in the exporter's coat mode, its Ks the weight of
                              a Fresnel coat rather than the reflectance head on that specular is (rt.h):
                              scale it by the ior's F0 (lerped to 1 by Pm), which is what it meant */
};

namespace obj_detail {

struct Vec {
    float x, y, z;
};

/* A group while it is being read. */
struct Builder {
    std::string name, material;
    std::map<std::tuple<int, int, int>, RtInt> index; /* (v, vt, vn) -> output vertex */
    std::vector<Vec> P, N, C;
    std::vector<float> st;
    std::vector<RtInt> nverts, verts;
    bool allN = true, allST = true, allC = true;
};

inline void copyString(const std::string &from, char *to, size_t size)
{
    size_t n = from.copy(to, size - 1);
    to[n] = '\0';
}

/* The rest of the line after position `from`, without surrounding blanks. */
inline std::string rest(const std::string &line, size_t from)
{
    size_t b = line.find_first_not_of(" \t", from);
    if (b == std::string::npos)
        return "";
    size_t e = line.find_last_not_of(" \t");
    return line.substr(b, e - b + 1);
}

/* The folder part of a path, with its trailing slash ("" if none). */
inline std::string folderOf(const std::string &path)
{
    size_t slash = path.find_last_of("/\\");
    return slash == std::string::npos ? "" : path.substr(0, slash + 1);
}

/* An OBJ index (1-based, or negative counting back from the end) as a
 * 0-based one; -1 if absent or out of range. */
inline int resolve(long i, size_t count)
{
    long r = i > 0 ? i - 1 : (long)count + i;
    return (i != 0 && r >= 0 && r < (long)count) ? (int)r : -1;
}

inline ObjMaterial defaultMaterial(const std::string &name)
{
    ObjMaterial m;
    std::memset(&m, 0, sizeof m);
    copyString(name, m.name, sizeof m.name);
    m.Kd[0] = m.Kd[1] = m.Kd[2] = 0.8f;
    m.Ns = 10;
    m.d = 1;
    m.Ni = 1.5f;
    m.kdHandle = m.dHandle = -2;
    return m;
}

/* A map's file name from the rest of a map_Kd line: the last token, so
 * that options like "-bm 1" before it are skipped (a name with spaces is
 * not supported). Backslashes become slashes. */
inline std::string mapName(const std::string &args, const std::string &folder)
{
    size_t e = args.find_last_not_of(" \t");
    if (e == std::string::npos)
        return "";
    size_t b = args.find_last_of(" \t", e);
    std::string name = args.substr(b == std::string::npos ? 0 : b + 1, e - (b == std::string::npos ? 0 : b + 1) + 1);
    for (char &c : name)
        if (c == '\\')
            c = '/';
    return folder + name;
}

/* Read one MTL file into `materials` (appending). */
inline void loadMtl(const std::string &path, std::vector<ObjMaterial> &materials)
{
    std::ifstream file(path);
    if (!file) {
        std::fprintf(stderr, "obj_load: cannot open material file %s\n", path.c_str());
        return;
    }
    const std::string folder = folderOf(path);
    std::string line;
    while (std::getline(file, line)) {
        if (!line.empty() && line.back() == '\r')
            line.pop_back();
        size_t b = line.find_first_not_of(" \t");
        if (b == std::string::npos || line[b] == '#')
            continue;
        size_t e = line.find_first_of(" \t", b);
        std::string key = line.substr(b, e == std::string::npos ? std::string::npos : e - b);
        std::string args = rest(line, e == std::string::npos ? line.size() : e);
        const char *p = args.c_str();
        char *end;

        if (key == "newmtl") {
            materials.push_back(defaultMaterial(args));
            continue;
        }
        if (materials.empty())
            continue; /* a value before any newmtl */
        ObjMaterial &m = materials.back();
        if (key == "Kd" || key == "Ks" || key == "Ke") {
            RtFloat *c = key == "Kd" ? m.Kd : key == "Ks" ? m.Ks : m.Ke;
            c[0] = std::strtof(p, &end);
            c[1] = std::strtof(end, &end);
            c[2] = std::strtof(end, &end);
        } else if (key == "Ns") {
            m.Ns = std::strtof(p, &end);
        } else if (key == "d") {
            m.d = std::strtof(p, &end);
        } else if (key == "Tr") {
            m.d = 1 - std::strtof(p, &end);
        } else if (key == "Ni") {
            m.Ni = std::strtof(p, &end);
        } else if (key == "Pm") {
            m.Pm = std::strtof(p, &end);
        } else if (key == "map_Kd") {
            copyString(mapName(args, folder), m.map_Kd, sizeof m.map_Kd);
        } else if (key == "map_d") {
            copyString(mapName(args, folder), m.map_d, sizeof m.map_d);
        } else if (key == "map_Ke") {
            copyString(mapName(args, folder), m.map_Ke, sizeof m.map_Ke);
        } else if (key == "map_Pm") {
            copyString(mapName(args, folder), m.map_Pm, sizeof m.map_Pm);
        } else if (key == "map_Pr") {
            copyString(mapName(args, folder), m.map_Pr, sizeof m.map_Pr);
        } else if (key == "norm") {
            copyString(mapName(args, folder), m.map_norm, sizeof m.map_norm);
        }
    }
}

} // namespace obj_detail

/* The binary twin of an OBJ, models/<name>/<name>.mesh: the ObjMesh an
 * obj_load() of the OBJ produces, written field by field (little-endian
 * 32-bit ints and floats; the materials with their map paths, the groups
 * with their polygons and per-vertex arrays), so that loading it gives
 * the same mesh and the same image, in less than half the file and a
 * fraction of the time. The course writes it once from the OBJ; the kit
 * ships the .mesh files and obj_load() reads either by extension. */
namespace obj_detail {
inline bool endsWith(const std::string &s, const char *suffix)
{
    const size_t n = std::strlen(suffix);
    return s.size() >= n && s.compare(s.size() - n, n, suffix) == 0;
}
inline bool writeBytes(std::FILE *f, const void *p, size_t n) { return std::fwrite(p, 1, n, f) == n; }
inline bool readBytes(std::FILE *f, void *p, size_t n) { return std::fread(p, 1, n, f) == n; }
inline bool writeInt(std::FILE *f, RtInt v) { return writeBytes(f, &v, sizeof v); }
inline bool readInt(std::FILE *f, RtInt &v) { return readBytes(f, &v, sizeof v); }
/* A name or path field (at most 256 bytes) zeroed after its terminator,
 * so the file does not depend on what the rest of the array held. */
inline bool writeString(std::FILE *f, const char *s, size_t size)
{
    char buf[256];
    std::memset(buf, 0, sizeof buf);
    for (size_t i = 0; i + 1 < size && i + 1 < sizeof buf && s[i]; i++)
        buf[i] = s[i];
    return size <= sizeof buf && writeBytes(f, buf, size);
}
} // namespace obj_detail

inline void obj_free(ObjMesh *m);

inline bool mesh_save(const ObjMesh *m, const char *path)
{
    using namespace obj_detail;
    std::FILE *f = std::fopen(path, "wb");
    if (!f)
        return false;
    /* the sentinel after the magic reads as 0x01020304 on a little-endian
     * host, which every machine the course runs on is */
    bool ok = writeBytes(f, "GRTMESH1", 8) && writeInt(f, 0x01020304) && writeInt(f, m->nmaterials) && writeInt(f, m->ngroups) &&
              writeInt(f, m->ntriangles);
    for (int i = 0; ok && i < m->nmaterials; i++) {
        const ObjMaterial &mt = m->materials[i];
        ok = writeString(f, mt.name, sizeof mt.name) && writeBytes(f, mt.Kd, sizeof mt.Kd) && writeBytes(f, mt.Ks, sizeof mt.Ks) &&
             writeBytes(f, mt.Ke, sizeof mt.Ke) && writeBytes(f, &mt.Ns, sizeof mt.Ns) && writeBytes(f, &mt.d, sizeof mt.d) &&
             writeBytes(f, &mt.Ni, sizeof mt.Ni) && writeBytes(f, &mt.Pm, sizeof mt.Pm) && writeString(f, mt.map_Kd, sizeof mt.map_Kd) &&
             writeString(f, mt.map_d, sizeof mt.map_d) && writeString(f, mt.map_Ke, sizeof mt.map_Ke) &&
             writeString(f, mt.map_Pm, sizeof mt.map_Pm) && writeString(f, mt.map_Pr, sizeof mt.map_Pr) &&
             writeString(f, mt.map_norm, sizeof mt.map_norm);
    }
    for (int k = 0; ok && k < m->ngroups; k++) {
        const ObjGroup &g = m->groups[k];
        RtInt total = 0;
        for (int i = 0; i < g.npolys; i++)
            total += g.nverts[i];
        const RtInt flags = (g.N ? 1 : 0) | (g.st ? 2 : 0) | (g.Cs ? 4 : 0);
        ok = writeString(f, g.name, sizeof g.name) && writeString(f, g.material, sizeof g.material) && writeInt(f, g.materialIndex) &&
             writeInt(f, g.npolys) && writeInt(f, total) && writeInt(f, g.nvertices) && writeInt(f, flags) &&
             writeBytes(f, g.nverts, sizeof(RtInt) * (size_t)g.npolys) && writeBytes(f, g.verts, sizeof(RtInt) * (size_t)total) &&
             writeBytes(f, g.P, sizeof(RtPoint) * (size_t)g.nvertices) &&
             (!g.N || writeBytes(f, g.N, sizeof(RtNormal) * (size_t)g.nvertices)) &&
             (!g.st || writeBytes(f, g.st, sizeof(RtFloat) * 2 * (size_t)g.nvertices)) &&
             (!g.Cs || writeBytes(f, g.Cs, sizeof(RtColor) * (size_t)g.nvertices));
    }
    return std::fclose(f) == 0 && ok;
}

inline ObjMesh *mesh_load(const char *opath)
{
    char path[512];
    snprintf(path, sizeof(path), "assets/%s", opath);

    using namespace obj_detail;
    std::FILE *f = std::fopen(path, "rb");
    if (!f) {
        std::fprintf(stderr, "mesh_load: cannot open %s (run from the repo root)\n", path);
        return NULL;
    }
    char magic[8];
    RtInt sentinel = 0, nmaterials = 0, ngroups = 0, ntriangles = 0;
    if (!readBytes(f, magic, 8) || std::memcmp(magic, "GRTMESH1", 8) != 0 || !readInt(f, sentinel)) {
        std::fprintf(stderr, "mesh_load: %s is not a GRTMESH1 file\n", path);
        std::fclose(f);
        return NULL;
    }
    if (sentinel != 0x01020304) {
        std::fprintf(stderr, "mesh_load: %s is little-endian and this machine is not; the .mesh files need a build for it\n", path);
        std::fclose(f);
        return NULL;
    }
    if (!readInt(f, nmaterials) || !readInt(f, ngroups) || !readInt(f, ntriangles) || nmaterials < 0 || ngroups < 0) {
        std::fprintf(stderr, "mesh_load: %s is truncated\n", path);
        std::fclose(f);
        return NULL;
    }
    ObjMesh *mesh = new ObjMesh;
    mesh->nmaterials = nmaterials;
    mesh->ngroups = ngroups;
    mesh->ntriangles = ntriangles;
    mesh->materials = new ObjMaterial[(size_t)nmaterials + 1];
    mesh->groups = new ObjGroup[(size_t)ngroups + 1];
    for (int k = 0; k < ngroups; k++) {
        ObjGroup &g = mesh->groups[k];
        g.nverts = g.verts = NULL;
        g.P = NULL;
        g.N = NULL;
        g.st = NULL;
        g.Cs = NULL;
    }
    bool ok = true;
    for (int i = 0; ok && i < nmaterials; i++) {
        ObjMaterial &mt = mesh->materials[i];
        std::memset(&mt, 0, sizeof mt);
        ok = readBytes(f, mt.name, sizeof mt.name) && readBytes(f, mt.Kd, sizeof mt.Kd) && readBytes(f, mt.Ks, sizeof mt.Ks) &&
             readBytes(f, mt.Ke, sizeof mt.Ke) && readBytes(f, &mt.Ns, sizeof mt.Ns) && readBytes(f, &mt.d, sizeof mt.d) &&
             readBytes(f, &mt.Ni, sizeof mt.Ni) && readBytes(f, &mt.Pm, sizeof mt.Pm) && readBytes(f, mt.map_Kd, sizeof mt.map_Kd) &&
             readBytes(f, mt.map_d, sizeof mt.map_d) && readBytes(f, mt.map_Ke, sizeof mt.map_Ke) && readBytes(f, mt.map_Pm, sizeof mt.map_Pm) &&
             readBytes(f, mt.map_Pr, sizeof mt.map_Pr) && readBytes(f, mt.map_norm, sizeof mt.map_norm);
        mt.kdHandle = mt.dHandle = -2;
    }
    for (int k = 0; ok && k < ngroups; k++) {
        ObjGroup &g = mesh->groups[k];
        RtInt total = 0, flags = 0;
        ok = readBytes(f, g.name, sizeof g.name) && readBytes(f, g.material, sizeof g.material) && readInt(f, g.materialIndex) &&
             readInt(f, g.npolys) && readInt(f, total) && readInt(f, g.nvertices) && readInt(f, flags) && g.npolys >= 0 && total >= 0 &&
             g.nvertices >= 0;
        if (!ok)
            break;
        g.nverts = new RtInt[(size_t)g.npolys + 1];
        g.verts = new RtInt[(size_t)total + 1];
        g.P = new RtPoint[(size_t)g.nvertices + 1];
        g.N = (flags & 1) ? new RtNormal[(size_t)g.nvertices + 1] : NULL;
        g.st = (flags & 2) ? new RtFloat[2 * (size_t)g.nvertices + 1] : NULL;
        g.Cs = (flags & 4) ? new RtColor[(size_t)g.nvertices + 1] : NULL;
        ok = readBytes(f, g.nverts, sizeof(RtInt) * (size_t)g.npolys) && readBytes(f, g.verts, sizeof(RtInt) * (size_t)total) &&
             readBytes(f, g.P, sizeof(RtPoint) * (size_t)g.nvertices) &&
             (!g.N || readBytes(f, g.N, sizeof(RtNormal) * (size_t)g.nvertices)) &&
             (!g.st || readBytes(f, g.st, sizeof(RtFloat) * 2 * (size_t)g.nvertices)) &&
             (!g.Cs || readBytes(f, g.Cs, sizeof(RtColor) * (size_t)g.nvertices));
    }
    std::fclose(f);
    if (!ok) {
        std::fprintf(stderr, "mesh_load: %s is truncated or damaged\n", path);
        obj_free(mesh);
        return NULL;
    }
    return mesh;
}

/* Read an OBJ: v (with an optional r g b vertex color), vt, vn, f (any
 * corner count, v, v/vt, v//vn or v/vt/vn, negative indices allowed), g,
 * usemtl and mtllib; the rest is ignored. MTL (from the OBJ's folder):
 * newmtl, Kd, Ks, Ns, d, Tr, Ni, Ke, Pm, map_Kd, map_d, and the paths of
 * map_Ke, map_Pm, map_Pr, norm (for a scene's shader). Every distinct
 * (v, vt, vn) triple is one output vertex; a group starts at each g or
 * usemtl line; st holds (s, 1 - t), since OBJ's t = 0 is the bottom row.
 * A path ending in .mesh is read by mesh_load instead. NULL plus a
 * message on stderr on failure. */
inline ObjMesh *obj_load(const char *opath)
{
    using namespace obj_detail;
    if (endsWith(opath, ".mesh"))
        return mesh_load(opath);
        
    char path[512];
    snprintf(path, sizeof(path), "assets/%s", opath);
    
    std::ifstream file(path);
    if (!file) {
        std::fprintf(stderr, "obj_load: cannot open %s (run from the repo root)\n", path);
        return NULL;
    }
    const std::string folder = folderOf(path);

    std::vector<Vec> v, vn, vc; /* vc: a color per v, or (-1, -1, -1) when the line had none */
    std::vector<float> vt;
    std::vector<Builder> groups(1);
    std::vector<ObjMaterial> materials;
    std::string line, name, material;
    int lineno = 0;

    while (std::getline(file, line)) {
        lineno++;
        if (!line.empty() && line.back() == '\r')
            line.pop_back();
        size_t b = line.find_first_not_of(" \t");
        if (b == std::string::npos || line[b] == '#')
            continue;
        size_t e = line.find_first_of(" \t", b);
        std::string key = line.substr(b, e == std::string::npos ? std::string::npos : e - b);
        const char *p = line.c_str() + (e == std::string::npos ? line.size() : e);
        char *end;

        if (key == "v" || key == "vn") {
            Vec a;
            a.x = std::strtof(p, &end);
            a.y = std::strtof(end, &end);
            a.z = std::strtof(end, &end);
            (key == "v" ? v : vn).push_back(a);
            if (key == "v") {
                /* v x y z r g b: a vertex color, if three more numbers follow */
                Vec c = {-1, -1, -1};
                const char *q = end;
                char *e2;
                float red = std::strtof(q, &e2);
                if (e2 != q) {
                    q = e2;
                    float green = std::strtof(q, &e2);
                    if (e2 != q) {
                        q = e2;
                        float blue = std::strtof(q, &e2);
                        if (e2 != q)
                            c = Vec{red, green, blue};
                    }
                }
                vc.push_back(c);
            }
        } else if (key == "vt") {
            float s = std::strtof(p, &end), t = std::strtof(end, &end);
            vt.push_back(s);
            vt.push_back(1 - t); /* OBJ's t runs up the image, ours down */
        } else if (key == "g" || key == "usemtl") {
            (key == "g" ? name : material) = rest(line, e == std::string::npos ? line.size() : e);
            groups.emplace_back();
            groups.back().name = name;
            groups.back().material = material;
        } else if (key == "mtllib") {
            std::string lib = rest(line, e == std::string::npos ? line.size() : e);
            loadMtl(folder + lib, materials);
        } else if (key == "f") {
            Builder &g = groups.back();
            RtInt corners = 0;
            while (true) {
                /* one corner: v, v/vt, v//vn or v/vt/vn */
                long iv = std::strtol(p, &end, 10), ivt = 0, ivn = 0;
                if (end == p)
                    break; /* no more corners */
                p = end;
                if (*p == '/') {
                    p++;
                    if (*p != '/') {
                        ivt = std::strtol(p, &end, 10);
                        p = end;
                    }
                    if (*p == '/') {
                        ivn = std::strtol(p + 1, &end, 10);
                        p = end;
                    }
                }
                int a = resolve(iv, v.size()), t = resolve(ivt, vt.size() / 2);
                int n = resolve(ivn, vn.size());
                if (a < 0 || (ivt != 0 && t < 0) || (ivn != 0 && n < 0)) {
                    std::fprintf(stderr, "obj_load: %s:%d: index out of range\n", path, lineno);
                    return NULL;
                }
                g.allST = g.allST && t >= 0;
                g.allN = g.allN && n >= 0;
                g.allC = g.allC && vc[a].x >= 0;

                auto key3 = std::make_tuple(a, t, n);
                auto found = g.index.find(key3);
                RtInt out;
                if (found != g.index.end()) {
                    out = found->second;
                } else {
                    out = (RtInt)g.P.size();
                    g.index[key3] = out;
                    g.P.push_back(v[a]);
                    g.N.push_back(n >= 0 ? vn[n] : Vec{0, 0, 0});
                    g.C.push_back(vc[a]);
                    g.st.push_back(t >= 0 ? vt[2 * t] : 0);
                    g.st.push_back(t >= 0 ? vt[2 * t + 1] : 0);
                }
                g.verts.push_back(out);
                corners++;
            }
            if (corners < 3) {
                std::fprintf(stderr, "obj_load: %s:%d: a face needs 3 corners\n", path, lineno);
                return NULL;
            }
            g.nverts.push_back(corners);
        }
    }

    ObjMesh *mesh = new ObjMesh;
    mesh->ngroups = 0;
    mesh->ntriangles = 0;
    for (const Builder &g : groups)
        mesh->ngroups += !g.nverts.empty();
    mesh->groups = new ObjGroup[mesh->ngroups];
    mesh->nmaterials = (int)materials.size();
    mesh->materials = new ObjMaterial[materials.size() + 1]; /* + 1: never a zero-length array */
    for (size_t i = 0; i < materials.size(); i++)
        mesh->materials[i] = materials[i];

    int k = 0;
    for (const Builder &g : groups) {
        if (g.nverts.empty())
            continue;
        ObjGroup &o = mesh->groups[k++];
        copyString(g.name, o.name, sizeof o.name);
        copyString(g.material, o.material, sizeof o.material);
        o.materialIndex = -1;
        for (int i = 0; i < mesh->nmaterials; i++)
            if (g.material == mesh->materials[i].name) {
                o.materialIndex = i;
                break;
            }
        o.npolys = (RtInt)g.nverts.size();
        o.nverts = new RtInt[g.nverts.size()];
        o.verts = new RtInt[g.verts.size()];
        o.nvertices = (RtInt)g.P.size();
        o.P = new RtPoint[g.P.size()];
        o.N = g.allN ? new RtNormal[g.P.size()] : NULL;
        o.st = g.allST ? new RtFloat[g.st.size()] : NULL;
        o.Cs = g.allC ? new RtColor[g.P.size()] : NULL;
        for (size_t i = 0; i < g.nverts.size(); i++) {
            o.nverts[i] = g.nverts[i];
            mesh->ntriangles += g.nverts[i] - 2;
        }
        for (size_t i = 0; i < g.verts.size(); i++)
            o.verts[i] = g.verts[i];
        for (size_t i = 0; i < g.P.size(); i++) {
            o.P[i][0] = g.P[i].x;
            o.P[i][1] = g.P[i].y;
            o.P[i][2] = g.P[i].z;
            if (o.N) {
                o.N[i][0] = g.N[i].x;
                o.N[i][1] = g.N[i].y;
                o.N[i][2] = g.N[i].z;
            }
            if (o.Cs) {
                o.Cs[i][0] = g.C[i].x;
                o.Cs[i][1] = g.C[i].y;
                o.Cs[i][2] = g.C[i].z;
            }
        }
        if (o.st)
            for (size_t i = 0; i < g.st.size(); i++)
                o.st[i] = g.st[i];
    }
    return mesh;
}

inline void obj_free(ObjMesh *m)
{
    if (!m)
        return;
    for (int k = 0; k < m->ngroups; k++) {
        ObjGroup &g = m->groups[k];
        delete[] g.nverts;
        delete[] g.verts;
        delete[] g.P;
        delete[] g.N;
        delete[] g.st;
        delete[] g.Cs;
    }
    delete[] m->groups;
    delete[] m->materials;
    delete m;
}

/* The RtMaterial for an MTL material: albedo = Kd, specular = Ks,
 * roughness = (2 / (Ns + 2))^(1/4) (the Phong exponent n = 2 / alpha^2 - 2
 * of the Whitted model inverted, alpha = roughness^2), metallic = Pm,
 * transmission = 1 - d, ior = Ni, emission = Ke. */
inline RtMaterial ObjMaterialToRt(const ObjMaterial &om)
{
    RtMaterial m;
    for (int i = 0; i < 3; i++) {
        m.albedo[i] = om.Kd[i];
        m.specular[i] = om.Ks[i];
        m.emission[i] = om.Ke[i];
    }
    m.roughness = std::min(1.0f, std::sqrt(std::sqrt(2.0f / (std::max(0.0f, om.Ns) + 2.0f))));
    m.metallic = std::min(1.0f, std::max(0.0f, om.Pm)); /* whitted and cook ignore it; kajiya's metals need it */
    m.transmission = std::min(1.0f, std::max(0.0f, 1.0f - om.d));
    m.ior = om.Ni > 0 ? om.Ni : 1.5f;
    return m;
}

/* A coat-mode material's Ks as the reflectance head on: Ks times
 * F0(ior) (1 - Pm) + Pm. The five HW4 hero models were exported so;
 * nothing new is. */
inline void ObjCoatModeToF0(RtMaterial &m)
{
    const float r = (m.ior - 1) / (m.ior + 1);
    const float k = r * r * (1 - m.metallic) + m.metallic;
    for (int i = 0; i < 3; i++)
        m.specular[i] *= k;
}

/* A scene's hook into ObjDraw: called once per group, after its material
 * and maps are bound and before its triangles are drawn, with the MTL
 * material name ("" for none), so the scene can bind its own shader for
 * chosen names. Whatever it binds stays bound until the next group. */
typedef void (*ObjMaterialHook)(const char *material, void *user);

/* Draw every group of the mesh under the current transformation: fan each
 * polygon into triangles and call rtTriangleMesh once per group after
 * binding its material, a TEXTURED surface shader when
 * it has a map_Kd or map_d, and an ALPHA_CUTOUT cutout when it has a
 * map_d. Vertex colors override Kd. Handles are loaded the first time a
 * material is drawn and kept in the mesh; they belong to one rtBegin/End
 * session, so load the model inside the session that draws it. On return
 * the shaders are unbound and the last group's material stays bound. */
inline void ObjDraw(ObjMesh *mesh, int flags = 0, ObjMaterialHook hook = NULL, void *user = NULL)
{
    if (!mesh)
        return;
    std::vector<RtInt> indices;
    for (int k = 0; k < mesh->ngroups; k++) {
        const ObjGroup &g = mesh->groups[k];

        if (!(flags & OBJ_KEEP_MATERIAL)) {
            ObjMaterial *om = g.materialIndex >= 0 ? &mesh->materials[g.materialIndex] : NULL;
            RtMaterial m = om ? ObjMaterialToRt(*om) : ObjMaterialToRt(obj_detail::defaultMaterial(""));
            if (flags & OBJ_COAT_MODE)
                ObjCoatModeToF0(m);
            rtMaterial(&m);
            bool maps = om && !(flags & OBJ_NO_TEXTURES) && (om->map_Kd[0] || om->map_d[0]);
            if (maps) {
                if (om->kdHandle == -2)
                    om->kdHandle = om->map_Kd[0] ? rtMakeTexture(om->map_Kd, 1) : -1;
                if (om->dHandle == -2)
                    om->dHandle = om->map_d[0] ? rtMakeTexture(om->map_d, 0) : -1;
                TexturedParams tp = {om->kdHandle, om->dHandle};
                rtSurface(TEXTURED, &tp, sizeof tp);
                if (om->dHandle >= 0) {
                    CutoutParams cp = {tp, 0.5f};
                    rtCutout(ALPHA_CUTOUT, &cp, sizeof cp);
                } else {
                    rtCutout(NULL, NULL, 0);
                }
            } else {
                rtSurface(NULL, NULL, 0);
                rtCutout(NULL, NULL, 0);
            }
        }
        if (hook)
            hook(g.material, user);

        /* Fan each polygon from its first corner. */
        indices.clear();
        const RtInt *corner = g.verts;
        for (RtInt p = 0; p < g.npolys; p++) {
            for (RtInt i = 1; i + 1 < g.nverts[p]; i++) {
                indices.push_back(corner[0]);
                indices.push_back(corner[i]);
                indices.push_back(corner[i + 1]);
            }
            corner += g.nverts[p];
        }
        rtTriangleMesh(g.nvertices, g.P, g.N, g.st, g.Cs, (RtInt)(indices.size() / 3), indices.data());
    }
    if (!(flags & OBJ_KEEP_MATERIAL)) {
        rtSurface(NULL, NULL, 0);
        rtCutout(NULL, NULL, 0);
    }
}

/* Draw the instances listed in a text file (a model's instances.txt):
 * per line a model's file name (.mesh or .obj) in the file's folder and
 * the twelve numbers of a 4 x 3 transformation in rtConcatTransform's
 * row-vector convention. Each model is loaded once and drawn once per line under the current
 * transformation; # starts a comment. Returns the count drawn. */
inline int ObjDrawInstances(const char *path, int flags = 0, ObjMaterialHook hook = NULL, void *user = NULL)
{
    std::ifstream file(path);
    if (!file) {
        std::fprintf(stderr, "ObjDrawInstances: cannot open %s\n", path);
        return 0;
    }
    const std::string folder = obj_detail::folderOf(path);
    std::map<std::string, ObjMesh *> loaded;
    std::string line, name;
    int count = 0;
    while (std::getline(file, line)) {
        if (line.empty() || line[0] == '#')
            continue;
        std::istringstream in(line);
        RtMatrix m = {{1, 0, 0, 0}, {0, 1, 0, 0}, {0, 0, 1, 0}, {0, 0, 0, 1}};
        in >> name;
        for (int r = 0; r < 4 && in; r++)
            for (int c = 0; c < 3; c++)
                in >> m[r][c];
        if (!in)
            continue;
        ObjMesh *&mesh = loaded[name];
        if (!mesh)
            mesh = obj_load((folder + name).c_str());
        if (!mesh)
            continue;
        rtTransformBegin();
            rtConcatTransform(m);
            ObjDraw(mesh, flags, hook, user);
        rtTransformEnd();
        count++;
    }
    for (std::map<std::string, ObjMesh *>::iterator it = loaded.begin(); it != loaded.end(); it++)
        obj_free(it->second);
    return count;
}

#endif /* OBJ_H */
