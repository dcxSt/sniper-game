//! The stick-figure body: one pose function (walk, crouch, aim, look around) that produces joint
//! positions, a drawing routine for any set of joints, and a Verlet ragdoll that takes over from the
//! live pose the moment someone dies — so bodies crumple, fold over walls and slide down stairs.

use crate::gfx::{Col, Mesh};
use crate::math::*;
use crate::world::*;

pub const PELVIS: usize = 0;
pub const NECK: usize = 1;
pub const HEAD: usize = 2;
const HIP_L: usize = 3;
const KNEE_L: usize = 4;
const FOOT_L: usize = 5;
const TOE_L: usize = 6;
const HIP_R: usize = 7;
const KNEE_R: usize = 8;
const FOOT_R: usize = 9;
const TOE_R: usize = 10;
const SH_L: usize = 11;
const ELB_L: usize = 12;
const HAND_L: usize = 13;
const SH_R: usize = 14;
const ELB_R: usize = 15;
const HAND_R: usize = 16;
pub const NJ: usize = 17;

const BONES: [(usize, usize); 16] = [
    (PELVIS, NECK),
    (NECK, HEAD),
    (PELVIS, HIP_L),
    (HIP_L, KNEE_L),
    (KNEE_L, FOOT_L),
    (FOOT_L, TOE_L),
    (PELVIS, HIP_R),
    (HIP_R, KNEE_R),
    (KNEE_R, FOOT_R),
    (FOOT_R, TOE_R),
    (NECK, SH_L),
    (SH_L, ELB_L),
    (ELB_L, HAND_L),
    (NECK, SH_R),
    (SH_R, ELB_R),
    (ELB_R, HAND_R),
];

#[derive(Clone)]
pub struct Pose {
    pub j: [V3; NJ],
    /// grip end, muzzle, stock end
    pub gun: Option<[V3; 3]>,
}

/// Everything the pose function needs to know about a figure.
pub struct Fig {
    pub pos: V3,
    pub yaw: f32,
    pub twist: f32, // upper body turned relative to the legs (looking around)
    pub s: f32,
    pub phase: f32,
    pub moving: f32,
    pub aim: Option<V3>,
    pub crouch: f32,
    pub flinch: f32,
    pub gun: u8, // 0 none, 1 pistol, 2 rifle, 3 shotgun
    pub arms_up: bool,
}

pub fn gun_dims(gun: u8) -> (f32, f32) {
    match gun {
        1 => (0.34, 0.05),
        2 => (0.95, 0.055),
        _ => (0.9, 0.07),
    }
}

pub fn pose(f: &Fig) -> Pose {
    let s = f.s;
    let c = f.crouch.clamp(0.0, 1.0);
    let (fwd, right) = (fwd_from(f.yaw), right_from(f.yaw));
    let (fwd_u, right_u) = (fwd_from(f.yaw + f.twist), right_from(f.yaw + f.twist));
    let th = f.flinch * 0.2; // stagger back when hit
    let (sn, cs) = th.sin_cos();
    let tf = |lr: f32, ly: f32, lf: f32| -> V3 {
        let ff = lf * cs - ly * sn;
        let yy = lf * sn + ly * cs;
        f.pos + right * lr + fwd * ff + v3(0.0, yy.max(0.04), 0.0)
    };
    let tu = |lr: f32, ly: f32, lf: f32| -> V3 {
        let ff = lf * cs - ly * sn;
        let yy = lf * sn + ly * cs;
        f.pos + right_u * lr + fwd_u * ff + v3(0.0, yy.max(0.04), 0.0)
    };
    let mut j = [V3::ZERO; NJ];
    let hipy = lerpf(0.88, 0.54, c) * s;
    let hipf = lerpf(0.0, -0.06, c) * s;
    let amp = 0.34 * s * f.moving * (1.0 - 0.45 * c);
    let ph = f.phase;
    for (k, (hi, kn, fo, to)) in [(-1.0f32, (HIP_L, KNEE_L, FOOT_L, TOE_L)), (1.0, (HIP_R, KNEE_R, FOOT_R, TOE_R))] {
        let a = ph + if k > 0.0 { 0.0 } else { std::f32::consts::PI };
        let sw = a.sin() * amp;
        let lift = a.cos().max(0.0) * 0.17 * s * f.moving * (1.0 - 0.5 * c);
        j[hi] = tf(k * 0.08 * s, hipy, hipf);
        let ks = (k * 0.10 * s, 0.46 * s + lift * 0.6, sw * 0.5 + 0.11 * s * f.moving);
        let kc = (k * 0.14 * s, 0.50 * s + lift * 0.4, 0.33 * s + sw * 0.4);
        j[kn] = tf(lerpf(ks.0, kc.0, c), lerpf(ks.1, kc.1, c), lerpf(ks.2, kc.2, c));
        j[fo] = tf(k * 0.11 * s, lift, sw + 0.02 * c * s);
        j[to] = tf(k * 0.11 * s, lift, sw + 0.15 * s);
    }
    j[PELVIS] = tf(0.0, hipy, hipf);
    let neck_y = lerpf(1.46, 1.08, c) * s;
    let neck_f = lerpf(0.0, 0.17, c) * s;
    j[NECK] = tu(0.0, neck_y, neck_f);
    j[HEAD] = tu(0.0, lerpf(1.66, 1.28, c) * s, lerpf(0.02, 0.23, c) * s);
    let sh_y = lerpf(1.38, 1.01, c) * s;
    let sh_f = lerpf(0.0, 0.15, c) * s;
    j[SH_L] = tu(-0.07 * s, sh_y, sh_f);
    j[SH_R] = tu(0.07 * s, sh_y, sh_f);
    let mut gun = None;
    if f.gun > 0 {
        let aim_w = f.aim.unwrap_or_else(|| (fwd_u + v3(0.0, -0.35, 0.0)).norm());
        let a = (aim_w.dot(right_u), aim_w.y, aim_w.dot(fwd_u));
        let sh_l = (0.0, sh_y, sh_f);
        let hr = (sh_l.0 + a.0 * 0.5 * s + 0.07 * s, sh_l.1 + a.1 * 0.5 * s, sh_l.2 + a.2 * 0.5 * s);
        let (glen, _) = gun_dims(f.gun);
        let reach = (0.30 + glen * 0.55) * s;
        let hl = (sh_l.0 + a.0 * reach - 0.05 * s, sh_l.1 + a.1 * reach, sh_l.2 + a.2 * reach);
        j[HAND_R] = tu(hr.0, hr.1, hr.2);
        j[HAND_L] = tu(hl.0, hl.1, hl.2);
        j[ELB_R] = tu(hr.0 * 0.5 + 0.1 * s, (hr.1 + sh_l.1) * 0.5 - 0.1 * s, (hr.2 + sh_l.2) * 0.5);
        j[ELB_L] = tu(hl.0 * 0.5 - 0.12 * s, (hl.1 + sh_l.1) * 0.5 - 0.09 * s, (hl.2 + sh_l.2) * 0.5);
        let g0 = tu(hr.0 - a.0 * 0.14 * s, hr.1 - a.1 * 0.14 * s, hr.2 - a.2 * 0.14 * s);
        let g1 = tu(hr.0 + a.0 * glen * s, hr.1 + a.1 * glen * s, hr.2 + a.2 * glen * s);
        let st = tu(hr.0 - a.0 * 0.3 * s, hr.1 - a.1 * 0.3 * s - 0.05 * s, hr.2 - a.2 * 0.3 * s);
        gun = Some([g0, g1, st]);
    } else {
        for (k, (el, ha)) in [(-1.0f32, (ELB_L, HAND_L)), (1.0, (ELB_R, HAND_R))] {
            let a = ph + if k > 0.0 { std::f32::consts::PI } else { 0.0 };
            if f.arms_up {
                j[ha] = tu(k * 0.18 * s, lerpf(1.85, 1.45, c) * s, 0.05 * s);
                j[el] = tu(k * 0.16 * s, lerpf(1.60, 1.22, c) * s, 0.0);
            } else {
                j[ha] = tu(k * 0.13 * s, lerpf(0.83, 0.6, c) * s, a.sin() * 0.24 * s * f.moving + 0.2 * c * s);
                j[el] = tu(k * 0.12 * s, lerpf(1.12, 0.8, c) * s, a.sin() * 0.10 * s * f.moving + 0.15 * c * s);
            }
        }
    }
    Pose { j, gun }
}

/// Hit flash: briefly lights a black figure up.
pub fn tint(c: Col, k: f32) -> Col {
    [lerpf(c[0], 0.9, k), lerpf(c[1], 0.9, k), lerpf(c[2], 0.9, k), c[3]]
}

pub struct Look {
    pub body: Col,
    pub head: Col,
    pub s: f32,
    pub flash: f32,
    pub vest: Option<Col>,
    pub gun: u8,
    pub muzzle: bool,
}

pub fn draw_pose(m: &mut Mesh, fx: &mut Mesh, cam: V3, p: &Pose, l: &Look) {
    let s = l.s;
    let bc = tint(l.body, l.flash);
    let t = 0.08 * s;
    let j = &p.j;
    for (a, b) in BONES {
        if a == NECK && b == HEAD {
            continue;
        }
        let th = if a == PELVIS && b == NECK {
            t * 1.25
        } else if b == TOE_L || b == TOE_R {
            t * 0.9
        } else if a == NECK || a == SH_L || a == SH_R || a == ELB_L || a == ELB_R {
            if a == NECK {
                t * 0.8
            } else {
                t * 0.9
            }
        } else {
            t
        };
        m.limb(j[a], j[b], th, bc);
    }
    if let Some(vc) = l.vest {
        let d = j[NECK] - j[PELVIS];
        m.limb(j[PELVIS] + d * 0.08, j[PELVIS] + d * 0.92, t * 3.2, tint(vc, l.flash));
    }
    m.ellipsoid(j[HEAD], v3(0.19 * s, 0.19 * s, 0.19 * s), tint(l.head, l.flash), 8, 12);
    if let Some([g0, g1, st]) = p.gun {
        let gun_col = [0.10, 0.10, 0.115, 0.0];
        let (_, gth) = gun_dims(l.gun);
        m.limb(g0, g1, gth * s, gun_col);
        if l.gun >= 2 {
            m.limb(g0, st, gth * 1.4 * s, gun_col);
        }
        if l.muzzle {
            let dn = (g1 - g0).norm();
            fx.ribbon(g1, g1 + dn * 0.9, 0.45, cam, [1.0, 0.9, 0.6, 0.9]);
            fx.ribbon(g1 - dn * 0.1, g1 + dn * 0.5, 0.8, cam, [1.0, 0.75, 0.4, 0.45]);
        }
    }
}

// ---------------------------------------------------------------- ragdoll --

#[derive(Clone, Copy)]
struct Link {
    a: usize,
    b: usize,
    len: f32,
    min_only: bool,
}

pub struct Ragdoll {
    p: [V3; NJ],
    q: [V3; NJ],
    g: [V3; 3],
    gq: [V3; 3],
    has_gun: bool,
    links: Vec<Link>,
    glinks: [(usize, usize, f32); 3],
    acc: f32,
    still: f32,
    pub t: f32,
    pub asleep: bool,
}

const RD_DT: f32 = 1.0 / 120.0;
const RAD: f32 = 0.06;

impl Ragdoll {
    /// Take over from a live pose; `vel` is how the body was already moving.
    pub fn new(pose: &Pose, vel: V3) -> Ragdoll {
        let p = pose.j;
        let mut q = p;
        for k in q.iter_mut() {
            *k -= vel * RD_DT;
        }
        let mut links: Vec<Link> = BONES.iter().map(|&(a, b)| Link { a, b, len: p[a].dist(p[b]), min_only: false }).collect();
        // keep the torso a rigid box and the feet at an angle to the shins
        for (a, b) in [
            (HIP_L, HIP_R),
            (SH_L, SH_R),
            (HIP_L, NECK),
            (HIP_R, NECK),
            (SH_L, PELVIS),
            (SH_R, PELVIS),
            (SH_L, HIP_L),
            (SH_R, HIP_R),
            (HEAD, SH_L),
            (HEAD, SH_R),
            (KNEE_L, TOE_L),
            (KNEE_R, TOE_R),
        ] {
            links.push(Link { a, b, len: p[a].dist(p[b]), min_only: false });
        }
        // joints can bend but not fold flat
        let leg_l = p[HIP_L].dist(p[KNEE_L]) + p[KNEE_L].dist(p[FOOT_L]);
        let leg_r = p[HIP_R].dist(p[KNEE_R]) + p[KNEE_R].dist(p[FOOT_R]);
        let arm_l = p[SH_L].dist(p[ELB_L]) + p[ELB_L].dist(p[HAND_L]);
        let arm_r = p[SH_R].dist(p[ELB_R]) + p[ELB_R].dist(p[HAND_R]);
        links.push(Link { a: HIP_L, b: FOOT_L, len: leg_l * 0.55, min_only: true });
        links.push(Link { a: HIP_R, b: FOOT_R, len: leg_r * 0.55, min_only: true });
        links.push(Link { a: SH_L, b: HAND_L, len: arm_l * 0.4, min_only: true });
        links.push(Link { a: SH_R, b: HAND_R, len: arm_r * 0.4, min_only: true });
        links.push(Link { a: HEAD, b: PELVIS, len: p[HEAD].dist(p[PELVIS]) * 0.92, min_only: true });
        links.push(Link { a: KNEE_L, b: KNEE_R, len: 0.14, min_only: true });
        links.push(Link { a: PELVIS, b: KNEE_L, len: p[PELVIS].dist(p[KNEE_L]) * 0.75, min_only: true });
        links.push(Link { a: PELVIS, b: KNEE_R, len: p[PELVIS].dist(p[KNEE_R]) * 0.75, min_only: true });
        let (g, has_gun) = match pose.gun {
            Some(g) => (g, true),
            None => ([V3::ZERO; 3], false),
        };
        let mut gq = g;
        for k in gq.iter_mut() {
            *k -= vel * RD_DT;
        }
        let glinks = [(0, 1, g[0].dist(g[1])), (0, 2, g[0].dist(g[2])), (1, 2, g[1].dist(g[2]))];
        Ragdoll { p, q, g, gq, has_gun, links, glinks, acc: 0.0, still: 0.0, t: 0.0, asleep: false }
    }

    /// A bullet's shove: joints near `at` get kicked along `dir`.
    pub fn impulse(&mut self, at: V3, dir: V3, speed: f32) {
        for i in 0..NJ {
            let d2 = self.p[i].dist(at).powi(2);
            let w = (-d2 / 0.18).exp();
            self.q[i] -= dir * (speed * w * RD_DT);
        }
        // the whole body staggers a little with it
        for i in 0..NJ {
            self.q[i] -= dir * (speed * 0.18 * RD_DT);
        }
        if self.has_gun {
            for k in 0..3 {
                self.gq[k] -= (dir * 0.5 + v3(0.0, 0.6, 0.0)) * (speed * 0.4 * RD_DT);
            }
        }
        self.asleep = false;
        self.still = 0.0;
    }

    pub fn pelvis(&self) -> V3 {
        self.p[PELVIS]
    }

    fn collide(w: &World, p: &mut V3, q: &mut V3) {
        let g = w.ground_at(p.x, p.z, 0.04, p.y + 0.25, 0.0);
        if p.y < g + RAD {
            p.y = g + RAD;
            // friction: bleed off sliding
            q.x = lerpf(q.x, p.x, 0.55);
            q.z = lerpf(q.z, p.z, 0.55);
            if q.y < p.y {
                q.y = lerpf(q.y, p.y, 0.6);
            }
        }
    }

    pub fn update(&mut self, dt: f32, w: &World) {
        if self.asleep {
            return;
        }
        self.t += dt;
        self.acc += dt;
        while self.acc >= RD_DT {
            self.acc -= RD_DT;
            self.step(w);
        }
        if self.t > 9.0 {
            self.asleep = true;
        }
    }

    fn step(&mut self, w: &World) {
        let grav = v3(0.0, -18.0 * RD_DT * RD_DT, 0.0);
        let mut moved = 0.0f32;
        for i in 0..NJ {
            let v = (self.p[i] - self.q[i]) * 0.99;
            self.q[i] = self.p[i];
            self.p[i] += v + grav;
        }
        if self.has_gun {
            for k in 0..3 {
                let v = (self.g[k] - self.gq[k]) * 0.99;
                self.gq[k] = self.g[k];
                self.g[k] += v + grav;
            }
        }
        for _ in 0..6 {
            for l in &self.links {
                let d = self.p[l.b] - self.p[l.a];
                let len = d.len();
                if len < 1e-6 || (l.min_only && len >= l.len) {
                    continue;
                }
                let corr = d * (0.5 * (len - l.len) / len);
                self.p[l.a] += corr;
                self.p[l.b] -= corr;
            }
            if self.has_gun {
                for &(a, b, rest) in &self.glinks {
                    let d = self.g[b] - self.g[a];
                    let len = d.len();
                    if len > 1e-6 {
                        let corr = d * (0.5 * (len - rest) / len);
                        self.g[a] += corr;
                        self.g[b] -= corr;
                    }
                }
            }
            for i in 0..NJ {
                let (mut p, mut q) = (self.p[i], self.q[i]);
                Ragdoll::collide(w, &mut p, &mut q);
                self.p[i] = p;
                self.q[i] = q;
            }
        }
        // walls and anything tall: push joints out sideways
        for i in 0..NJ {
            let mut p = self.p[i] - v3(0.0, 0.3, 0.0);
            if w.resolve(&mut p, 0.07, 0.35) {
                self.p[i].x = p.x;
                self.p[i].z = p.z;
            }
            moved = moved.max((self.p[i] - self.q[i]).len());
        }
        if self.has_gun {
            for k in 0..3 {
                let (mut p, mut q) = (self.g[k], self.gq[k]);
                Ragdoll::collide(w, &mut p, &mut q);
                let mut pp = p - v3(0.0, 0.3, 0.0);
                if w.resolve(&mut pp, 0.05, 0.35) {
                    p.x = pp.x;
                    p.z = pp.z;
                }
                self.g[k] = p;
                self.gq[k] = q;
            }
        }
        if moved < 0.0025 {
            self.still += RD_DT;
            if self.still > 0.6 {
                self.asleep = true;
            }
        } else {
            self.still = 0.0;
        }
    }

    pub fn pose(&self) -> Pose {
        Pose { j: self.p, gun: if self.has_gun { Some(self.g) } else { None } }
    }
}
