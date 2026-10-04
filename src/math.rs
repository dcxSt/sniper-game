use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub const Y: V3 = v3(0.0, 1.0, 0.0);
    #[inline]
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    #[inline]
    pub fn len2(self) -> f32 {
        self.dot(self)
    }
    #[inline]
    pub fn len(self) -> f32 {
        self.len2().sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            V3::ZERO
        }
    }
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self + (o - self) * t
    }
    pub fn xz(self) -> V3 {
        v3(self.x, 0.0, self.z)
    }
    pub fn dist(self, o: V3) -> f32 {
        (self - o).len()
    }
    pub fn dist_xz(self, o: V3) -> f32 {
        (self - o).xz().len()
    }
}

impl Add for V3 {
    type Output = V3;
    #[inline]
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    #[inline]
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    #[inline]
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for V3 {
    type Output = V3;
    #[inline]
    fn div(self, s: f32) -> V3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for V3 {
    type Output = V3;
    #[inline]
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for V3 {
    fn add_assign(&mut self, o: V3) {
        *self = *self + o;
    }
}
impl SubAssign for V3 {
    fn sub_assign(&mut self, o: V3) {
        *self = *self - o;
    }
}
impl MulAssign<f32> for V3 {
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

pub type M4 = [f32; 16];

pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> M4 {
    let f = 1.0 / (fovy * 0.5).tan();
    let mut m = [0.0; 16];
    m[0] = f / aspect;
    m[5] = f;
    m[10] = (far + near) / (near - far);
    m[11] = -1.0;
    m[14] = 2.0 * far * near / (near - far);
    m
}

pub fn identity() -> M4 {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn ortho(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> M4 {
    let mut m = [0.0; 16];
    m[0] = 2.0 / (r - l);
    m[5] = 2.0 / (t - b);
    m[10] = -2.0 / (f - n);
    m[12] = -(r + l) / (r - l);
    m[13] = -(t + b) / (t - b);
    m[14] = -(f + n) / (f - n);
    m[15] = 1.0;
    m
}

pub fn look_at(eye: V3, target: V3, up: V3) -> M4 {
    let f = (target - eye).norm();
    let s = f.cross(up).norm();
    let u = s.cross(f);
    [
        s.x,
        u.x,
        -f.x,
        0.0,
        s.y,
        u.y,
        -f.y,
        0.0,
        s.z,
        u.z,
        -f.z,
        0.0,
        -s.dot(eye),
        -u.dot(eye),
        f.dot(eye),
        1.0,
    ]
}

pub fn mat_mul(a: &M4, b: &M4) -> M4 {
    let mut o = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += a[k * 4 + r] * b[c * 4 + k];
            }
            o[c * 4 + r] = s;
        }
    }
    o
}

/// Project a world point; returns (ndc_x, ndc_y, w) when in front of the camera.
pub fn project(vp: &M4, p: V3) -> Option<(f32, f32, f32)> {
    let x = vp[0] * p.x + vp[4] * p.y + vp[8] * p.z + vp[12];
    let y = vp[1] * p.x + vp[5] * p.y + vp[9] * p.z + vp[13];
    let w = vp[3] * p.x + vp[7] * p.y + vp[11] * p.z + vp[15];
    if w > 0.05 {
        Some((x / w, y / w, w))
    } else {
        None
    }
}

/// yaw 0 looks toward -Z, positive yaw turns toward +X.
pub fn fwd_from(yaw: f32) -> V3 {
    v3(yaw.sin(), 0.0, -yaw.cos())
}
pub fn right_from(yaw: f32) -> V3 {
    v3(yaw.cos(), 0.0, yaw.sin())
}
pub fn look_dir(yaw: f32, pitch: f32) -> V3 {
    v3(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
}
pub fn yaw_to(v: V3) -> f32 {
    v.x.atan2(-v.z)
}

pub fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

pub fn clamp(x: f32, a: f32, b: f32) -> f32 {
    x.max(a).min(b)
}

pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    pub fn u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }
    pub fn f(&mut self) -> f32 {
        (self.u32() >> 8) as f32 / 16777216.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
}
