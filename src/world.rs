//! Terrain, collision and ray queries.
//!
//! The world is a set of convex prisms (any convex footprint, extruded between two heights) kept in a
//! uniform spatial grid. Each prism says what it does via flags: stop bullets, stop sight (foliage),
//! carry your weight, or block movement. Stairs are *ramps* (a prism whose top is a sloped plane) for
//! walking, with the visual steps as separate bullet-stopping prisms — so climbing is perfectly smooth.

use crate::math::*;
use std::cell::{Cell, RefCell};

pub const STEP: f32 = 0.55;
pub const GRAV: f32 = 22.0;

/// stops bullets (and sight)
pub const RAY: u8 = 1;
/// stops sight only — foliage, hedges
pub const SIGHT: u8 = 2;
/// you can stand on top of it
pub const WALK: u8 = 4;
/// stops movement
pub const BLOCK: u8 = 8;
pub const SOLID: u8 = RAY | WALK | BLOCK;

// ---------------------------------------------------------------- terrain --

/// The lookout hill in the south-east.
pub const HILL: (f32, f32) = (54.0, 80.0);

fn rect_factor(x: f32, z: f32, x0: f32, x1: f32, z0: f32, z1: f32, blend: f32) -> f32 {
    let dx = (x0 - x).max(x - x1).max(0.0);
    let dz = (z0 - z).max(z - z1).max(0.0);
    smoothstep(0.0, blend, (dx * dx + dz * dz).sqrt())
}

/// 0 where the ground is levelled for building (prison, village, farm), 1 in the wild.
pub fn flat_factor(x: f32, z: f32) -> f32 {
    rect_factor(x, z, -46.0, 46.0, -82.0, 36.0, 14.0)
        .min(rect_factor(x, z, -84.0, -10.0, 40.0, 114.0, 12.0))
        .min(rect_factor(x, z, 52.0, 118.0, -48.0, 24.0, 12.0))
}

/// Raw hill mask: 1 on the flat crown of the lookout hill, fading to 0 at its foot.
pub fn hill_mask(x: f32, z: f32) -> f32 {
    let hx = (x - HILL.0) / 38.0;
    let hz = (z - HILL.1) / 32.0;
    1.0 - smoothstep(0.2, 1.0, (hx * hx + hz * hz).sqrt())
}

pub fn terrain_h(x: f32, z: f32) -> f32 {
    // gentle, long undulations — calm rather than lumpy
    let rolling = 1.3 * ((x * 0.036 + 1.3).sin() * (z * 0.041).cos()) + 0.6 * (x * 0.083 + z * 0.061).sin();
    let hm = hill_mask(x, z);
    let hill = 16.0 * hm;
    // a low wooded mound north-west of the prison
    let (mx, mz) = ((x + 86.0) / 26.0, (z + 58.0) / 24.0);
    let mound = 9.0 * (-(mx * mx + mz * mz)).exp();
    // a soft bowl of hills all the way round, which the fog melts into the sky
    let r = (x * x + z * z).sqrt();
    let edge = smoothstep(140.0, 200.0, r) * 60.0;
    let h = rolling * (1.0 - hm * 0.9) + hill + mound + edge;
    h * flat_factor(x, z)
}

pub fn terrain_normal(x: f32, z: f32) -> V3 {
    let e = 0.6;
    let dx = terrain_h(x + e, z) - terrain_h(x - e, z);
    let dz = terrain_h(x, z + e) - terrain_h(x, z - e);
    v3(-dx, 2.0 * e, -dz).norm()
}

// ------------------------------------------------------------------ boxes --

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: V3,
    pub max: V3,
}

impl Aabb {
    pub fn overlaps_xz(&self, x: f32, z: f32, r: f32) -> bool {
        x + r > self.min.x && x - r < self.max.x && z + r > self.min.z && z - r < self.max.z
    }

    /// Signed separation of a point from the box footprint (negative inside), and the outward axis.
    fn sep(&self, x: f32, z: f32) -> (f32, (f32, f32)) {
        let c = [
            (self.min.x - x, (-1.0, 0.0)),
            (x - self.max.x, (1.0, 0.0)),
            (self.min.z - z, (0.0, -1.0)),
            (z - self.max.z, (0.0, 1.0)),
        ];
        let mut best = c[0];
        for k in &c[1..] {
            if k.0 > best.0 {
                best = *k;
            }
        }
        best
    }

    /// Ray vs box. Returns entry distance and surface normal. Rays starting inside are ignored.
    pub fn ray(&self, o: V3, d: V3, tmax: f32) -> Option<(f32, V3)> {
        let mut tmin = 0.0f32;
        let mut tmx = tmax;
        let mut n = V3::ZERO;
        let oo = [o.x, o.y, o.z];
        let dd = [d.x, d.y, d.z];
        let mn = [self.min.x, self.min.y, self.min.z];
        let mx = [self.max.x, self.max.y, self.max.z];
        for i in 0..3 {
            if dd[i].abs() < 1e-8 {
                if oo[i] < mn[i] || oo[i] > mx[i] {
                    return None;
                }
            } else {
                let inv = 1.0 / dd[i];
                let mut t0 = (mn[i] - oo[i]) * inv;
                let mut t1 = (mx[i] - oo[i]) * inv;
                let mut sign = -1.0;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                    sign = 1.0;
                }
                if t0 > tmin {
                    tmin = t0;
                    n = V3::ZERO;
                    match i {
                        0 => n.x = sign,
                        1 => n.y = sign,
                        _ => n.z = sign,
                    }
                }
                if t1 < tmx {
                    tmx = t1;
                }
                if tmin > tmx {
                    return None;
                }
            }
        }
        if tmin <= 0.0 {
            return None;
        }
        Some((tmin, n))
    }
}

// ----------------------------------------------------------------- prisms --

/// A convex footprint extruded from `y0` to `y1`. With `ramp` set, the top is the plane
/// `a + gx·x + gz·z` (clamped to [y0, y1]) — that's how stairs are walked.
#[derive(Clone)]
pub struct Shape {
    #[allow(dead_code)]
    pub pts: Vec<(f32, f32)>,
    /// edge planes (nx, nz, d): inside where nx·x + nz·z <= d
    pub eq: Vec<(f32, f32, f32)>,
    pub y0: f32,
    pub y1: f32,
    pub lo: (f32, f32),
    pub hi: (f32, f32),
    pub flags: u8,
    pub ramp: Option<(f32, f32, f32)>,
}

/// Twice the signed area of a polygon in the xz plane (positive = counter-clockwise).
pub fn poly_area2(p: &[(f32, f32)]) -> f32 {
    let mut a = 0.0;
    for i in 0..p.len() {
        let (x0, z0) = p[i];
        let (x1, z1) = p[(i + 1) % p.len()];
        a += x0 * z1 - x1 * z0;
    }
    a
}

impl Shape {
    pub fn new(mut pts: Vec<(f32, f32)>, y0: f32, y1: f32, flags: u8) -> Shape {
        if poly_area2(&pts) < 0.0 {
            pts.reverse();
        }
        let mut eq = Vec::with_capacity(pts.len());
        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            let (dx, dz) = (b.0 - a.0, b.1 - a.1);
            let l = (dx * dx + dz * dz).sqrt();
            if l > 1e-5 {
                let (nx, nz) = (dz / l, -dx / l);
                eq.push((nx, nz, nx * a.0 + nz * a.1));
            }
            lo = (lo.0.min(a.0), lo.1.min(a.1));
            hi = (hi.0.max(a.0), hi.1.max(a.1));
        }
        Shape { pts, eq, y0, y1, lo, hi, flags, ramp: None }
    }

    pub fn boxed(min: V3, max: V3, flags: u8) -> Shape {
        Shape::new(vec![(min.x, min.z), (max.x, min.z), (max.x, max.z), (min.x, max.z)], min.y, max.y, flags)
    }

    /// Largest signed distance from the footprint's edges (negative inside) and that edge's normal.
    #[inline]
    pub fn sep(&self, x: f32, z: f32) -> (f32, (f32, f32)) {
        let mut best = (f32::MIN, (0.0, 0.0));
        for &(nx, nz, d) in &self.eq {
            let s = nx * x + nz * z - d;
            if s > best.0 {
                best = (s, (nx, nz));
            }
        }
        best
    }

    #[inline]
    fn near_xz(&self, x: f32, z: f32, r: f32) -> bool {
        x + r > self.lo.0 && x - r < self.hi.0 && z + r > self.lo.1 && z - r < self.hi.1
    }

    /// Height of the top surface at (x, z).
    #[inline]
    pub fn top_at(&self, x: f32, z: f32) -> f32 {
        match self.ramp {
            Some((a, gx, gz)) => (a + gx * x + gz * z).clamp(self.y0, self.y1),
            None => self.y1,
        }
    }

    /// The highest part of the top that a circle of radius r at (x, z) touches.
    #[inline]
    fn top_reach(&self, x: f32, z: f32, r: f32) -> f32 {
        match self.ramp {
            Some((a, gx, gz)) => (a + gx * x + gz * z + r * (gx * gx + gz * gz).sqrt()).clamp(self.y0, self.y1),
            None => self.y1,
        }
    }

    /// Ray vs prism (Cyrus–Beck). Entry distance and normal; rays that start inside are ignored.
    pub fn ray(&self, o: V3, d: V3, tmax: f32) -> Option<(f32, V3)> {
        let mut t0 = 0.0f32;
        let mut t1 = tmax;
        let mut n = V3::ZERO;
        if d.y.abs() < 1e-8 {
            if o.y < self.y0 || o.y > self.y1 {
                return None;
            }
        } else {
            let inv = 1.0 / d.y;
            let (mut a, mut b) = ((self.y0 - o.y) * inv, (self.y1 - o.y) * inv);
            let mut ny = -1.0;
            if a > b {
                std::mem::swap(&mut a, &mut b);
                ny = 1.0;
            }
            if a > t0 {
                t0 = a;
                n = v3(0.0, ny, 0.0);
            }
            t1 = t1.min(b);
            if t0 > t1 {
                return None;
            }
        }
        for &(nx, nz, dd) in &self.eq {
            let den = nx * d.x + nz * d.z;
            let num = dd - (nx * o.x + nz * o.z);
            if den.abs() < 1e-9 {
                if num < 0.0 {
                    return None;
                }
                continue;
            }
            let t = num / den;
            if den < 0.0 {
                if t > t0 {
                    t0 = t;
                    n = v3(nx, 0.0, nz);
                }
            } else if t < t1 {
                t1 = t;
            }
            if t0 > t1 {
                return None;
            }
        }
        if t0 <= 0.0 {
            return None;
        }
        Some((t0, n))
    }
}

// ------------------------------------------------------------------ doors --

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DoorKind {
    Gate,
    SideGate,
    Main,
    Cell,
    BlastRear,
    BlastWall,
}

#[derive(Clone)]
pub struct Door {
    pub b: Aabb,
    pub kind: DoorKind,
    pub open: f32,
    pub opening: bool,
    pub slide: V3,   // how far it travels when fully open
    pub fence: bool, // chain-link: you can see and shoot through it (but not walk through it)
    pub locked: bool,
    pub col: [f32; 4],
}

impl Door {
    pub fn eff(&self) -> Aabb {
        let o = self.slide * self.open;
        Aabb { min: self.b.min + o, max: self.b.max + o }
    }
    pub fn solid(&self) -> bool {
        self.open < 0.9
    }
}

/// A padlocked latch: shoot it and its door swings free.
#[derive(Clone)]
pub struct Latch {
    pub pos: V3,
    pub half: V3,
    pub door: usize,
    pub broken: bool,
    /// which way the latch plate faces (for drawing)
    pub n: V3,
}

impl Latch {
    pub fn aabb(&self) -> Aabb {
        Aabb { min: self.pos - self.half, max: self.pos + self.half }
    }
}

pub struct RayHit {
    pub t: f32,
    pub n: V3,
    pub terrain: bool,
}

// ------------------------------------------------------------------ world --

const GRID_X0: f32 = -204.0;
const GRID_CS: f32 = 6.0;
const GRID_N: i32 = 68;

pub struct World {
    pub shapes: Vec<Shape>,
    pub doors: Vec<Door>,
    cells: Vec<Vec<u32>>,
    stamp: RefCell<Vec<u32>>,
    stamp_n: Cell<u32>,
}

impl World {
    pub fn new(shapes: Vec<Shape>, doors: Vec<Door>) -> World {
        let mut cells = vec![Vec::new(); (GRID_N * GRID_N) as usize];
        for (i, s) in shapes.iter().enumerate() {
            let (ix0, iz0) = cell_of(s.lo.0, s.lo.1);
            let (ix1, iz1) = cell_of(s.hi.0, s.hi.1);
            for ix in ix0..=ix1 {
                for iz in iz0..=iz1 {
                    cells[(iz * GRID_N + ix) as usize].push(i as u32);
                }
            }
        }
        let n = shapes.len();
        World { shapes, doors, cells, stamp: RefCell::new(vec![0; n]), stamp_n: Cell::new(0) }
    }

    fn next_stamp(&self) -> u32 {
        let mut s = self.stamp_n.get().wrapping_add(1);
        if s == 0 {
            self.stamp.borrow_mut().iter_mut().for_each(|v| *v = 0);
            s = 1;
        }
        self.stamp_n.set(s);
        s
    }

    /// Call `f` once for every shape whose grid cells overlap the square (x±r, z±r).
    pub fn visit<F: FnMut(&Shape)>(&self, x: f32, z: f32, r: f32, mut f: F) {
        let (ix0, iz0) = cell_of(x - r, z - r);
        let (ix1, iz1) = cell_of(x + r, z + r);
        if ix0 == ix1 && iz0 == iz1 {
            for &si in &self.cells[(iz0 * GRID_N + ix0) as usize] {
                f(&self.shapes[si as usize]);
            }
            return;
        }
        let sid = self.next_stamp();
        let mut st = self.stamp.borrow_mut();
        for iz in iz0..=iz1 {
            for ix in ix0..=ix1 {
                for &si in &self.cells[(iz * GRID_N + ix) as usize] {
                    let k = si as usize;
                    if st[k] != sid {
                        st[k] = sid;
                        f(&self.shapes[k]);
                    }
                }
            }
        }
    }

    /// True if an upright cylinder (feet at `p`) overlaps something it can't simply step onto.
    pub fn blocked(&self, p: V3, r: f32, h: f32) -> bool {
        let mut hit = false;
        self.visit(p.x, p.z, r, |s| {
            if !hit
                && s.flags & BLOCK != 0
                && s.y0 < p.y + h
                && s.near_xz(p.x, p.z, r)
                && s.top_reach(p.x, p.z, r) > p.y + STEP
                && s.sep(p.x, p.z).0 < r
            {
                hit = true;
            }
        });
        if hit {
            return true;
        }
        self.doors.iter().any(|d| {
            let b = d.eff();
            d.solid() && b.overlaps_xz(p.x, p.z, r) && b.max.y > p.y + STEP && b.min.y < p.y + h
        })
    }

    /// Push an upright cylinder out of everything it is stuck in, sliding along walls at any angle.
    /// Returns true if it had to be moved.
    pub fn resolve(&self, p: &mut V3, r: f32, h: f32) -> bool {
        let mut moved = false;
        for _ in 0..4 {
            let mut push: Option<(f32, f32, f32)> = None; // deepest penetration first
            let q = *p;
            self.visit(q.x, q.z, r + 0.05, |s| {
                if s.flags & BLOCK == 0 || s.y0 >= q.y + h || !s.near_xz(q.x, q.z, r) {
                    return;
                }
                if s.top_reach(q.x, q.z, r) <= q.y + STEP {
                    return;
                }
                let (sep, (nx, nz)) = s.sep(q.x, q.z);
                let pen = r - sep;
                if pen > 0.0 && push.map(|b| pen > b.0).unwrap_or(true) {
                    push = Some((pen, nx, nz));
                }
            });
            for d in &self.doors {
                let b = d.eff();
                if !d.solid() || b.max.y <= q.y + STEP || b.min.y >= q.y + h {
                    continue;
                }
                let (sep, (nx, nz)) = b.sep(q.x, q.z);
                let pen = r - sep;
                if pen > 0.0 && push.map(|b| pen > b.0).unwrap_or(true) {
                    push = Some((pen, nx, nz));
                }
            }
            match push {
                Some((pen, nx, nz)) => {
                    p.x += nx * (pen + 0.001);
                    p.z += nz * (pen + 0.001);
                    moved = true;
                }
                None => break,
            }
        }
        moved
    }

    /// Highest walkable surface under (x,z) that can be reached from `feet` (within `step`).
    pub fn ground_at(&self, x: f32, z: f32, r: f32, feet: f32, step: f32) -> f32 {
        let mut g = terrain_h(x, z);
        let lim = feet + step;
        self.visit(x, z, r, |s| {
            if s.flags & WALK == 0 || s.y0 > lim || !s.near_xz(x, z, r) {
                return;
            }
            let top = s.top_at(x, z);
            if top <= lim && top > g && s.sep(x, z).0 < r {
                g = top;
            }
        });
        for d in &self.doors {
            if d.solid() && !d.fence {
                let b = d.eff();
                if b.max.y <= lim && b.max.y > g && b.overlaps_xz(x, z, r) {
                    g = b.max.y;
                }
            }
        }
        g
    }

    /// Lowest underside above `head` over (x,z); INFINITY if open sky.
    pub fn ceiling_at(&self, x: f32, z: f32, r: f32, head: f32) -> f32 {
        let mut c = f32::INFINITY;
        self.visit(x, z, r, |s| {
            if s.flags & BLOCK != 0 && s.y0 >= head - 0.05 && s.y0 < c && s.near_xz(x, z, r) && s.sep(x, z).0 < r {
                c = s.y0;
            }
        });
        c
    }

    fn ray_shapes(&self, o: V3, d: V3, best: &mut f32, mask: u8) -> Option<(f32, V3)> {
        let mut hit = None;
        let fx = (o.x - GRID_X0) / GRID_CS;
        let fz = (o.z - GRID_X0) / GRID_CS;
        let mut ix = fx.floor() as i32;
        let mut iz = fz.floor() as i32;
        if ix < 0 || iz < 0 || ix >= GRID_N || iz >= GRID_N {
            return None;
        }
        let (sx, sz) = (if d.x >= 0.0 { 1 } else { -1 }, if d.z >= 0.0 { 1 } else { -1 });
        let tdx = if d.x.abs() > 1e-9 { GRID_CS / d.x.abs() } else { f32::INFINITY };
        let tdz = if d.z.abs() > 1e-9 { GRID_CS / d.z.abs() } else { f32::INFINITY };
        let mut tmx = if d.x > 1e-9 {
            ((ix + 1) as f32 - fx) * tdx
        } else if d.x < -1e-9 {
            (fx - ix as f32) * tdx
        } else {
            f32::INFINITY
        };
        let mut tmz = if d.z > 1e-9 {
            ((iz + 1) as f32 - fz) * tdz
        } else if d.z < -1e-9 {
            (fz - iz as f32) * tdz
        } else {
            f32::INFINITY
        };
        let sid = self.next_stamp();
        let mut st = self.stamp.borrow_mut();
        loop {
            for &si in &self.cells[(iz * GRID_N + ix) as usize] {
                let k = si as usize;
                if st[k] == sid {
                    continue;
                }
                st[k] = sid;
                let s = &self.shapes[k];
                if s.flags & mask == 0 {
                    continue;
                }
                if let Some((t, n)) = s.ray(o, d, *best) {
                    *best = t;
                    hit = Some((t, n));
                }
            }
            let tn = tmx.min(tmz);
            if tn > *best {
                break;
            }
            if tmx < tmz {
                ix += sx;
                tmx += tdx;
            } else {
                iz += sz;
                tmz += tdz;
            }
            if ix < 0 || iz < 0 || ix >= GRID_N || iz >= GRID_N {
                break;
            }
        }
        hit
    }

    /// What a bullet hits.
    pub fn raycast(&self, o: V3, d: V3, maxd: f32) -> Option<RayHit> {
        self.raycast_mask(o, d, maxd, RAY)
    }

    pub fn raycast_mask(&self, o: V3, d: V3, maxd: f32, mask: u8) -> Option<RayHit> {
        let mut bt = maxd;
        let mut best: Option<RayHit> = None;
        if let Some((t, n)) = self.ray_shapes(o, d, &mut bt, mask) {
            best = Some(RayHit { t, n, terrain: false });
        }
        for dr in &self.doors {
            if dr.solid() && !dr.fence {
                if let Some((t, n)) = dr.eff().ray(o, d, bt) {
                    bt = t;
                    best = Some(RayHit { t, n, terrain: false });
                }
            }
        }
        // terrain march
        let lim = bt.min(480.0);
        let mut t = 0.0;
        if o.y > terrain_h(o.x, o.z) {
            while t < lim {
                let step = clamp((o.y + d.y * t - terrain_h(o.x + d.x * t, o.z + d.z * t)) * 0.6, 0.5, 3.0);
                let prev_t = t;
                t += step;
                let p = o + d * t;
                if p.y < terrain_h(p.x, p.z) {
                    let (mut lo, mut hi) = (prev_t, t);
                    for _ in 0..8 {
                        let mid = (lo + hi) * 0.5;
                        let q = o + d * mid;
                        if q.y < terrain_h(q.x, q.z) {
                            hi = mid;
                        } else {
                            lo = mid;
                        }
                    }
                    if hi < bt {
                        let q = o + d * hi;
                        best = Some(RayHit { t: hi, n: terrain_normal(q.x, q.z), terrain: true });
                    }
                    break;
                }
            }
        }
        best
    }

    /// Can an eye at `a` see the point `b`? (Foliage hides things too.)
    pub fn los(&self, a: V3, b: V3) -> bool {
        let d = b - a;
        let l = d.len();
        if l < 0.01 {
            return true;
        }
        self.raycast_mask(a, d / l, l - 0.25, RAY | SIGHT).is_none()
    }
}

fn cell_of(x: f32, z: f32) -> (i32, i32) {
    (
        (((x - GRID_X0) / GRID_CS).floor() as i32).clamp(0, GRID_N - 1),
        (((z - GRID_X0) / GRID_CS).floor() as i32).clamp(0, GRID_N - 1),
    )
}
