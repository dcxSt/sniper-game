//! Level-building toolkit: walls at any angle with door and window openings, octagonal rings,
//! floors with stair holes, smooth stairs (drawn steps + a walkable ramp), switchbacks, lookouts,
//! roofs, trees and hedges. Everything drawn here also goes into the collision world.

use crate::gfx::{Col, Mesh};
use crate::math::*;
use crate::world::*;
use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};

pub type P2 = (f32, f32);

pub fn add2(a: P2, b: P2) -> P2 {
    (a.0 + b.0, a.1 + b.1)
}
pub fn sub2(a: P2, b: P2) -> P2 {
    (a.0 - b.0, a.1 - b.1)
}
pub fn mul2(a: P2, s: f32) -> P2 {
    (a.0 * s, a.1 * s)
}
pub fn dot2(a: P2, b: P2) -> f32 {
    a.0 * b.0 + a.1 * b.1
}
pub fn len2(a: P2) -> f32 {
    dot2(a, a).sqrt()
}
pub fn norm2(a: P2) -> P2 {
    let l = len2(a).max(1e-9);
    (a.0 / l, a.1 / l)
}
/// left-hand perpendicular (rotate +90° in the xz plane)
pub fn perp2(a: P2) -> P2 {
    (-a.1, a.0)
}
pub fn lerp2(a: P2, b: P2, t: f32) -> P2 {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Keep the part of a convex polygon where n·p <= d.
pub fn clip(poly: &[P2], n: P2, d: f32) -> Vec<P2> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let (da, db) = (dot2(n, a) - d, dot2(n, b) - d);
        if da <= 0.0 {
            out.push(a);
        }
        if (da < 0.0 && db > 0.0) || (da > 0.0 && db < 0.0) {
            out.push(lerp2(a, b, da / (da - db)));
        }
    }
    out
}

/// Regular octagon with flat sides facing the axes and diagonals; side k faces `oct_dir(k)`.
pub fn octagon(cx: f32, cz: f32, r: f32) -> Vec<P2> {
    (0..8)
        .map(|i| {
            let a = -FRAC_PI_8 + i as f32 * FRAC_PI_4;
            (cx + r * a.cos(), cz + r * a.sin())
        })
        .collect()
}
/// Outward direction of octagon side k (0 = +x east, 2 = +z south, 4 = −x west, 6 = −z north).
pub fn oct_dir(k: usize) -> P2 {
    let a = k as f32 * FRAC_PI_4;
    (a.cos(), a.sin())
}
pub fn apothem(r: f32) -> f32 {
    r * FRAC_PI_8.cos()
}

pub fn rect(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<P2> {
    vec![(x0, z0), (x1, z0), (x1, z1), (x0, z1)]
}

/// A rectangular hole: centre, unit axis, half-length along the axis, half-width across it.
#[derive(Clone, Copy)]
pub struct Hole {
    pub c: P2,
    pub u: P2,
    pub hu: f32,
    pub hv: f32,
}

/// A convex polygon minus a rectangle, as up to four convex pieces.
pub fn poly_minus(poly: &[P2], h: Hole) -> Vec<Vec<P2>> {
    let v = perp2(h.u);
    let (cu, cv) = (dot2(h.c, h.u), dot2(h.c, v));
    let mut out = vec![];
    out.push(clip(poly, h.u, cu - h.hu));
    out.push(clip(poly, mul2(h.u, -1.0), -(cu + h.hu)));
    let band = clip(&clip(poly, mul2(h.u, -1.0), -(cu - h.hu)), h.u, cu + h.hu);
    out.push(clip(&band, v, cv - h.hv));
    out.push(clip(&band, mul2(v, -1.0), -(cv + h.hv)));
    out.retain(|p| p.len() >= 3 && poly_area2(p).abs() > 0.02);
    out
}

// a quiet palette: off-white structures, charcoal details, soft sage ground, a few warm notes
pub const CONCRETE: Col = [0.86, 0.86, 0.84, 0.0];
pub const CONCRETE_D: Col = [0.72, 0.72, 0.71, 0.0];
pub const FLOOR_IN: Col = [0.78, 0.78, 0.77, 0.0];
pub const WALL_IN: Col = [0.88, 0.88, 0.86, 0.0];
pub const METAL: Col = [0.22, 0.24, 0.27, 0.0];
pub const WOOD: Col = [0.74, 0.70, 0.64, 0.0];
pub const WOOD_D: Col = [0.55, 0.49, 0.42, 0.0];
pub const ROOF_R: Col = [0.20, 0.21, 0.24, 0.0];
pub const TILE: Col = [0.70, 0.53, 0.45, 0.0];
pub const PLASTER: Col = [0.91, 0.89, 0.85, 0.0];
pub const PLASTER_W: Col = [0.90, 0.85, 0.76, 0.0];
pub const STONE: Col = [0.76, 0.74, 0.70, 0.0];
pub const BARN: Col = [0.64, 0.40, 0.35, 0.0];
pub const SAND: Col = [0.80, 0.77, 0.70, 0.0];
pub const CRATE: Col = [0.77, 0.74, 0.69, 0.0];
pub const BLACK: Col = [0.09, 0.09, 0.10, 0.0];
pub const LAMP: Col = [1.0, 0.97, 0.88, 1.0];
pub const ACCENT: Col = [0.93, 0.45, 0.20, 0.0];
pub const HEDGE: Col = [0.42, 0.52, 0.44, 0.0];

pub const STAIR_W: f32 = 2.0;
const RUN: f32 = 0.40;
const MAX_RISE: f32 = 0.29;
pub const LANDING: f32 = 1.9;
const LANE_GAP: f32 = 0.15;

/// An opening in a wall: (start, end) along the wall, (bottom, top) heights.
pub type Opening = (f32, f32, f32, f32);

pub struct Builder {
    pub mesh: Mesh,
    pub fence_mesh: Mesh,
    pub shapes: Vec<Shape>,
    pub rng: Rng,
}

impl Builder {
    pub fn new(seed: u64) -> Builder {
        Builder { mesh: Mesh::default(), fence_mesh: Mesh::default(), shapes: vec![], rng: Rng::new(seed) }
    }

    pub fn jc(&mut self, c: Col) -> Col {
        let k = 1.0 + self.rng.range(-0.012, 0.012);
        [c[0] * k, c[1] * k, c[2] * k, c[3]]
    }

    /// Collision only.
    pub fn add(&mut self, s: Shape) {
        self.shapes.push(s);
    }

    pub fn bx(&mut self, min: V3, max: V3, c: Col, flags: u8) {
        let c = self.jc(c);
        self.mesh.cuboid(min, max, c);
        if flags != 0 {
            self.shapes.push(Shape::boxed(min, max, flags));
        }
    }
    pub fn solid(&mut self, min: V3, max: V3, c: Col) {
        self.bx(min, max, c, SOLID);
    }
    pub fn deco(&mut self, min: V3, max: V3, c: Col) {
        self.bx(min, max, c, 0);
    }

    pub fn poly(&mut self, pts: Vec<P2>, y0: f32, y1: f32, c: Col, flags: u8) {
        if pts.len() < 3 || y1 - y0 < 0.005 {
            return;
        }
        let c = self.jc(c);
        self.mesh.prism(&pts, y0, y1, c);
        if flags != 0 {
            self.shapes.push(Shape::new(pts, y0, y1, flags));
        }
    }

    /// One piece of `quad` between distances s0..s1 along `u` (measured from `origin`).
    fn quad_piece(&mut self, quad: &[P2], origin: P2, u: P2, s0: f32, s1: f32, y0: f32, y1: f32, c: Col, flags: u8) {
        if s1 - s0 < 0.01 || y1 - y0 < 0.01 {
            return;
        }
        let o = dot2(origin, u);
        let p = clip(quad, mul2(u, -1.0), -(o + s0));
        let p = clip(&p, u, o + s1);
        if p.len() >= 3 && poly_area2(&p).abs() > 1e-4 {
            self.poly(p, y0, y1, c, flags);
        }
    }

    /// A wall piece with openings cut through it. Openings are measured along `u` from `origin`.
    pub fn wall_quad(&mut self, quad: &[P2], origin: P2, u: P2, y0: f32, y1: f32, c: Col, flags: u8, ops: &[Opening]) {
        let mut cuts = ops.to_vec();
        cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut cur = -1.0e4;
        for g in cuts {
            self.quad_piece(quad, origin, u, cur, g.0, y0, y1, c, flags);
            self.quad_piece(quad, origin, u, g.0, g.1, y0, g.2.max(y0), c, flags);
            self.quad_piece(quad, origin, u, g.0, g.1, g.3.min(y1), y1, c, flags);
            cur = g.1;
        }
        self.quad_piece(quad, origin, u, cur, 1.0e4, y0, y1, c, flags);
    }

    /// A straight wall along the centre line a→b, `t` thick. Openings are measured from `a`.
    pub fn wall(&mut self, a: P2, b: P2, t: f32, y0: f32, y1: f32, c: Col, ops: &[Opening]) {
        let u = norm2(sub2(b, a));
        let v = mul2(perp2(u), t * 0.5);
        let quad = [add2(a, v), add2(b, v), sub2(b, v), sub2(a, v)];
        self.wall_quad(&quad, a, u, y0, y1, c, SOLID, ops);
    }

    /// An octagonal ring of wall. `ops(k)` gives the openings in side k, measured from the side's midpoint.
    #[allow(clippy::too_many_arguments)]
    pub fn ring(&mut self, cx: f32, cz: f32, r: f32, t: f32, y0: f32, y1: f32, c: Col, flags: u8, ops: &dyn Fn(usize) -> Vec<Opening>) {
        let ri = r - t / FRAC_PI_8.cos();
        let o = octagon(cx, cz, r);
        let i = octagon(cx, cz, ri);
        for k in 0..8 {
            let k1 = (k + 1) % 8;
            let quad = [o[k], o[k1], i[k1], i[k]];
            let mid = lerp2(o[k], o[k1], 0.5);
            let u = norm2(sub2(o[k1], o[k]));
            self.wall_quad(&quad, mid, u, y0, y1, c, flags, &ops(k));
        }
    }

    /// A floor slab over a convex footprint, optionally with a rectangular stair hole.
    pub fn floor(&mut self, poly: &[P2], y0: f32, y1: f32, c: Col, hole: Option<Hole>) {
        match hole {
            None => self.poly(poly.to_vec(), y0, y1, c, SOLID),
            Some(h) => {
                for p in poly_minus(poly, h) {
                    self.poly(p, y0, y1, c, SOLID);
                }
            }
        }
    }

    /// A straight flight of stairs from `p0` (centre of the bottom edge) climbing along unit `dir` from y0 to y1.
    /// The steps are drawn and stop bullets; the walking surface is a smooth ramp through the treads, so you
    /// glide up instead of popping from step to step. Returns the centre of the top edge.
    #[allow(clippy::too_many_arguments)]
    pub fn flight(&mut self, p0: P2, dir: P2, w: f32, y0: f32, y1: f32, c: Col, solid_under: bool) -> P2 {
        let rise = y1 - y0;
        let n = (rise / MAX_RISE).ceil().max(1.0) as usize;
        let ri = rise / n as f32;
        let lat = mul2(perp2(dir), w * 0.5);
        for i in 0..n {
            let (a, b) = (i as f32 * RUN, (i + 1) as f32 * RUN);
            let top = y0 + (i + 1) as f32 * ri;
            let bot = if solid_under { y0 - 0.4 } else { (top - 0.8).max(y0 - 0.3) };
            let pa = add2(p0, mul2(dir, a));
            let pb = add2(p0, mul2(dir, b));
            let q = vec![sub2(pa, lat), sub2(pb, lat), add2(pb, lat), add2(pa, lat)];
            self.poly(q, bot, top, c, RAY);
        }
        let len = n as f32 * RUN;
        let k = ri / RUN;
        let (gx, gz) = (dir.0 * k, dir.1 * k);
        let a0 = y0 + ri * 0.5 - (gx * p0.0 + gz * p0.1);
        let end = add2(p0, mul2(dir, len));
        let q = vec![sub2(p0, lat), sub2(end, lat), add2(end, lat), add2(p0, lat)];
        let mut s = Shape::new(q, if solid_under { y0 - 0.4 } else { y0 - 0.3 }, y1, WALK | BLOCK);
        s.ramp = Some((a0, gx, gz));
        self.shapes.push(s);
        end
    }

    pub fn flight_len(rise: f32) -> f32 {
        (rise / MAX_RISE).ceil().max(1.0) * RUN
    }

    /// A guard rail along a→b at floor height y: slim posts and two rails. Stops you, not bullets or eyes.
    pub fn railing(&mut self, a: P2, b: P2, y: f32) {
        let l = len2(sub2(b, a));
        let n = (l / 1.3).ceil().max(1.0) as usize;
        for i in 0..=n {
            let p = lerp2(a, b, i as f32 / n as f32);
            self.mesh.limb(v3(p.0, y, p.1), v3(p.0, y + 1.0, p.1), 0.05, METAL);
        }
        for h in [1.0, 0.5] {
            self.mesh.limb(v3(a.0, y + h, a.1), v3(b.0, y + h, b.1), 0.05, METAL);
        }
        let u = norm2(sub2(b, a));
        let v = mul2(perp2(u), 0.06);
        self.add(Shape::new(vec![add2(a, v), add2(b, v), sub2(b, v), sub2(a, v)], y, y + 1.0, BLOCK));
    }

    /// Thin handrail with a couple of posts, purely visual.
    pub fn rail(&mut self, a: V3, b: V3) {
        let up = v3(0.0, 0.95, 0.0);
        self.mesh.limb(a + up, b + up, 0.05, METAL);
        let n = ((b - a).xz().len() / 2.2).ceil().max(1.0) as usize;
        for i in 0..=n {
            let p = a.lerp(b, i as f32 / n as f32);
            self.mesh.limb(p, p + up, 0.045, METAL);
        }
    }

    /// Stairs folding back on themselves in two lanes. Lane A spans v∈[0,W] from `origin` along unit `v`,
    /// lane B v∈[W+gap, 2W+gap]. Flight 1 climbs lane A from u=D+L down to u=D (travelling −u), a landing
    /// spans both lanes at u∈[0,D], flight 2 climbs lane B back to u=D+L, and so on.
    /// Returns (exit point at the top of the last flight, top height, direction of travel there).
    #[allow(clippy::too_many_arguments)]
    pub fn zigzag(&mut self, origin: P2, u: P2, v: P2, y0: f32, rpf: f32, flights: usize, c: Col, posts: bool) -> (P2, f32, P2) {
        let w = STAIR_W;
        let l = Builder::flight_len(rpf);
        let d = LANDING;
        let at = |uu: f32, vv: f32| add2(origin, add2(mul2(u, uu), mul2(v, vv)));
        let lane = |f: usize| if f % 2 == 0 { w * 0.5 } else { w + LANE_GAP + w * 0.5 };
        let full = 2.0 * w + LANE_GAP;
        let mut exit = (0.0, 0.0);
        let mut dir_out = u;
        for f in 0..flights {
            let yb = y0 + f as f32 * rpf;
            let (start, dir) = if f % 2 == 0 { (at(d + l, lane(f)), mul2(u, -1.0)) } else { (at(d, lane(f)), u) };
            let end = self.flight(start, dir, w, yb, yb + rpf, c, f == 0);
            exit = end;
            dir_out = dir;
            // open side rail
            let side = if f % 2 == 0 { 0.0 } else { full };
            let (ra, rb) = if f % 2 == 0 { (at(d + l, side), at(d, side)) } else { (at(d, side), at(d + l, side)) };
            self.rail(v3(ra.0, yb + 0.15, ra.1), v3(rb.0, yb + rpf, rb.1));
            if f + 1 < flights {
                let yt = yb + rpf;
                let (u0, u1) = if f % 2 == 0 { (0.0, d) } else { (d + l, 2.0 * d + l) };
                let q = vec![at(u0, 0.0), at(u1, 0.0), at(u1, full), at(u0, full)];
                self.poly(q, yt - 0.3, yt, c, SOLID);
                if posts {
                    let um = if f % 2 == 0 { u0 + 0.2 } else { u1 - 0.2 };
                    for vv in [0.2, full - 0.2] {
                        let p = at(um, vv);
                        self.solid(v3(p.0 - 0.13, y0 - 0.5, p.1 - 0.13), v3(p.0 + 0.13, yt - 0.3, p.1 + 0.13), METAL);
                    }
                }
            }
        }
        (exit, y0 + flights as f32 * rpf, dir_out)
    }

    /// Footprint of a zigzag stair: (length along u, width along v).
    pub fn zigzag_size(rpf: f32, flights: usize) -> (f32, f32) {
        let l = Builder::flight_len(rpf);
        (if flights > 1 { 2.0 * LANDING + l } else { LANDING + l }, 2.0 * STAIR_W + LANE_GAP)
    }

    /// An open octagonal lookout: a deck on posts at `base + ph`, a waist-high parapet, a pyramid roof on
    /// slim columns, and a switchback stair arriving on side `ks` (lane A on the `vs` side). Returns the deck centre.
    #[allow(clippy::too_many_arguments)]
    pub fn lookout(&mut self, cx: f32, cz: f32, r: f32, base: f32, ph: f32, ks: usize, vs: f32, roof: bool) -> V3 {
        let top = base + ph;
        self.poly(octagon(cx, cz, r), top - 0.35, top, CONCRETE_D, SOLID);
        let posts = octagon(cx, cz, r * 0.7);
        for (i, p) in posts.iter().enumerate() {
            self.solid(v3(p.0 - 0.2, base - 0.6, p.1 - 0.2), v3(p.0 + 0.2, top - 0.35, p.1 + 0.2), CONCRETE);
            // diagonal bracing between neighbouring posts
            let q = posts[(i + 1) % 8];
            let (lo, hi) = (base + 0.6, top - 0.8);
            if i % 2 == 0 {
                self.mesh.limb(v3(p.0, lo, p.1), v3(q.0, hi, q.1), 0.09, METAL);
            } else {
                self.mesh.limb(v3(p.0, hi, p.1), v3(q.0, lo, q.1), 0.09, METAL);
            }
        }
        self.ring(cx, cz, r, 0.16, top, top + 1.05, METAL, SOLID, &|k| if k == ks { vec![(-9.0, 9.0, top - 1.0, top + 9.0)] } else { vec![] });
        if roof {
            let cols = octagon(cx, cz, r - 0.1);
            for (k, p) in cols.iter().enumerate() {
                // keep the stair opening clear
                let _ = k;
                self.solid(v3(p.0 - 0.07, top + 1.05, p.1 - 0.07), v3(p.0 + 0.07, top + 2.75, p.1 + 0.07), METAL);
            }
            self.poly(octagon(cx, cz, r + 0.35), top + 2.75, top + 2.95, CONCRETE_D, SOLID);
            let apex = v3(cx, top + 4.0, cz);
            let c = self.jc(ROOF_R);
            self.mesh.pyramid(&octagon(cx, cz, r + 0.35), top + 2.95, apex, c);
            self.add(Shape::new(octagon(cx, cz, r * 0.55), top + 2.95, top + 3.5, RAY));
            self.deco(v3(cx - 0.3, top + 2.6, cz - 0.3), v3(cx + 0.3, top + 2.75, cz + 0.3), LAMP);
        }
        // the switchback: its last flight arrives at the middle of side ks, heading for the centre
        let f = oct_dir(ks);
        let u = mul2(f, -1.0);
        let v = mul2(perp2(f), vs);
        let m = (cx + f.0 * apothem(r), cz + f.1 * apothem(r));
        let rpf = ph * 0.5;
        let (lu, _) = Builder::zigzag_size(rpf, 2);
        let lane_b = STAIR_W * 1.5 + LANE_GAP;
        let origin = sub2(sub2(m, mul2(u, lu - LANDING)), mul2(v, lane_b));
        self.zigzag(origin, u, v, base, rpf, 2, METAL, true);
        v3(cx, top, cz)
    }

    /// A calm, simple conifer: two soft cones on a thin trunk. The canopy hides you from sight but not bullets.
    pub fn tree(&mut self, x: f32, z: f32) {
        let h = terrain_h(x, z);
        let s = self.rng.range(0.9, 1.4);
        self.bx(v3(x - 0.2 * s, h - 0.5, z - 0.2 * s), v3(x + 0.2 * s, h + 2.2 * s, z + 0.2 * s), [0.30, 0.31, 0.33, 0.0], SOLID);
        let g = self.rng.range(-0.025, 0.025);
        let col = [0.34 + g, 0.43 + g, 0.38 + g, 0.0];
        self.mesh.cone_smooth(v3(x, h + 0.9 * s, z), 2.2 * s, 5.2 * s, col, 12);
        self.mesh.cone_smooth(v3(x, h + 3.4 * s, z), 1.5 * s, 4.0 * s, col, 12);
        self.add(Shape::new(octagon(x, z, 1.45 * s), h + 1.0 * s, h + 4.6 * s, SIGHT));
    }

    /// A round broad-leaved tree (village, farm).
    pub fn round_tree(&mut self, x: f32, z: f32) {
        let h = terrain_h(x, z);
        let s = self.rng.range(0.85, 1.25);
        self.bx(v3(x - 0.18 * s, h - 0.5, z - 0.18 * s), v3(x + 0.18 * s, h + 2.6 * s, z + 0.18 * s), [0.32, 0.31, 0.30, 0.0], SOLID);
        let g = self.rng.range(-0.03, 0.03);
        let col = [0.40 + g, 0.50 + g, 0.42 + g, 0.0];
        self.mesh.ellipsoid(v3(x, h + 3.9 * s, z), v3(2.3 * s, 1.9 * s, 2.3 * s), col, 7, 12);
        self.add(Shape::new(octagon(x, z, 1.9 * s), h + 2.4 * s, h + 5.4 * s, SIGHT));
    }

    /// A low bush you can crouch in: hides you, doesn't stop you or bullets.
    pub fn bush(&mut self, x: f32, z: f32, r: f32) {
        let h = terrain_h(x, z);
        let g = self.rng.range(-0.03, 0.03);
        let col = [0.41 + g, 0.51 + g, 0.43 + g, 0.0];
        self.mesh.ellipsoid(v3(x, h + r * 0.55, z), v3(r, r * 0.8, r), col, 6, 10);
        self.add(Shape::new(octagon(x, z, r * 0.85), h, h + r * 1.25, SIGHT));
    }

    /// A clipped hedge: blocks walking and sight, but bullets go through.
    pub fn hedge(&mut self, a: P2, b: P2, h: f32) {
        let y = terrain_h(a.0, a.1).min(terrain_h(b.0, b.1)) - 0.2;
        let u = norm2(sub2(b, a));
        let v = mul2(perp2(u), 0.45);
        let q = vec![add2(a, v), add2(b, v), sub2(b, v), sub2(a, v)];
        let c = self.jc(HEDGE);
        self.mesh.prism(&q, y, y + h, c);
        self.add(Shape::new(q, y, y + h, SIGHT | BLOCK));
    }

    /// A rustic wooden fence at any angle: posts and two rails. Waist high — blocks walking, not much else.
    pub fn wood_fence(&mut self, a: P2, b: P2) {
        let l = len2(sub2(b, a));
        let n = (l / 2.5).ceil().max(1.0) as usize;
        let ga = terrain_h(a.0, a.1);
        let gb = terrain_h(b.0, b.1);
        for i in 0..=n {
            let p = lerp2(a, b, i as f32 / n as f32);
            let g = terrain_h(p.0, p.1);
            self.bx(v3(p.0 - 0.07, g - 0.3, p.1 - 0.07), v3(p.0 + 0.07, g + 1.15, p.1 + 0.07), WOOD_D, RAY);
        }
        for y in [0.5, 1.0] {
            self.mesh.limb(v3(a.0, ga + y, a.1), v3(b.0, gb + y, b.1), 0.07, WOOD_D);
        }
        let u = norm2(sub2(b, a));
        let v = mul2(perp2(u), 0.1);
        self.add(Shape::new(vec![add2(a, v), add2(b, v), sub2(b, v), sub2(a, v)], ga.min(gb) - 0.3, ga.max(gb) + 1.1, BLOCK));
    }

    /// A chain-link fence run between two points on one axis. See-through and shoot-through (the lattice is
    /// drawn by the fence shader and is NOT a raycast solid) but it still blocks walking.
    /// `gaps` are (start, end) stretches along the run where a gate or breach section stands instead.
    pub fn fence_run(&mut self, a: P2, b: P2, h: f32, gaps: &[(f32, f32)]) {
        let along_x = (b.0 - a.0).abs() >= (b.1 - a.1).abs();
        let (s, e) = if along_x { (a.0.min(b.0), a.0.max(b.0)) } else { (a.1.min(b.1), a.1.max(b.1)) };
        let fixed = if along_x { a.1 } else { a.0 };
        let mut cuts = gaps.to_vec();
        cuts.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let mut segs: Vec<(f32, f32)> = vec![];
        let mut cur = s;
        for g in cuts {
            if g.0 > cur {
                segs.push((cur, g.0));
            }
            cur = g.1;
        }
        if cur < e {
            segs.push((cur, e));
        }
        let wire = [0.20, 0.21, 0.24, 0.0];
        let at = |t: f32, side: f32| if along_x { (t, fixed + side) } else { (fixed + side, t) };
        for (p, q) in segs {
            let (p0, p1) = (at(p, 0.0), at(q, 0.0));
            self.fence_mesh.fence_quad(v3(p0.0, 0.0, p0.1), v3(p1.0, 0.0, p1.1), 0.0, h, 0.0, 0.0);
            let (mn, mx) = (at(p, -0.15), at(q, 0.15));
            self.add(Shape::boxed(
                v3(mn.0.min(mx.0), 0.0, mn.1.min(mx.1)),
                v3(mn.0.max(mx.0), h, mn.1.max(mx.1)),
                BLOCK,
            ));
            for (y0, y1, th) in [(h - 0.07, h + 0.04, 0.05), (0.0, 0.09, 0.04)] {
                let (a0, a1) = (at(p, -th), at(q, th));
                self.deco(v3(a0.0.min(a1.0), y0, a0.1.min(a1.1)), v3(a0.0.max(a1.0), y1, a0.1.max(a1.1)), METAL);
            }
            let n = ((q - p) / 3.0).ceil().max(1.0) as usize;
            for i in 0..=n {
                let t = p + (q - p) * i as f32 / n as f32;
                let c = at(t, 0.0);
                self.deco(v3(c.0 - 0.06, 0.0, c.1 - 0.06), v3(c.0 + 0.06, h + 0.4, c.1 + 0.06), METAL);
            }
            let (a0, a1) = (at(p, -0.02), at(q, 0.02));
            self.deco(v3(a0.0.min(a1.0), h + 0.3, a0.1.min(a1.1)), v3(a0.0.max(a1.0), h + 0.33, a0.1.max(a1.1)), wire);
        }
    }

    /// Pitched roof over an axis-aligned footprint (ridge along the longer side). Drawn as planes; its bulk
    /// stops bullets as a few stacked slabs.
    pub fn gable(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, rise: f32, c: Col) {
        let o = 0.35;
        let (x0, z0, x1, z1) = (x0 - o, z0 - o, x1 + o, z1 + o);
        let c = self.jc(c);
        let along_x = (x1 - x0) >= (z1 - z0);
        if along_x {
            let zm = (z0 + z1) * 0.5;
            let (a, b) = (v3(x0, y + rise, zm), v3(x1, y + rise, zm));
            self.mesh.slab(v3(x0, y, z0), v3(x1, y, z0), b, a, c);
            self.mesh.slab(v3(x1, y, z1), v3(x0, y, z1), a, b, c);
            let w = WALL_IN;
            for x in [x0 + o, x1 - o] {
                self.mesh.tri(v3(x, y, z0 + o), v3(x, y, z1 - o), v3(x, y + rise * 0.92, zm), w);
                self.mesh.tri(v3(x, y, z1 - o), v3(x, y, z0 + o), v3(x, y + rise * 0.92, zm), w);
            }
            for k in 0..3 {
                let f = k as f32 / 3.0;
                let hw = (z1 - z0) * 0.5 * (1.0 - f) * 0.9;
                self.add(Shape::boxed(v3(x0 + o, y + rise * f, zm - hw), v3(x1 - o, y + rise * (f + 0.33), zm + hw), RAY));
            }
        } else {
            let xm = (x0 + x1) * 0.5;
            let (a, b) = (v3(xm, y + rise, z0), v3(xm, y + rise, z1));
            self.mesh.slab(v3(x0, y, z1), v3(x0, y, z0), a, b, c);
            self.mesh.slab(v3(x1, y, z0), v3(x1, y, z1), b, a, c);
            let w = WALL_IN;
            for z in [z0 + o, z1 - o] {
                self.mesh.tri(v3(x0 + o, y, z), v3(x1 - o, y, z), v3(xm, y + rise * 0.92, z), w);
                self.mesh.tri(v3(x1 - o, y, z), v3(x0 + o, y, z), v3(xm, y + rise * 0.92, z), w);
            }
            for k in 0..3 {
                let f = k as f32 / 3.0;
                let hw = (x1 - x0) * 0.5 * (1.0 - f) * 0.9;
                self.add(Shape::boxed(v3(xm - hw, y + rise * f, z0 + o), v3(xm + hw, y + rise * (f + 0.33), z1 - o), RAY));
            }
        }
    }

    /// Pyramid roof over a convex footprint; stops bullets as two stacked slabs.
    pub fn hip(&mut self, pts: &[P2], y: f32, rise: f32, c: Col) {
        let n = pts.len() as f32;
        let (cx, cz) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
        let c = self.jc(c);
        self.mesh.pyramid(pts, y, v3(cx, y + rise, cz), c);
        for (f, s) in [(0.0, 0.8), (0.4, 0.45)] {
            let q: Vec<P2> = pts.iter().map(|p| (cx + (p.0 - cx) * s, cz + (p.1 - cz) * s)).collect();
            self.add(Shape::new(q, y + rise * f, y + rise * (f + 0.4), RAY));
        }
    }
}
