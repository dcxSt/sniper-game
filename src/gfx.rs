use crate::math::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

pub type Col = [f32; 4];

/// pos(3) normal(3) color(4)
pub const FLOATS: usize = 10;

#[derive(Default, Clone)]
pub struct Mesh {
    pub v: Vec<f32>,
}

impl Mesh {
    pub fn clear(&mut self) {
        self.v.clear();
    }

    #[inline]
    pub fn vert(&mut self, p: V3, n: V3, c: Col) {
        self.v
            .extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, c[0], c[1], c[2], c[3]]);
    }

    pub fn tri_n(&mut self, a: V3, b: V3, c: V3, n: V3, col: Col) {
        self.vert(a, n, col);
        self.vert(b, n, col);
        self.vert(c, n, col);
    }

    pub fn quad_n(&mut self, a: V3, b: V3, c: V3, d: V3, n: V3, col: Col) {
        self.tri_n(a, b, c, n, col);
        self.tri_n(a, c, d, n, col);
    }

    /// Per-corner coloured quad (a, b bottom; c, d top for walls).
    fn quad_gc(&mut self, p: [V3; 4], n: V3, c: [Col; 4]) {
        self.vert(p[0], n, c[0]);
        self.vert(p[1], n, c[1]);
        self.vert(p[2], n, c[2]);
        self.vert(p[0], n, c[0]);
        self.vert(p[2], n, c[2]);
        self.vert(p[3], n, c[3]);
    }

    pub fn cuboid(&mut self, min: V3, max: V3, col: Col) {
        let (x0, y0, z0) = (min.x, min.y, min.z);
        let (x1, y1, z1) = (max.x, max.y, max.z);
        // cheap ambient occlusion: the foot of a tall wall is a little darker than its top
        let h = y1 - y0;
        let k = if col[3] > 0.5 { 1.0 } else if h > 1.2 { 0.80 } else if h > 0.5 { 0.9 } else { 1.0 };
        let lo = [col[0] * k, col[1] * k, col[2] * k, col[3]];
        // +z
        self.quad_gc(
            [v3(x0, y0, z1), v3(x1, y0, z1), v3(x1, y1, z1), v3(x0, y1, z1)],
            v3(0.0, 0.0, 1.0),
            [lo, lo, col, col],
        );
        // -z
        self.quad_gc(
            [v3(x1, y0, z0), v3(x0, y0, z0), v3(x0, y1, z0), v3(x1, y1, z0)],
            v3(0.0, 0.0, -1.0),
            [lo, lo, col, col],
        );
        // +x
        self.quad_gc(
            [v3(x1, y0, z1), v3(x1, y0, z0), v3(x1, y1, z0), v3(x1, y1, z1)],
            v3(1.0, 0.0, 0.0),
            [lo, lo, col, col],
        );
        // -x
        self.quad_gc(
            [v3(x0, y0, z0), v3(x0, y0, z1), v3(x0, y1, z1), v3(x0, y1, z0)],
            v3(-1.0, 0.0, 0.0),
            [lo, lo, col, col],
        );
        // +y
        self.quad_n(
            v3(x0, y1, z1),
            v3(x1, y1, z1),
            v3(x1, y1, z0),
            v3(x0, y1, z0),
            v3(0.0, 1.0, 0.0),
            col,
        );
        // -y
        self.quad_n(
            v3(x0, y0, z0),
            v3(x1, y0, z0),
            v3(x1, y0, z1),
            v3(x0, y0, z1),
            v3(0.0, -1.0, 0.0),
            lo,
        );
    }

    /// Box centered at `c` with half-extents `h`.
    pub fn cuboid_c(&mut self, c: V3, h: V3, col: Col) {
        self.cuboid(c - h, c + h, col);
    }

    /// Thin round "bone" from a to b: a smooth-shaded 6-sided prism with flat end caps.
    pub fn limb(&mut self, a: V3, b: V3, t: f32, col: Col) {
        let d = b - a;
        let l = d.len();
        if l < 1e-5 {
            return;
        }
        let dn = d / l;
        let up = if dn.y.abs() < 0.95 { V3::Y } else { v3(1.0, 0.0, 0.0) };
        let u = dn.cross(up).norm();
        let w = dn.cross(u).norm();
        const N: usize = 6;
        let r = t * 0.56;
        let ring = |i: usize| -> V3 {
            let ang = std::f32::consts::TAU * i as f32 / N as f32;
            u * ang.cos() + w * ang.sin()
        };
        for i in 0..N {
            let (n0, n1) = (ring(i), ring(i + 1));
            let (p0, p1) = (a + n0 * r, a + n1 * r);
            let (q0, q1) = (b + n0 * r, b + n1 * r);
            self.vert(p0, n0, col);
            self.vert(p1, n1, col);
            self.vert(q1, n1, col);
            self.vert(p0, n0, col);
            self.vert(q1, n1, col);
            self.vert(q0, n0, col);
            self.tri_n(b, q0, q1, dn, col);
            self.tri_n(a, p1, p0, -dn, col);
        }
    }

    pub fn ellipsoid(&mut self, c: V3, r: V3, col: Col, stacks: usize, slices: usize) {
        let pt = |i: usize, j: usize| -> (V3, V3) {
            let th = std::f32::consts::PI * i as f32 / stacks as f32;
            let ph = std::f32::consts::TAU * j as f32 / slices as f32;
            let d = v3(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin());
            let n = v3(d.x / r.x.max(1e-4), d.y / r.y.max(1e-4), d.z / r.z.max(1e-4)).norm();
            (c + v3(d.x * r.x, d.y * r.y, d.z * r.z), n)
        };
        for i in 0..stacks {
            for j in 0..slices {
                let (p00, n00) = pt(i, j);
                let (p01, n01) = pt(i, j + 1);
                let (p10, n10) = pt(i + 1, j);
                let (p11, n11) = pt(i + 1, j + 1);
                self.vert(p00, n00, col);
                self.vert(p10, n10, col);
                self.vert(p11, n11, col);
                self.vert(p00, n00, col);
                self.vert(p11, n11, col);
                self.vert(p01, n01, col);
            }
        }
    }

    /// Soft cone: smooth normals all round, so it reads as one calm form rather than facets.
    pub fn cone_smooth(&mut self, base: V3, r: f32, h: f32, col: Col, seg: usize) {
        let apex = base + v3(0.0, h, 0.0);
        let n_at = |a: f32| v3(a.cos() * h, r, a.sin() * h).norm();
        for i in 0..seg {
            let a0 = std::f32::consts::TAU * i as f32 / seg as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / seg as f32;
            let am = (a0 + a1) * 0.5;
            let p0 = base + v3(a0.cos() * r, 0.0, a0.sin() * r);
            let p1 = base + v3(a1.cos() * r, 0.0, a1.sin() * r);
            self.vert(p0, n_at(a0), col);
            self.vert(apex, n_at(am), col);
            self.vert(p1, n_at(a1), col);
        }
    }

    pub fn cylinder(&mut self, base: V3, r: f32, h: f32, col: Col, seg: usize) {
        let top = v3(0.0, h, 0.0);
        for i in 0..seg {
            let a0 = std::f32::consts::TAU * i as f32 / seg as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / seg as f32;
            let p0 = base + v3(a0.cos() * r, 0.0, a0.sin() * r);
            let p1 = base + v3(a1.cos() * r, 0.0, a1.sin() * r);
            let am = (a0 + a1) * 0.5;
            let n = v3(am.cos(), 0.0, am.sin());
            self.quad_n(p0, p1, p1 + top, p0 + top, n, col);
            self.tri_n(base + top, p0 + top, p1 + top, V3::Y, col);
        }
    }

    pub fn billboard(&mut self, c: V3, right: V3, up: V3, hw: f32, hh: f32, col: Col) {
        let r = right * hw;
        let u = up * hh;
        self.quad_n(c - r - u, c + r - u, c + r + u, c - r + u, V3::Y, col);
    }

    /// Soft contact shadow: darkest in the middle, fading to nothing at the rim.
    pub fn blob(&mut self, c: V3, r: f32, alpha: f32) {
        let seg = 16;
        for i in 0..seg {
            let a0 = std::f32::consts::TAU * i as f32 / seg as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / seg as f32;
            self.vert(c, V3::Y, [0.0, 0.0, 0.0, alpha]);
            self.vert(c + v3(a0.cos() * r, 0.0, a0.sin() * r), V3::Y, [0.0, 0.0, 0.0, 0.0]);
            self.vert(c + v3(a1.cos() * r, 0.0, a1.sin() * r), V3::Y, [0.0, 0.0, 0.0, 0.0]);
        }
    }

    /// Soft round glow: opaque-ish centre fading to nothing at the rim (use with additive blending).
    pub fn glow(&mut self, c: V3, right: V3, up: V3, r: f32, col: Col) {
        let seg = 12;
        let rim = [col[0], col[1], col[2], 0.0];
        for i in 0..seg {
            let a0 = std::f32::consts::TAU * i as f32 / seg as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / seg as f32;
            let p0 = c + right * (a0.cos() * r) + up * (a0.sin() * r);
            let p1 = c + right * (a1.cos() * r) + up * (a1.sin() * r);
            self.vert(c, V3::Y, col);
            self.vert(p0, V3::Y, rim);
            self.vert(p1, V3::Y, rim);
        }
    }

    /// One chain-link fence panel between two ground points. The lattice itself is drawn by the
    /// fence shader; `voff`/`uoff` say how far the panel has slid (vertically / along its run).
    pub fn fence_quad(&mut self, a: V3, b: V3, y0: f32, y1: f32, voff: f32, uoff: f32) {
        let d = b - a;
        let n = v3(-d.z, 0.0, d.x).norm();
        let col = [voff, uoff, 0.0, 1.0];
        self.quad_n(v3(a.x, y0, a.z), v3(b.x, y0, b.z), v3(b.x, y1, b.z), v3(a.x, y1, a.z), n, col);
    }

    /// Camera-facing ribbon between two points (tracers, beams).
    pub fn ribbon(&mut self, a: V3, b: V3, w: f32, cam: V3, col: Col) {
        let d = b - a;
        let mid = (a + b) * 0.5;
        let side = d.cross(cam - mid).norm() * (w * 0.5);
        self.quad_n(a - side, a + side, b + side, b - side, V3::Y, col);
    }

    /// Triangle with its normal worked out from the winding (counter-clockwise seen from the front).
    pub fn tri(&mut self, a: V3, b: V3, c: V3, col: Col) {
        let n = (b - a).cross(c - a).norm();
        self.tri_n(a, b, c, n, col);
    }

    /// A convex footprint (any winding) extruded from y0 to y1, with the same baked wall shading as `cuboid`.
    /// `top_y` lets the top be sloped (ramps): it gives the height of the top at each footprint corner.
    pub fn prism_h(&mut self, pts: &[(f32, f32)], y0: f32, top_y: &dyn Fn(f32, f32) -> f32, col: Col, cap_bottom: bool) {
        let n = pts.len();
        if n < 3 {
            return;
        }
        let ccw = crate::world::poly_area2(pts) >= 0.0;
        let p = |i: usize| -> (f32, f32) { if ccw { pts[i % n] } else { pts[(n - 1 - i % n) % n] } };
        let mut hmax = y0;
        for &(x, z) in pts {
            hmax = hmax.max(top_y(x, z));
        }
        let h = hmax - y0;
        let k = if col[3] > 0.5 { 1.0 } else if h > 1.2 { 0.80 } else if h > 0.5 { 0.9 } else { 1.0 };
        let lo = [col[0] * k, col[1] * k, col[2] * k, col[3]];
        for i in 0..n {
            let (a, b) = (p(i), p(i + 1));
            let (dx, dz) = (b.0 - a.0, b.1 - a.1);
            let l = (dx * dx + dz * dz).sqrt();
            if l < 1e-5 {
                continue;
            }
            let nn = v3(dz / l, 0.0, -dx / l);
            let (ta, tb) = (top_y(a.0, a.1), top_y(b.0, b.1));
            self.quad_gc(
                [v3(b.0, y0, b.1), v3(a.0, y0, a.1), v3(a.0, ta, a.1), v3(b.0, tb, b.1)],
                nn,
                [lo, lo, col, col],
            );
        }
        // top: a fan (convex), normal from the actual plane
        let c0 = p(0);
        for i in 1..n - 1 {
            let (b, c) = (p(i), p(i + 1));
            let (pa, pb, pc) = (v3(c0.0, top_y(c0.0, c0.1), c0.1), v3(b.0, top_y(b.0, b.1), b.1), v3(c.0, top_y(c.0, c.1), c.1));
            let mut nn = (pc - pa).cross(pb - pa).norm();
            if nn.y < 0.0 {
                nn = -nn;
            }
            self.tri_n(pa, pc, pb, nn, col);
        }
        if cap_bottom {
            for i in 1..n - 1 {
                let (b, c) = (p(i), p(i + 1));
                self.tri_n(v3(c0.0, y0, c0.1), v3(b.0, y0, b.1), v3(c.0, y0, c.1), v3(0.0, -1.0, 0.0), lo);
            }
        }
    }

    pub fn prism(&mut self, pts: &[(f32, f32)], y0: f32, y1: f32, col: Col) {
        self.prism_h(pts, y0, &|_, _| y1, col, y0 > 0.05);
    }

    /// Pyramid roof over a convex footprint at height y0, rising to `apex`.
    pub fn pyramid(&mut self, pts: &[(f32, f32)], y0: f32, apex: V3, col: Col) {
        let n = pts.len();
        let ccw = crate::world::poly_area2(pts) >= 0.0;
        for i in 0..n {
            let (a, b) = if ccw { (pts[i], pts[(i + 1) % n]) } else { (pts[(i + 1) % n], pts[i]) };
            let (pa, pb) = (v3(a.0, y0, a.1), v3(b.0, y0, b.1));
            let mut nn = (apex - pa).cross(pb - pa).norm();
            if nn.y < 0.0 {
                nn = -nn;
            }
            self.tri_n(pa, apex, pb, nn, col);
            self.tri_n(pa, pb, v3((a.0 + b.0) * 0.5, y0, (a.1 + b.1) * 0.5), v3(0.0, -1.0, 0.0), col);
        }
        // underside
        let c = pts.iter().fold((0.0, 0.0), |s, p| (s.0 + p.0, s.1 + p.1));
        let c = v3(c.0 / n as f32, y0, c.1 / n as f32);
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            self.tri_n(c, v3(a.0, y0, a.1), v3(b.0, y0, b.1), v3(0.0, -1.0, 0.0), col);
        }
    }

    /// Double-sided sloped quad (gable roof planes and the like).
    pub fn slab(&mut self, a: V3, b: V3, c: V3, d: V3, col: Col) {
        let mut nn = (b - a).cross(d - a).norm();
        if nn.y < 0.0 {
            nn = -nn;
        }
        self.quad_n(a, b, c, d, nn, col);
        self.quad_n(d, c, b, a, -nn, col);
    }
}

pub struct Buf {
    buf: WebGlBuffer,
    vao: WebGlVertexArrayObject,
    pub count: i32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Lit,
    View,
    Sky,
    Alpha,
    Additive,
    Fence,
    Scope,
}

pub struct Renderer {
    pub gl: GL,
    prog: WebGlProgram,
    u_vp: Option<WebGlUniformLocation>,
    u_off: Option<WebGlUniformLocation>,
    u_cam: Option<WebGlUniformLocation>,
    u_light: Option<WebGlUniformLocation>,
    u_fogcol: Option<WebGlUniformLocation>,
    u_fogden: Option<WebGlUniformLocation>,
    u_fogon: Option<WebGlUniformLocation>,
    u_mode: Option<WebGlUniformLocation>,
    u_inmin: Option<WebGlUniformLocation>,
    u_inmax: Option<WebGlUniformLocation>,
    u_inon: Option<WebGlUniformLocation>,
    u_aux: Option<WebGlUniformLocation>,
    u_lvp: Option<WebGlUniformLocation>,
    u_shadow: Option<WebGlUniformLocation>,
    u_shadow_on: Option<WebGlUniformLocation>,
    u_shadow_texel: Option<WebGlUniformLocation>,
    shadow_prog: WebGlProgram,
    shadow_u_lvp: Option<WebGlUniformLocation>,
    shadow_tex: WebGlTexture,
    shadow_fbo: WebGlFramebuffer,
    shadow_size: i32,
    shadow_ok: bool,
    pub w: i32,
    pub h: i32,
}

const VS: &str = r#"#version 300 es
precision highp float;
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNor;
layout(location=2) in vec4 aCol;
uniform mat4 uVP;
uniform mat4 uLVP;
uniform vec3 uOff;
uniform vec3 uCam;
uniform float uMode;
out vec3 vNor;
out vec4 vCol;
out vec3 vPos;
out float vDist;
out vec2 vNdc;
out vec3 vSh;
void main(){
  vec3 p = aPos + uOff;
  vNdc = aPos.xy;
  if (uMode > 2.5) {
    gl_Position = vec4(aPos.xy, 0.0, 1.0);   // full-screen overlay: positions are already clip space
  } else {
    gl_Position = uVP * vec4(p, 1.0);
  }
  vNor = aNor;
  vCol = aCol;
  vPos = p;
  vDist = length(p - uCam);
  // where this point lands in the sun's shadow map (nudged along the normal to avoid acne)
  vec4 lp = uLVP * vec4(p + aNor * 0.14, 1.0);
  vSh = lp.xyz * 0.5 + 0.5;
}
"#;

const FS: &str = r#"#version 300 es
precision highp float;
precision highp sampler2DShadow;
in vec3 vNor;
in vec4 vCol;
in vec3 vPos;
in float vDist;
in vec2 vNdc;
in vec3 vSh;
uniform vec4 uAux;     // x = aspect ratio, y = scope amount (0..1)
uniform vec3 uLight;
uniform vec3 uFogCol;
uniform float uFogDen;
uniform float uFogOn;
uniform float uMode;
uniform vec3 uInMin;
uniform vec3 uInMax;
uniform float uInOn;
uniform sampler2DShadow uShadow;
uniform float uShadowOn;
uniform float uShadowTexel;
out vec4 o;

// soft sun shadow: 3x3 taps of hardware-filtered comparisons
float shadowAt(vec3 sc) {
  if (uShadowOn < 0.5) return 1.0;
  float edge = min(min(sc.x, 1.0 - sc.x), min(sc.y, 1.0 - sc.y));
  if (edge <= 0.0 || sc.z >= 1.0) return 1.0;
  float s = 0.0;
  for (int i = -1; i <= 1; i++) {
    for (int j = -1; j <= 1; j++) {
      s += texture(uShadow, vec3(sc.xy + vec2(float(i), float(j)) * uShadowTexel * 1.3, sc.z - 0.0005));
    }
  }
  s /= 9.0;
  return mix(1.0, s, smoothstep(0.0, 0.03, edge));
}

void main(){
  if (uMode > 2.5) {
    // scope: black everywhere outside a circle, with a soft edge
    float r = length(vec2(vNdc.x * uAux.x, vNdc.y));
    float R = 0.94;
    o = vec4(0.0, 0.0, 0.0, smoothstep(R * 0.93, R, r) * uAux.y);
    return;
  }
  if (uMode > 1.5) {
    // chain-link fence: a diamond lattice computed from world position, so one quad is a whole panel.
    // vCol.r / vCol.g carry how far the panel has slid (so gates keep their mesh as they move).
    float alongZ = step(0.5, abs(vNor.x));
    float u = mix(vPos.x, vPos.z, alongZ) - vCol.g;
    float v = vPos.y - vCol.r;
    float cell = 0.26;
    float t1 = (u + v) / cell;
    float t2 = (u - v) / cell;
    float d1 = abs(fract(t1 + 0.5) - 0.5);
    float d2 = abs(fract(t2 + 0.5) - 0.5);
    float f1 = fwidth(t1) * 0.75;
    float f2 = fwidth(t2) * 0.75;
    float w = 0.055;
    float a1 = 1.0 - smoothstep(w - f1, w + f1, d1);
    float a2 = 1.0 - smoothstep(w - f2, w + f2, d2);
    float cov = a1 + a2 - a1 * a2;
    // once the mesh is finer than a pixel, show its average coverage instead of shimmering
    cov = mix(cov, 0.20, smoothstep(0.35, 1.2, max(f1, f2)));
    vec3 n = normalize(vNor);
    vec3 lit = vec3(0.17, 0.19, 0.22) * (0.75 + 0.35 * abs(dot(n, uLight)));
    float f = 1.0 - exp(-pow(vDist * uFogDen, 1.5));
    lit = mix(lit, uFogCol, clamp(f, 0.0, 1.0));
    o = vec4(lit, cov * 0.9);
    return;
  }
  if (uMode > 0.5) {
    o = vec4(vCol.rgb, vCol.a);
    return;
  }
  vec3 n = normalize(vNor);
  float inside = 0.0;
  if (uInOn > 0.5 && all(greaterThan(vPos, uInMin)) && all(lessThan(vPos, uInMax))) inside = 1.0;
  float nl = max(dot(n, uLight), 0.0);
  float sh = shadowAt(vSh);
  float hemi = 0.5 + 0.5 * n.y;
  vec3 amb = mix(vec3(0.52, 0.52, 0.52), vec3(0.68, 0.72, 0.78), hemi);
  vec3 sun = vec3(1.0, 0.96, 0.90) * nl * 0.58 * sh;
  amb *= mix(1.0, 0.82, inside);
  sun *= mix(1.0, 0.10, inside);
  vec3 lit = vCol.rgb * (amb + sun);
  lit = mix(lit, vCol.rgb, vCol.a);
  float f = 1.0 - exp(-pow(vDist * uFogDen, 1.5));
  lit = mix(lit, uFogCol, clamp(f, 0.0, 1.0) * uFogOn);
  o = vec4(lit, 1.0);
}
"#;

const VS_SHADOW: &str = r#"#version 300 es
precision highp float;
layout(location=0) in vec3 aPos;
uniform mat4 uLVP;
void main(){ gl_Position = uLVP * vec4(aPos, 1.0); }
"#;

const FS_SHADOW: &str = r#"#version 300 es
precision lowp float;
out vec4 o;
void main(){ o = vec4(1.0); }
"#;

fn compile(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, JsValue> {
    let s = gl.create_shader(kind).ok_or("shader alloc")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if gl
        .get_shader_parameter(&s, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(s)
    } else {
        Err(JsValue::from_str(
            &gl.get_shader_info_log(&s).unwrap_or_default(),
        ))
    }
}

impl Renderer {
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Renderer, JsValue> {
        let opts = js_sys::Object::new();
        js_sys::Reflect::set(&opts, &"antialias".into(), &true.into())?;
        js_sys::Reflect::set(&opts, &"alpha".into(), &false.into())?;
        js_sys::Reflect::set(&opts, &"powerPreference".into(), &"high-performance".into())?;
        let gl: GL = canvas
            .get_context_with_context_options("webgl2", &opts)?
            .ok_or("WebGL2 not supported")?
            .dyn_into()?;
        let vs = compile(&gl, GL::VERTEX_SHADER, VS)?;
        let fs = compile(&gl, GL::FRAGMENT_SHADER, FS)?;
        let prog = gl.create_program().ok_or("program alloc")?;
        gl.attach_shader(&prog, &vs);
        gl.attach_shader(&prog, &fs);
        gl.link_program(&prog);
        if !gl
            .get_program_parameter(&prog, GL::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            return Err(JsValue::from_str(
                &gl.get_program_info_log(&prog).unwrap_or_default(),
            ));
        }
        gl.use_program(Some(&prog));
        let u = |n: &str| gl.get_uniform_location(&prog, n);

        // sun shadow map: a depth-only texture we render the static world into once
        let shadow_size = 3072;
        let svs = compile(&gl, GL::VERTEX_SHADER, VS_SHADOW)?;
        let sfs = compile(&gl, GL::FRAGMENT_SHADER, FS_SHADOW)?;
        let shadow_prog = gl.create_program().ok_or("program alloc")?;
        gl.attach_shader(&shadow_prog, &svs);
        gl.attach_shader(&shadow_prog, &sfs);
        gl.link_program(&shadow_prog);
        let shadow_tex = gl.create_texture().ok_or("texture alloc")?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&shadow_tex));
        gl.tex_storage_2d(GL::TEXTURE_2D, 1, GL::DEPTH_COMPONENT24, shadow_size, shadow_size);
        for (k, v) in [
            (GL::TEXTURE_MIN_FILTER, GL::LINEAR),
            (GL::TEXTURE_MAG_FILTER, GL::LINEAR),
            (GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE),
            (GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE),
            (GL::TEXTURE_COMPARE_MODE, GL::COMPARE_REF_TO_TEXTURE),
            (GL::TEXTURE_COMPARE_FUNC, GL::LEQUAL),
        ] {
            gl.tex_parameteri(GL::TEXTURE_2D, k, v as i32);
        }
        let shadow_fbo = gl.create_framebuffer().ok_or("framebuffer alloc")?;
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&shadow_fbo));
        gl.framebuffer_texture_2d(GL::FRAMEBUFFER, GL::DEPTH_ATTACHMENT, GL::TEXTURE_2D, Some(&shadow_tex), 0);
        gl.draw_buffers(&js_sys::Array::of1(&JsValue::from_f64(GL::NONE as f64)));
        gl.read_buffer(GL::NONE);
        let shadow_ok = gl.check_framebuffer_status(GL::FRAMEBUFFER) == GL::FRAMEBUFFER_COMPLETE;
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);

        let r = Renderer {
            u_vp: u("uVP"),
            u_off: u("uOff"),
            u_cam: u("uCam"),
            u_light: u("uLight"),
            u_fogcol: u("uFogCol"),
            u_fogden: u("uFogDen"),
            u_fogon: u("uFogOn"),
            u_mode: u("uMode"),
            u_inmin: u("uInMin"),
            u_inmax: u("uInMax"),
            u_inon: u("uInOn"),
            u_aux: u("uAux"),
            u_lvp: u("uLVP"),
            u_shadow: u("uShadow"),
            u_shadow_on: u("uShadowOn"),
            u_shadow_texel: u("uShadowTexel"),
            shadow_u_lvp: gl.get_uniform_location(&shadow_prog, "uLVP"),
            shadow_prog,
            shadow_tex,
            shadow_fbo,
            shadow_size,
            shadow_ok,
            gl: gl.clone(),
            prog,
            w: canvas.width() as i32,
            h: canvas.height() as i32,
        };
        gl.enable(GL::DEPTH_TEST);
        gl.depth_func(GL::LEQUAL);
        r.set_shadow(&identity(), false);
        Ok(r)
    }

    /// Render the given (static) geometry into the sun's shadow map. Done once at start-up.
    pub fn render_shadow(&self, lvp: &M4, bufs: &[&Buf]) {
        if !self.shadow_ok {
            return;
        }
        let gl = &self.gl;
        gl.active_texture(GL::TEXTURE1);
        gl.bind_texture(GL::TEXTURE_2D, None);
        gl.active_texture(GL::TEXTURE0);
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.shadow_fbo));
        gl.viewport(0, 0, self.shadow_size, self.shadow_size);
        gl.use_program(Some(&self.shadow_prog));
        gl.uniform_matrix4fv_with_f32_array(self.shadow_u_lvp.as_ref(), false, lvp);
        gl.enable(GL::DEPTH_TEST);
        gl.depth_mask(true);
        gl.disable(GL::BLEND);
        gl.color_mask(false, false, false, false);
        gl.clear(GL::DEPTH_BUFFER_BIT);
        gl.enable(GL::POLYGON_OFFSET_FILL);
        gl.polygon_offset(2.0, 3.0);
        for b in bufs {
            gl.bind_vertex_array(Some(&b.vao));
            gl.draw_arrays(GL::TRIANGLES, 0, b.count);
        }
        gl.bind_vertex_array(None);
        gl.disable(GL::POLYGON_OFFSET_FILL);
        gl.color_mask(true, true, true, true);
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
        gl.viewport(0, 0, self.w, self.h);
        gl.use_program(Some(&self.prog));
        self.set_shadow(lvp, true);
    }

    /// Bind the shadow map for the main passes.
    pub fn set_shadow(&self, lvp: &M4, on: bool) {
        let gl = &self.gl;
        gl.use_program(Some(&self.prog));
        gl.uniform_matrix4fv_with_f32_array(self.u_lvp.as_ref(), false, lvp);
        gl.uniform1f(self.u_shadow_on.as_ref(), if on && self.shadow_ok { 1.0 } else { 0.0 });
        gl.uniform1f(self.u_shadow_texel.as_ref(), 1.0 / self.shadow_size as f32);
        gl.active_texture(GL::TEXTURE1);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.shadow_tex));
        gl.uniform1i(self.u_shadow.as_ref(), 1);
        gl.active_texture(GL::TEXTURE0);
    }

    pub fn set_shadow_on(&self, on: bool) {
        self.gl.uniform1f(self.u_shadow_on.as_ref(), if on && self.shadow_ok { 1.0 } else { 0.0 });
    }

    pub fn make_buf(&self) -> Buf {
        let gl = &self.gl;
        let vao = gl.create_vertex_array().unwrap();
        let buf = gl.create_buffer().unwrap();
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
        let stride = (FLOATS * 4) as i32;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 3, GL::FLOAT, false, stride, 12);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, stride, 24);
        gl.bind_vertex_array(None);
        Buf { buf, vao, count: 0 }
    }

    pub fn upload(&self, b: &mut Buf, data: &[f32], dynamic: bool) {
        let gl = &self.gl;
        gl.bind_vertex_array(Some(&b.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&b.buf));
        // SAFETY: the view is consumed immediately by buffer_data, no allocation in between.
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(
                GL::ARRAY_BUFFER,
                &view,
                if dynamic { GL::DYNAMIC_DRAW } else { GL::STATIC_DRAW },
            );
        }
        b.count = (data.len() / FLOATS) as i32;
        gl.bind_vertex_array(None);
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        self.gl.viewport(0, 0, w, h);
    }

    pub fn begin_frame(&self, clear: [f32; 3]) {
        let gl = &self.gl;
        gl.viewport(0, 0, self.w, self.h);
        gl.depth_mask(true);
        gl.disable(GL::BLEND);
        gl.clear_color(clear[0], clear[1], clear[2], 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);
    }

    pub fn clear_depth(&self) {
        self.gl.depth_mask(true);
        self.gl.clear(GL::DEPTH_BUFFER_BIT);
    }

    pub fn set_scene(&self, vp: &M4, cam: V3, light: V3, fog_col: [f32; 3], fog_den: f32) {
        let gl = &self.gl;
        gl.use_program(Some(&self.prog));
        gl.uniform_matrix4fv_with_f32_array(self.u_vp.as_ref(), false, vp);
        gl.uniform3f(self.u_cam.as_ref(), cam.x, cam.y, cam.z);
        gl.uniform3f(self.u_light.as_ref(), light.x, light.y, light.z);
        gl.uniform3f(self.u_fogcol.as_ref(), fog_col[0], fog_col[1], fog_col[2]);
        gl.uniform1f(self.u_fogden.as_ref(), fog_den);
        gl.uniform3f(self.u_off.as_ref(), 0.0, 0.0, 0.0);
    }

    pub fn set_indoor(&self, on: bool, min: V3, max: V3) {
        let gl = &self.gl;
        gl.uniform1f(self.u_inon.as_ref(), if on { 1.0 } else { 0.0 });
        gl.uniform3f(self.u_inmin.as_ref(), min.x, min.y, min.z);
        gl.uniform3f(self.u_inmax.as_ref(), max.x, max.y, max.z);
    }

    pub fn set_aux(&self, aspect: f32, scope: f32) {
        self.gl.uniform4f(self.u_aux.as_ref(), aspect, scope, 0.0, 0.0);
    }

    pub fn set_offset(&self, o: V3) {
        self.gl.uniform3f(self.u_off.as_ref(), o.x, o.y, o.z);
    }

    pub fn draw(&self, b: &Buf, mode: Mode) {
        if b.count == 0 {
            return;
        }
        let gl = &self.gl;
        match mode {
            Mode::Lit => {
                gl.disable(GL::BLEND);
                gl.depth_mask(true);
                gl.uniform1f(self.u_mode.as_ref(), 0.0);
                gl.uniform1f(self.u_fogon.as_ref(), 1.0);
            }
            Mode::View => {
                gl.disable(GL::BLEND);
                gl.depth_mask(true);
                gl.uniform1f(self.u_mode.as_ref(), 0.0);
                gl.uniform1f(self.u_fogon.as_ref(), 0.0);
            }
            Mode::Sky => {
                gl.disable(GL::BLEND);
                gl.depth_mask(false);
                gl.uniform1f(self.u_mode.as_ref(), 0.0);
                gl.uniform1f(self.u_fogon.as_ref(), 0.0);
            }
            Mode::Alpha => {
                gl.enable(GL::BLEND);
                gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
                gl.depth_mask(false);
                gl.uniform1f(self.u_mode.as_ref(), 1.0);
                gl.uniform1f(self.u_fogon.as_ref(), 0.0);
            }
            Mode::Additive => {
                gl.enable(GL::BLEND);
                gl.blend_func(GL::SRC_ALPHA, GL::ONE);
                gl.depth_mask(false);
                gl.uniform1f(self.u_mode.as_ref(), 1.0);
                gl.uniform1f(self.u_fogon.as_ref(), 0.0);
            }
            Mode::Fence => {
                gl.enable(GL::BLEND);
                gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
                gl.depth_mask(false);
                gl.uniform1f(self.u_mode.as_ref(), 2.0);
                gl.uniform1f(self.u_fogon.as_ref(), 0.0);
            }
            Mode::Scope => {
                gl.enable(GL::BLEND);
                gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
                gl.disable(GL::DEPTH_TEST);
                gl.depth_mask(false);
                gl.uniform1f(self.u_mode.as_ref(), 3.0);
                gl.uniform1f(self.u_fogon.as_ref(), 0.0);
            }
        }
        gl.bind_vertex_array(Some(&b.vao));
        gl.draw_arrays(GL::TRIANGLES, 0, b.count);
        gl.bind_vertex_array(None);
        gl.enable(GL::DEPTH_TEST);
        gl.depth_mask(true);
        gl.disable(GL::BLEND);
    }
}
