use crate::body::*;
use crate::gfx::*;
use crate::math::*;
use crate::weapons::*;
use crate::world::*;
use crate::*;

/// Fog melts into the horizon colour, so distant hills dissolve instead of ending in a hard edge.
pub const FOG_COL: [f32; 3] = [0.905, 0.905, 0.89];
const FOG_DEN: f32 = 0.0040;
/// Direction *to* the sun: low and from the east, so the world gets long, soft side-light.
pub const SUN: V3 = v3(0.70, 0.62, 0.30);

/// Orthographic view from the sun over the playable area (the prison, the ridge and the approach).
pub fn light_matrix() -> M4 {
    let l = SUN.norm();
    let centre = v3(4.0, 6.0, 26.0);
    let view = look_at(centre + l * 300.0, centre, V3::Y);
    let r = 158.0;
    mat_mul(&ortho(-r, r, -r, r, 40.0, 620.0), &view)
}

/// A clean, soft sky: pale horizon, a touch of blue overhead, and a gentle glow where the sun is.
pub fn build_sky() -> Mesh {
    let mut m = Mesh::default();
    let (rings, seg, rad) = (36usize, 72usize, 450.0f32);
    let horizon = [0.905f32, 0.905, 0.89];
    let zenith = [0.64f32, 0.75, 0.87];
    let sun = SUN.norm();
    let col = |d: V3| -> Col {
        let e = d.y.clamp(-1.0, 1.0).asin();
        let k = if e <= 0.0 { 0.0 } else { e.sin().powf(0.55) };
        let mut c = [
            lerpf(horizon[0], zenith[0], k),
            lerpf(horizon[1], zenith[1], k),
            lerpf(horizon[2], zenith[2], k),
        ];
        let ang = d.dot(sun).clamp(-1.0, 1.0).acos();
        let glow = (-(ang / 0.34).powi(2)).exp() * 0.30 + (-(ang / 0.06).powi(2)).exp() * 0.55;
        let warm = [1.0, 0.96, 0.88];
        for i in 0..3 {
            c[i] = (c[i] + warm[i] * glow).min(1.0);
        }
        [c[0], c[1], c[2], 1.0]
    };
    let dir = |e: f32, a: f32| v3(e.cos() * a.cos(), e.sin(), e.cos() * a.sin());
    for i in 0..rings {
        let t0 = i as f32 / rings as f32;
        let t1 = (i + 1) as f32 / rings as f32;
        // more rings near the horizon, where the gradient is
        let e0 = lerpf(-0.30, std::f32::consts::FRAC_PI_2, t0 * t0);
        let e1 = lerpf(-0.30, std::f32::consts::FRAC_PI_2, t1 * t1);
        for j in 0..seg {
            let a0 = std::f32::consts::TAU * j as f32 / seg as f32;
            let a1 = std::f32::consts::TAU * (j + 1) as f32 / seg as f32;
            let (d00, d01, d10, d11) = (dir(e0, a0), dir(e0, a1), dir(e1, a0), dir(e1, a1));
            let n = V3::Y;
            m.vert(d00 * rad, n, col(d00));
            m.vert(d10 * rad, n, col(d10));
            m.vert(d11 * rad, n, col(d11));
            m.vert(d00 * rad, n, col(d00));
            m.vert(d11 * rad, n, col(d11));
            m.vert(d01 * rad, n, col(d01));
        }
    }
    m
}

impl Game {
    fn camera(&mut self) -> (V3, V3, V3, f32) {
        // returns eye, dir, up, fov(rad)
        if self.state == MENU {
            let a = self.anim_t * 0.07;
            let c = v3(0.0, 4.0, -8.0);
            let eye = c + v3(a.sin() * 105.0, 34.0 + (self.anim_t * 0.2).sin() * 5.0, a.cos() * 105.0);
            let dir = (c - eye).norm();
            return (eye, dir, V3::Y, 62.0f32.to_radians());
        }
        let p = &self.player;
        let (yaw, pitch) = self.view_angles();
        let mut eye = self.eye();
        let sp = clamp(p.speed / 5.0, 0.0, 1.0) * if p.grounded { 1.0 } else { 0.0 };
        eye.y += (p.bob * 2.0).sin() * 0.03 * sp * (1.0 - p.ads_t);
        let mut roll = 0.0;
        if self.state == DEAD {
            let k = (self.death_t * 2.0).min(1.0);
            eye.y -= 0.85 * k;
            roll = 0.7 * k;
        }
        let sh = p.shake * (self.cur_fov_deg() / BASE_FOV).sqrt();
        let t = self.anim_t;
        let yaw = yaw + (t * 61.0).sin() * sh * 0.012;
        let pitch = pitch + (t * 47.0 + 1.0).sin() * sh * 0.012;
        let dir = look_dir(yaw, pitch);
        let right = dir.cross(V3::Y).norm();
        let up0 = right.cross(dir).norm();
        let up = (up0 * roll.cos() + right * roll.sin()).norm();
        (eye, dir, up, self.cur_fov_deg().to_radians())
    }

    pub(crate) fn render(&mut self, _dt: f32) {
        let (w, h) = (self.r.w as f32, self.r.h as f32);
        let aspect = w / h.max(1.0);
        let (eye, dir, up, fov) = self.camera();
        let proj = perspective(fov, aspect, 0.1, 800.0);
        let view = look_at(eye, eye + dir, up);
        let vp = mat_mul(&proj, &view);
        self.vp = vp;
        self.cam_pos = eye;
        self.cam_right = dir.cross(up).norm();
        self.cam_up = self.cam_right.cross(dir).norm();

        self.build_dynamic();

        let r = &self.r;
        r.upload_all(
            &mut self.b_dyn,
            &self.m_dyn,
            &mut self.b_alpha,
            &self.m_alpha,
            &mut self.b_add,
            &self.m_add,
        );
        r.upload(&mut self.b_fence_dyn, &self.m_fence.v, true);

        r.begin_frame(FOG_COL);
        r.set_scene(&vp, eye, SUN.norm(), FOG_COL, FOG_DEN);
        r.set_indoor(true, self.indoor_min, self.indoor_max);
        r.set_shadow(&self.light_vp, true);
        r.set_offset(eye);
        r.draw(&self.b_sky, Mode::Sky);
        r.set_offset(V3::ZERO);
        r.draw(&self.b_static, Mode::Lit);
        r.draw(&self.b_dyn, Mode::Lit);
        r.draw(&self.b_fence, Mode::Fence);
        r.draw(&self.b_fence_dyn, Mode::Fence);
        r.draw(&self.b_alpha, Mode::Alpha);
        r.draw(&self.b_add, Mode::Additive);

        // first-person weapon
        if self.state != MENU && self.scope_amount() < 0.97 {
            self.build_viewmodel();
            let r = &self.r;
            r.upload(&mut self.b_view, &self.m_view.v, true);
            r.upload(&mut self.b_view_add, &self.m_view_add.v, true);
            r.clear_depth();
            let vproj = perspective(62.0f32.to_radians(), aspect, 0.02, 10.0);
            r.set_scene(&vproj, V3::ZERO, v3(0.35, 0.8, 0.5).norm(), FOG_COL, 0.0);
            r.set_indoor(false, V3::ZERO, V3::ZERO);
            r.set_shadow_on(false);
            r.draw(&self.b_view, Mode::View);
            r.draw(&self.b_view_add, Mode::Additive);
        }

        // looking down the scope: black out everything outside the lens (done on the GPU)
        let sa = self.scope_amount();
        if self.state != MENU && sa > 0.01 {
            let r = &self.r;
            r.set_aux(aspect, sa);
            r.draw(&self.b_scope, Mode::Scope);
        }

        self.draw_hud();
    }

    fn build_dynamic(&mut self) {
        self.m_dyn.clear();
        self.m_alpha.clear();
        self.m_add.clear();
        self.m_fence.clear();
        let cam = self.cam_pos;
        let (cr, cu) = (self.cam_right, self.cam_up);
        let t = self.anim_t;

        // doors
        for d in &self.world.doors {
            let b = d.eff();
            if d.open >= 0.999 && d.slide.y < 0.0 {
                continue;
            }
            if d.fence {
                // chain-link panel: runs along x or along z
                let along_z = (b.max.z - b.min.z) > (b.max.x - b.min.x);
                let (xc, zc) = ((b.min.x + b.max.x) * 0.5, (b.min.z + b.max.z) * 0.5);
                let (a, e, uoff) = if along_z {
                    (v3(xc, 0.0, b.min.z), v3(xc, 0.0, b.max.z), d.slide.z * d.open)
                } else {
                    (v3(b.min.x, 0.0, zc), v3(b.max.x, 0.0, zc), d.slide.x * d.open)
                };
                self.m_fence.fence_quad(a, e, b.min.y, b.max.y, d.slide.y * d.open, uoff);
                let c = d.col;
                let (tx, tz) = if along_z { (0.07, 0.06) } else { (0.06, 0.07) };
                for p in [a, e] {
                    self.m_dyn.cuboid(v3(p.x - tx, b.min.y, p.z - tz), v3(p.x + tx, b.max.y + 0.05, p.z + tz), c);
                }
                let (rx, rz) = if along_z { (0.05, 0.0) } else { (0.0, 0.05) };
                self.m_dyn.cuboid(v3(a.x - rx, b.max.y - 0.07, a.z - rz), v3(e.x + rx, b.max.y + 0.04, e.z + rz), c);
                self.m_dyn.cuboid(v3(a.x - rx, b.min.y, a.z - rz), v3(e.x + rx, b.min.y + 0.1, e.z + rz), c);
                self.m_dyn.limb(a + v3(0.0, b.min.y + 0.1, 0.0), e + v3(0.0, b.max.y - 0.1, 0.0), 0.05, c);
            } else {
                match d.kind {
                    DoorKind::Cell => {
                        let n = 8;
                        let wdt = b.max.x - b.min.x;
                        for i in 0..n {
                            let x = b.min.x + (i as f32 + 0.5) * wdt / n as f32;
                            self.m_dyn.cuboid(v3(x - 0.03, b.min.y, b.min.z), v3(x + 0.03, b.max.y, b.max.z), d.col);
                        }
                        for yy in [0.2, 1.4, 2.7] {
                            self.m_dyn.cuboid(v3(b.min.x, b.min.y + yy, b.min.z), v3(b.max.x, b.min.y + yy + 0.07, b.max.z), d.col);
                        }
                    }
                    _ => self.m_dyn.cuboid(b.min, b.max, d.col),
                }
            }
            // status lamp: red while locked, green once open
            if matches!(d.kind, DoorKind::Gate | DoorKind::SideGate | DoorKind::Main | DoorKind::Cell) {
                let col = if d.locked { [1.0, 0.12, 0.08, 1.0] } else { [0.2, 1.0, 0.3, 1.0] };
                let (lp, out) = match d.kind {
                    DoorKind::Gate => (v3(d.b.min.x - 0.25, 2.9, d.b.max.z + 0.02), v3(0.0, 0.0, 1.0)),
                    DoorKind::SideGate => (v3(d.b.min.x - 0.02, 2.9, d.b.max.z + 0.25), v3(-1.0, 0.0, 0.0)),
                    DoorKind::Cell => (v3((d.b.min.x + d.b.max.x) * 0.5, d.b.max.y + 0.3, d.b.max.z + 0.02), v3(0.0, 0.0, 1.0)),
                    _ => (v3((d.b.min.x + d.b.max.x) * 0.5, d.b.max.y + 0.35, d.b.max.z + 0.02), v3(0.0, 0.0, 1.0)),
                };
                self.m_dyn.cuboid_c(lp, v3(0.11, 0.1, 0.11), col);
                let ac = if d.locked { [1.0, 0.2, 0.1, 0.22] } else { [0.3, 0.9, 0.4, 0.22] };
                self.m_add.glow(lp + out * 0.25, cr, cu, 0.3, ac);
            }
            // demolition charges
            if matches!(d.kind, DoorKind::BlastRear | DoorKind::BlastWall) && self.charges_t > 0.0 && d.open < 0.01 {
                let cx = (d.b.min.x + d.b.max.x) * 0.5;
                let blink = (t * 7.0).sin() > 0.0;
                for dx in [-2.0f32, 0.0, 2.0] {
                    self.m_dyn.cuboid(v3(cx + dx - 0.25, 1.2, d.b.max.z), v3(cx + dx + 0.25, 1.7, d.b.max.z + 0.2), [0.5, 0.45, 0.3, 0.0]);
                    if blink {
                        self.m_dyn.cuboid(v3(cx + dx - 0.06, 1.55, d.b.max.z + 0.2), v3(cx + dx + 0.06, 1.65, d.b.max.z + 0.26), [1.0, 0.1, 0.1, 1.0]);
                        self.m_add.glow(v3(cx + dx, 1.6, d.b.max.z + 0.4), cr, cu, 0.5, [1.0, 0.1, 0.1, 0.6]);
                    }
                }
            }
        }

        // padlocked latches: a chunky steel box with a shackle and a red tell-tale, pulsing when it's the target
        for (i, l) in self.latches.iter().enumerate() {
            if l.broken {
                continue;
            }
            let c = l.pos;
            self.m_dyn.cuboid(c - l.half, c + l.half, [0.16, 0.17, 0.19, 0.0]);
            let up = v3(0.0, l.half.y, 0.0);
            let side = v3(l.n.z, 0.0, -l.n.x) * (l.half.x.max(l.half.z) * 0.55);
            self.m_dyn.limb(c + up - side, c + up * 1.9 - side * 0.6, 0.05, [0.55, 0.56, 0.58, 0.0]);
            self.m_dyn.limb(c + up * 1.9 - side * 0.6, c + up * 1.9 + side * 0.6, 0.05, [0.55, 0.56, 0.58, 0.0]);
            self.m_dyn.limb(c + up * 1.9 + side * 0.6, c + up + side, 0.05, [0.55, 0.56, 0.58, 0.0]);
            let face = c + l.n * (l.half.x.min(l.half.z) + 0.01);
            self.m_dyn.cuboid_c(face, v3(0.05, 0.05, 0.05), [1.0, 0.25, 0.08, 1.0]);
            let target = self.latch_is_target(i);
            let pulse = if target { 0.5 + 0.5 * (t * 4.0).sin() } else { 0.0 };
            self.m_add.glow(face + l.n * 0.1, cr, cu, 0.32 + 0.25 * pulse, [1.0, 0.45, 0.15, 0.35 + 0.35 * pulse]);
        }

        // enemies
        for e in &self.enemies {
            if e.pos.dist(cam) > 300.0 {
                continue;
            }
            e.draw(&mut self.m_dyn, &mut self.m_add, &mut self.m_alpha, cam);
        }

        // prisoner
        {
            let pr = &self.prisoner;
            if pr.mode < 2 {
                let p = pose(&Fig {
                    pos: pr.pos,
                    yaw: pr.yaw,
                    twist: 0.0,
                    s: 1.0,
                    phase: pr.phase,
                    moving: if pr.mode == 1 { 1.0 } else { 0.0 },
                    aim: None,
                    crouch: 0.0,
                    flinch: 0.0,
                    gun: 0,
                    arms_up: pr.mode == 0,
                });
                let look = Look { body: [0.98, 0.40, 0.08, 0.0], head: [0.02, 0.02, 0.025, 0.0], s: 1.0, flash: 0.0, vest: None, gun: 0, muzzle: false };
                draw_pose(&mut self.m_dyn, &mut self.m_add, cam, &p, &look);
                self.m_alpha.blob(pr.pos + v3(0.0, 0.05, 0.0), 0.85, 0.30);
                if pr.mode == 0 {
                    let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                    self.m_add.glow(pr.pos + v3(0.0, 2.45, 0.0), cr, cu, 0.25, [1.0, 0.8, 0.1, 0.5 + 0.4 * pulse]);
                }
            }
        }

        // pickups
        for pk in &self.pickups {
            let bob = (pk.t * 2.2).sin() * 0.1;
            let c = pk.pos + v3(0.0, 0.55 + bob, 0.0);
            match pk.kind {
                PKind::Health => {
                    self.m_dyn.cuboid_c(c, v3(0.3, 0.2, 0.3), [0.95, 0.95, 0.95, 0.15]);
                    self.m_dyn.cuboid_c(c + v3(0.0, 0.2, 0.0), v3(0.2, 0.02, 0.07), [0.9, 0.1, 0.1, 0.6]);
                    self.m_dyn.cuboid_c(c + v3(0.0, 0.2, 0.0), v3(0.07, 0.025, 0.2), [0.9, 0.1, 0.1, 0.6]);
                    self.m_add.cylinder(pk.pos, 0.28, 9.0, [0.30, 0.85, 0.50, 0.12], 10);
                    self.m_add.glow(c, cr, cu, 0.7, [0.35, 0.85, 0.50, 0.22]);
                }
                PKind::Ammo(_) => {
                    self.m_dyn.cuboid_c(c, v3(0.28, 0.16, 0.2), [0.22, 0.24, 0.27, 0.0]);
                    self.m_dyn.cuboid_c(c, v3(0.06, 0.17, 0.21), [0.93, 0.45, 0.20, 0.3]);
                    self.m_add.glow(c, cr, cu, 0.5, [1.0, 0.85, 0.3, 0.2]);
                }
            }
            self.m_alpha.blob(pk.pos + v3(0.0, 0.04, 0.0), 0.7, 0.26);
        }

        // objective beacon
        if let Some(p) = self.objective_pos() {
            let pulse = 0.14 + 0.05 * (t * 3.0).sin();
            let (h, p) = if self.stage < 2 { (26.0, p + v3(0.0, 2.6, 0.0)) } else { (70.0, p) };
            self.m_add.cylinder(p, 0.5, h, [1.0, 0.62, 0.30, pulse * 0.8], 12);
            self.m_add.glow(p + v3(0.0, 1.0, 0.0), cr, cu, 1.4, [1.0, 0.62, 0.30, 0.18]);
        }

        // decals
        for d in &self.decals {
            let n = d.n;
            let u = n.cross(if n.y.abs() > 0.9 { v3(1.0, 0.0, 0.0) } else { V3::Y }).norm();
            let v = n.cross(u);
            let c = d.p + n * 0.015;
            let s = 0.07;
            let a = (1.0 - (d.t / 40.0)).clamp(0.0, 1.0) * 0.75;
            self.m_alpha.quad_n(c - u * s - v * s, c + u * s - v * s, c + u * s + v * s, c - u * s + v * s, n, [0.04, 0.04, 0.04, a]);
        }

        // particles
        for p in &self.parts {
            let a = (p.life / p.max).clamp(0.0, 1.0);
            let s = p.size * if p.add { a } else { 0.6 + 0.8 * (1.0 - a) };
            let col = [p.col[0], p.col[1], p.col[2], a * if p.add { 0.9 } else { 0.7 }];
            if p.add {
                self.m_add.glow(p.p, cr, cu, s * 1.3, col);
            } else {
                // soft round puffs, never hard squares
                self.m_alpha.glow(p.p, cr, cu, s * 1.5, col);
            }
        }

        // tracers
        for tr in &self.tracers {
            let a = tr.t / tr.max;
            if tr.a.dist(tr.b) < 2.0 {
                continue;
            }
            let col = [tr.col[0], tr.col[1], tr.col[2], a * 0.85];
            self.m_add.ribbon(tr.a, tr.b, tr.w, cam, col);
        }
    }

    fn build_viewmodel(&mut self) {
        self.m_view.clear();
        self.m_view_add.clear();
        let p = &self.player;
        let wi = p.cur;
        let def = &WEAPONS[wi];
        let ads = p.ads_t * p.ads_t * (3.0 - 2.0 * p.ads_t);
        let hip = v3(0.17, -0.18, -0.58);
        let adsp = if wi == 0 { v3(0.0, -0.1, -0.5) } else { v3(0.0, -0.085, -0.52) };
        let mut base = hip.lerp(adsp, ads);
        let sp = clamp(p.speed / 5.0, 0.0, 1.0) * (1.0 - ads * 0.85) * if p.grounded { 1.0 } else { 0.0 };
        base.x += p.bob.sin() * 0.012 * sp;
        base.y += (p.bob * 2.0).cos().abs() * 0.012 * sp - 0.006 * sp;
        let mut rot = p.recoil_anim * 0.13;
        base.z += p.recoil_anim * 0.07;
        // raising the rifle to your eye: it dips out of the way as the scope fades in
        let sa = self.scope_amount();
        base.y -= 0.22 * sa;
        rot -= 0.25 * sa;
        if p.reload_t > 0.0 {
            let prog = 1.0 - p.reload_t / def.reload;
            let dip = (prog * std::f32::consts::PI).sin();
            base.y -= 0.2 * dip;
            base.x -= 0.05 * dip;
            rot -= 0.55 * dip;
        }
        if p.switch_t > 0.0 {
            let k = p.switch_t / 0.45;
            base.y -= 0.45 * k * k;
            rot -= 0.4 * k;
        }
        let (sn, cs) = rot.sin_cos();
        let tf = |q: V3| -> V3 {
            let y = q.y * cs - q.z * sn;
            let z = q.y * sn + q.z * cs;
            base + v3(q.x, y, z) * 0.64
        };
        for (a, b, t, c) in gun_bones(wi) {
            self.m_view.limb(tf(a), tf(b), t, c);
        }
        // stick-man arms
        let skin = [0.03, 0.03, 0.035, 0.0];
        let grip = tf(v3(0.0, -0.1, 0.07));
        let fore = match wi {
            2 => tf(v3(-0.01, -0.09, 0.02)),
            3 => tf(v3(0.0, -0.05, -0.52)),
            0 => tf(v3(0.0, -0.03, -0.3)),
            _ => tf(v3(0.0, -0.06, -0.3)),
        };
        let r_start = v3(0.42, -0.5, 0.3);
        let l_start = v3(-0.4, -0.5, 0.2);
        self.m_view.limb(r_start, grip, 0.05, skin);
        self.m_view.limb(l_start, fore, 0.05, skin);
        self.m_view.cuboid_c(grip, v3(0.032, 0.032, 0.032), skin);
        self.m_view.cuboid_c(fore, v3(0.032, 0.032, 0.032), skin);

        if p.muzzle > 0.0 {
            let m = tf(muzzle_pos(wi));
            let k = p.muzzle / 0.06;
            let sz = if wi == 0 { 0.2 } else { 0.12 };
            self.m_view_add.glow(m, v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), sz * 1.3 * k, [1.0, 0.9, 0.5, 0.95]);
            self.m_view_add.billboard(m, v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), sz * 2.0 * k, sz * 0.7 * k, [1.0, 0.6, 0.2, 0.6]);
            self.m_view_add.billboard(m, v3(0.7, 0.0, 0.7), v3(0.0, 1.0, 0.0), sz * 1.4 * k, sz * 1.4 * k, [1.0, 0.75, 0.3, 0.5]);
        }
    }
}

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub fn upload_all(&self, b1: &mut Buf, m1: &Mesh, b2: &mut Buf, m2: &Mesh, b3: &mut Buf, m3: &Mesh) {
        self.upload(b1, &m1.v, true);
        self.upload(b2, &m2.v, true);
        self.upload(b3, &m3.v, true);
    }
}
