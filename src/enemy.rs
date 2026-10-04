//! Stick-man AI. Guards patrol and look around; what they notice depends on how squarely they're looking
//! at you. A bullet cracking past makes them duck, turn toward where it came from and search. In a fight
//! they move between cover along the nav grid, crouch out of sight and pop up to shoot.

use crate::body::*;
use crate::gfx::{Col, Mesh};
use crate::math::*;
use crate::nav::*;
use crate::world::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Guard,
    Rifleman,
    Heavy,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Patrol,
    Investigate,
    Search,
    Combat,
    Dead,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tactic {
    Engage,
    ToCover,
    Cover,
    Peek,
}

impl Kind {
    pub fn hp(self) -> f32 {
        match self {
            Kind::Guard => 100.0,
            Kind::Rifleman => 120.0,
            Kind::Heavy => 320.0,
        }
    }
    pub fn scale(self) -> f32 {
        match self {
            Kind::Heavy => 1.22,
            _ => 1.0,
        }
    }
    pub fn walk(self) -> f32 {
        match self {
            Kind::Guard => 1.5,
            Kind::Rifleman => 1.3,
            Kind::Heavy => 1.1,
        }
    }
    pub fn run(self) -> f32 {
        match self {
            Kind::Guard => 3.8,
            Kind::Rifleman => 3.3,
            Kind::Heavy => 2.9,
        }
    }
    pub fn sight(self) -> f32 {
        match self {
            Kind::Guard => 62.0,
            Kind::Rifleman => 105.0,
            Kind::Heavy => 52.0,
        }
    }
    pub fn range(self) -> f32 {
        match self {
            Kind::Guard => 45.0,
            Kind::Rifleman => 100.0,
            Kind::Heavy => 22.0,
        }
    }
    /// (min, max) preferred distance in combat
    pub fn pref(self) -> (f32, f32) {
        match self {
            Kind::Guard => (8.0, 22.0),
            Kind::Rifleman => (18.0, 50.0),
            Kind::Heavy => (3.0, 8.0),
        }
    }
    /// (base hit chance, loss per metre)
    pub fn acc(self) -> (f32, f32) {
        match self {
            Kind::Guard => (0.60, 0.013),
            Kind::Rifleman => (0.58, 0.0085),
            Kind::Heavy => (0.70, 0.03),
        }
    }
    pub fn dmg(self) -> f32 {
        match self {
            Kind::Guard => 8.0,
            Kind::Rifleman => 8.0,
            Kind::Heavy => 5.5, // per pellet
        }
    }
    pub fn gun(self) -> u8 {
        match self {
            Kind::Guard => 1,
            Kind::Rifleman => 2,
            Kind::Heavy => 3,
        }
    }
}

pub struct PlayerView {
    pub pos: V3,
    pub eye: V3,
    pub crouch: bool,
    pub vis: f32,
    pub speed: f32,
    pub alive: bool,
}

pub enum Ev {
    Shot { from: V3, to: V3, hit: bool, dmg: f32, kind: Kind },
    Shout { pos: V3, target: V3 },
    /// someone stumbled on a dead colleague
    Body { pos: V3 },
}

pub struct Ctx<'a> {
    pub world: &'a World,
    pub nav: &'a Nav,
    pub p: &'a PlayerView,
    pub rng: &'a mut Rng,
    pub ev: &'a mut Vec<Ev>,
    /// (id, position) of every dead enemy
    pub corpses: &'a [(usize, V3)],
    /// path / cover searches still allowed this tick (they're the expensive part)
    pub budget: &'a mut i32,
    /// cover spots already taken by someone
    pub claims: &'a [V3],
}

pub struct Enemy {
    pub kind: Kind,
    pub pos: V3,
    pub yaw: f32,
    pub hp: f32,
    pub state: State,
    pub zone: usize,
    pub patrol: Vec<V3>,
    pi: usize,
    wait: f32,
    home: V3,
    home_yaw: f32,
    pub stationary: bool,
    pub aware: f32,
    pub last_seen: V3,
    pub lost_t: f32,
    inv_t: f32,
    pub target: V3,
    shoot_cd: f32,
    burst: i32,
    pub sees: bool,
    pub phase: f32,
    pub moving: f32,
    pub crouch: f32,
    want_crouch: bool,
    pub dead_t: f32,
    pub flash: f32,
    pub muzzle: f32,
    pub hp_show: f32,
    pub aim_dir: V3,
    pub look_off: f32, // head / upper-body turn relative to the legs: where they're actually looking
    pub flinch: f32,
    seen_bodies: Vec<usize>,
    perc_t: f32,
    // navigation
    path: Vec<V3>,
    path_i: usize,
    path_goal: V3,
    repath_t: f32,
    stuck_t: f32,
    strafe_dir: f32,
    vel: V3,
    // fighting and searching
    pub tactic: Tactic,
    pub cover: Option<V3>,
    cover_t: f32,
    peek_t: f32,
    peek_shots: i32,
    tactic_t: f32,
    search_t: f32,
    search_phase: u8,
    search_hops: u8,
    startle_t: f32,
    pub alert: f32,
    look_t: f32,
    shots: i32,
    seed: f32,
    pub ragdoll: Option<Ragdoll>,
    pub vis_y: f32,
    /// the player can see this one right now (HUD markers), and how recently they spotted them (radar)
    pub in_view: bool,
    pub spotted: f32,
    unreachable: f32,
    searched: bool,
    exposed_t: f32,
}

fn turn_toward(cur: f32, target: f32, max: f32) -> f32 {
    let d = angle_diff(cur, target);
    cur + clamp(d, -max, max)
}

impl Enemy {
    pub fn new(kind: Kind, pos: V3, yaw: f32, patrol: Vec<V3>, zone: usize, rng: &mut Rng) -> Enemy {
        let stationary = patrol.is_empty();
        Enemy {
            kind,
            pos,
            yaw,
            hp: kind.hp(),
            state: State::Patrol,
            zone,
            patrol,
            pi: 0,
            wait: rng.range(0.0, 2.0),
            home: pos,
            home_yaw: yaw,
            stationary,
            aware: 0.0,
            last_seen: pos,
            lost_t: 0.0,
            inv_t: 0.0,
            target: pos,
            shoot_cd: 0.0,
            burst: 3,
            sees: false,
            phase: rng.range(0.0, 6.0),
            moving: 0.0,
            crouch: 0.0,
            want_crouch: false,
            dead_t: 0.0,
            flash: 0.0,
            muzzle: 0.0,
            hp_show: 0.0,
            aim_dir: fwd_from(yaw),
            look_off: 0.0,
            flinch: 0.0,
            seen_bodies: vec![],
            perc_t: rng.range(0.0, 0.2),
            path: vec![],
            path_i: 0,
            path_goal: pos,
            repath_t: 0.0,
            stuck_t: 0.0,
            strafe_dir: if rng.chance(0.5) { 1.0 } else { -1.0 },
            vel: V3::ZERO,
            tactic: Tactic::Engage,
            cover: None,
            cover_t: 0.0,
            peek_t: 0.0,
            peek_shots: 0,
            tactic_t: 0.0,
            search_t: 0.0,
            search_phase: 0,
            search_hops: 0,
            startle_t: 0.0,
            alert: 0.0,
            look_t: 0.0,
            shots: 0,
            seed: rng.range(0.0, 6.28),
            ragdoll: None,
            vis_y: pos.y,
            in_view: false,
            spotted: 0.0,
            unreachable: 0.0,
            searched: false,
            exposed_t: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.state != State::Dead
    }

    /// Marksmen posted up high stay put and fight from where they are.
    pub fn holds_post(&self) -> bool {
        self.stationary && self.kind == Kind::Rifleman
    }

    fn base(&self) -> V3 {
        v3(self.pos.x, self.vis_y, self.pos.z)
    }

    pub fn eye(&self) -> V3 {
        let s = self.kind.scale();
        self.base() + v3(0.0, lerpf(1.6, 1.22, self.crouch) * s, 0.0) + fwd_from(self.yaw + self.look_off) * (0.2 * self.crouch * s)
    }

    pub fn head_center(&self) -> V3 {
        let s = self.kind.scale();
        let c = self.crouch;
        self.base() + v3(0.0, lerpf(1.66, 1.28, c) * s, 0.0) + fwd_from(self.yaw + self.look_off) * (lerpf(0.02, 0.23, c) * s)
    }

    pub fn head_radius(&self) -> f32 {
        0.25 * self.kind.scale()
    }

    pub fn body_box(&self) -> Aabb {
        let s = self.kind.scale();
        let b = self.base() + fwd_from(self.yaw) * (0.08 * self.crouch * s);
        Aabb { min: b + v3(-0.3 * s, -0.02, -0.3 * s), max: b + v3(0.3 * s, lerpf(1.5, 1.08, self.crouch) * s, 0.3 * s) }
    }

    fn make_pose(&self) -> Pose {
        let aim = if self.state == State::Combat && self.sees && self.tactic != Tactic::Cover { Some(self.aim_dir) } else { None };
        pose(&Fig {
            pos: self.base(),
            yaw: self.yaw,
            twist: self.look_off,
            s: self.kind.scale(),
            phase: self.phase,
            moving: self.moving,
            aim,
            crouch: self.crouch,
            flinch: self.flinch,
            gun: self.kind.gun(),
            arms_up: false,
        })
    }

    /// Returns true if this hit killed the enemy.
    pub fn damage(&mut self, dmg: f32, from: V3) -> bool {
        if self.state == State::Dead {
            return false;
        }
        self.hp -= dmg;
        self.flash = 1.0;
        self.hp_show = 3.0;
        self.flinch = 1.0;
        if self.hp <= 0.0 {
            let pose = self.make_pose();
            self.ragdoll = Some(Ragdoll::new(&pose, self.vel * self.moving));
            self.state = State::Dead;
            self.dead_t = 0.0;
            self.muzzle = 0.0;
            self.cover = None;
            return true;
        }
        self.alert = 1.0;
        // a bullet in the body says exactly where the shooter is: turn on them right away
        if self.state != State::Combat {
            self.state = State::Combat;
            self.last_seen = from;
            self.aware = 1.2;
            self.shoot_cd = 0.55;
            self.lost_t = 0.0;
            self.shots = 0;
            self.tactic = if self.holds_post() { Tactic::Cover } else { Tactic::Engage };
            self.cover_t = 0.8;
            self.tactic_t = 0.0;
        } else if self.tactic == Tactic::Peek {
            // duck back down
            self.tactic = Tactic::Cover;
            self.cover_t = 1.4;
        } else if self.tactic == Tactic::Engage {
            self.tactic_t = 0.0;
        }
        false
    }

    /// Knock a fresh corpse along the bullet's path.
    pub fn shove(&mut self, at: V3, dir: V3, speed: f32) {
        if let Some(r) = self.ragdoll.as_mut() {
            r.impulse(at, dir, speed);
        }
    }

    pub fn hear(&mut self, pos: V3, rng: &mut Rng) {
        match self.state {
            State::Patrol | State::Investigate => {
                self.state = State::Investigate;
                self.target = pos + v3(rng.range(-3.0, 3.0), 0.0, rng.range(-3.0, 3.0));
                self.inv_t = 8.0;
                self.aware = self.aware.max(0.4);
            }
            State::Search => {
                self.target = pos + v3(rng.range(-2.0, 2.0), 0.0, rng.range(-2.0, 2.0));
                self.search_t = self.search_t.max(10.0);
            }
            _ => {}
        }
    }

    pub fn alarm(&mut self, target: V3) {
        if self.alive() {
            if self.state != State::Combat {
                self.tactic = if self.holds_post() { Tactic::Cover } else { Tactic::Engage };
                self.tactic_t = 0.3;
                self.cover_t = 0.6;
            }
            self.state = State::Combat;
            self.last_seen = target;
            self.aware = 1.0;
            self.shoot_cd = 0.8;
            self.lost_t = 0.0;
            self.alert = 1.0;
        }
    }

    /// A shot cracked past or smacked into something nearby: duck, turn toward the shooter, then search.
    pub fn startle(&mut self, shooter: V3, rng: &mut Rng) {
        if !self.alive() {
            return;
        }
        self.alert = 1.0;
        let d = shooter.dist(self.pos);
        let err = d * 0.1;
        let guess = shooter + v3(rng.range(-err, err), 0.0, rng.range(-err, err));
        if self.state == State::Combat {
            if !self.sees {
                self.last_seen = guess;
            }
            if self.tactic == Tactic::Peek || self.tactic == Tactic::Engage {
                self.tactic = if self.holds_post() { Tactic::Cover } else { Tactic::Engage };
                self.tactic_t = 0.0;
                self.cover_t = 1.2;
            }
            return;
        }
        self.state = State::Search;
        self.target = guess;
        self.last_seen = guess;
        self.startle_t = rng.range(0.5, 0.9);
        self.search_t = 18.0;
        self.search_phase = 0;
        self.search_hops = 0;
        self.tactic_t = rng.range(2.5, 4.5);
        self.cover = None;
        self.aware = self.aware.max(0.65);
        self.flinch = self.flinch.max(0.6);
    }

    fn can_stand(&self, w: &World, p: V3) -> bool {
        let s = self.kind.scale();
        let r = 0.33 * s;
        if w.blocked(p, r, 1.75 * s) {
            return false;
        }
        let g = w.ground_at(p.x, p.z, r, self.pos.y, STEP);
        self.pos.y - g < 0.7
    }

    /// Steer toward `goal` with a few feelers. Returns whether we moved.
    fn walk_to(&mut self, goal: V3, speed: f32, dt: f32, w: &World, face_move: bool) -> bool {
        let to = (goal - self.pos).xz();
        let dist = to.len();
        if dist < 0.03 {
            return false;
        }
        let dir = to / dist;
        let speed = speed * if self.crouch > 0.5 { 0.6 } else { 1.0 };
        let step = (speed * dt).min(dist);
        for a in [0.0f32, 0.5, -0.5, 1.1, -1.1, 1.8, -1.8] {
            let a = a * self.strafe_dir;
            let (sn, cs) = a.sin_cos();
            let d = v3(dir.x * cs - dir.z * sn, 0.0, dir.x * sn + dir.z * cs);
            let np = self.pos + d * step;
            let probe = self.pos + d * (step + 0.4);
            if self.can_stand(w, np) && self.can_stand(w, probe) {
                self.pos.x = np.x;
                self.pos.z = np.z;
                let s = self.kind.scale();
                self.pos.y = w.ground_at(np.x, np.z, 0.33 * s, self.pos.y, STEP);
                if face_move {
                    self.yaw = turn_toward(self.yaw, yaw_to(d), 5.0 * dt);
                }
                self.phase += dt * speed * 3.4 / s;
                self.moving = (self.moving + dt * 6.0).min(1.0);
                self.vel = d * speed;
                return true;
            }
        }
        false
    }

    /// Walk along the nav grid toward `goal`. Returns true on arrival.
    fn go(&mut self, goal: V3, speed: f32, dt: f32, c: &mut Ctx, face: bool) -> bool {
        let d = self.pos.dist_xz(goal);
        if d < 0.5 && (self.pos.y - goal.y).abs() < 1.3 {
            self.path.clear();
            return true;
        }
        self.repath_t -= dt;
        let moved = self.path_goal.dist(goal) > 1.5;
        if moved {
            self.searched = false;
        }
        let stale = moved || self.repath_t <= 0.0 || (self.path.is_empty() && !self.searched);
        if stale && *c.budget > 0 {
            *c.budget -= 1;
            self.searched = true;
            self.path = if d < 9.0 && walk_ok(c.world, self.pos, goal, NAV_R) {
                vec![goal]
            } else {
                match c.nav.path(self.pos, goal, 5000, &c.world.doors) {
                    Some(p) => p,
                    None => {
                        // no way there (locked gate, fence): don't walk into the wall
                        self.unreachable = 2.0;
                        vec![]
                    }
                }
            };
            if !self.path.is_empty() {
                self.unreachable = 0.0;
            }
            self.path_i = 0;
            self.path_goal = goal;
            self.repath_t = 3.0 + c.rng.range(0.0, 1.5);
        }
        if self.path_i < self.path.len() {
            let wp = self.path[self.path_i];
            if self.pos.dist_xz(wp) < 0.45 && (self.pos.y - wp.y).abs() < 1.3 {
                self.path_i += 1;
                // cut the corner when the way ahead is clear
                if self.path_i + 1 < self.path.len() && walk_ok(c.world, self.pos, self.path[self.path_i + 1], NAV_R) {
                    self.path_i += 1;
                }
                if self.path_i >= self.path.len() {
                    return d < 0.8;
                }
            }
        }
        if self.path.is_empty() && self.unreachable > 0.0 {
            // no way there right now: wait for the next look rather than walking into a wall
            return false;
        }
        let wp = if self.path_i < self.path.len() { self.path[self.path_i] } else { goal };
        let moved = self.walk_to(wp, speed, dt, c.world, face);
        if !moved {
            self.stuck_t += dt;
            if self.stuck_t > 0.7 {
                self.stuck_t = 0.0;
                self.repath_t = 0.0;
                self.strafe_dir = -self.strafe_dir;
            }
        } else {
            self.stuck_t = 0.0;
        }
        false
    }

    fn perceive(&mut self, c: &mut Ctx, dt: f32) {
        let p = c.p;
        let eye = self.eye();
        let to = p.eye - eye;
        let dist = to.len();
        let gaze = fwd_from(self.yaw + self.look_off);
        let cosang = if dist > 0.1 { gaze.dot(to.xz().norm()) } else { 1.0 };
        // how squarely they're looking at you: straight on, they spot you sooner and much further away
        let focus = smoothstep(-0.2, 0.97, cosang);
        let state_k = match self.state {
            State::Combat => 1.5,
            State::Search => 1.3,
            State::Investigate => 1.2,
            _ => 1.0,
        } * (1.0 + 0.2 * self.alert);
        let range = self.kind.sight() * p.vis * state_k * lerpf(0.35, 1.15, focus);
        let cone = match self.state {
            State::Combat => -1.0,
            State::Search => -0.5,
            State::Investigate => -0.3,
            _ => -0.15,
        };
        let mut visible = false;
        if p.alive && dist < range && (cosang > cone || dist < 7.0) {
            visible = c.world.los(eye, p.eye) || (dist < range * 0.7 && c.world.los(eye, p.pos + v3(0.0, 0.9, 0.0)));
        }
        self.sees = visible;
        if visible {
            let near = clamp(1.0 - dist / range, 0.0, 1.0);
            let mut rate = (0.35 + near * 2.4) * lerpf(0.25, 1.7, focus * focus);
            if dist < 12.0 {
                rate += 2.0;
            }
            if p.speed > 0.6 {
                rate *= 1.3; // movement catches the eye
            }
            if matches!(self.state, State::Search | State::Investigate) {
                rate *= 1.4;
            }
            self.aware = (self.aware + rate * dt).min(1.3);
            self.last_seen = p.pos;
            self.lost_t = 0.0;
        } else {
            self.aware = (self.aware - 0.12 * dt * (1.0 - 0.5 * self.alert)).max(0.0);
            if self.state == State::Combat {
                self.lost_t += dt;
            }
        }
        let threshold = if self.state == State::Search { 0.8 } else { 1.0 };
        if visible && self.aware >= threshold && self.state != State::Combat {
            self.state = State::Combat;
            self.tactic = if self.holds_post() { Tactic::Cover } else { Tactic::Engage };
            self.tactic_t = c.rng.range(0.8, 2.0);
            self.cover_t = 0.5;
            self.shoot_cd = c.rng.range(0.35, 0.8);
            self.shots = 0;
            self.alert = 1.0;
            c.ev.push(Ev::Shout { pos: self.pos, target: p.pos });
        } else if visible && self.aware > 0.3 && self.state == State::Patrol {
            self.state = State::Investigate;
            self.target = p.pos;
            self.inv_t = 7.0;
        }
        // stumbling on a dead colleague raises the alarm
        if self.state != State::Combat {
            for &(id, pos) in c.corpses {
                if self.seen_bodies.contains(&id) {
                    continue;
                }
                if pos.dist(self.pos) < 22.0 && gaze.dot((pos - self.pos).xz().norm()) > -0.2 && c.world.los(eye, pos + v3(0.0, 0.4, 0.0)) {
                    self.seen_bodies.push(id);
                    self.alert = 1.0;
                    if self.state != State::Search {
                        self.state = State::Investigate;
                        self.target = pos;
                        self.inv_t = 10.0;
                    }
                    self.aware = self.aware.max(0.6);
                    c.ev.push(Ev::Body { pos });
                }
            }
        }
        if self.state == State::Combat && self.lost_t > 8.0 {
            self.state = State::Search;
            self.target = self.last_seen;
            self.search_t = 16.0;
            self.search_phase = 1;
            self.search_hops = 0;
            self.cover = None;
            self.aware = 0.5;
        }
    }

    pub fn update(&mut self, dt: f32, c: &mut Ctx) {
        self.flash = (self.flash - dt * 5.0).max(0.0);
        self.flinch = (self.flinch - dt * 3.0).max(0.0);
        self.muzzle = (self.muzzle - dt).max(0.0);
        self.hp_show = (self.hp_show - dt).max(0.0);
        if self.state == State::Dead {
            self.look_off = 0.0;
            self.dead_t += dt;
            self.moving = 0.0;
            if let Some(r) = self.ragdoll.as_mut() {
                r.update(dt, c.world);
                let pl = r.pelvis();
                self.pos = v3(pl.x, pl.y - 0.1, pl.z);
                self.vis_y = self.pos.y;
            }
            return;
        }
        self.alert = (self.alert - dt / 90.0).max(0.0);
        self.unreachable = (self.unreachable - dt).max(0.0);
        self.shoot_cd -= dt;
        self.moving = (self.moving - dt * 4.0).max(0.0);
        self.want_crouch = false;
        self.perc_t -= dt;
        if self.perc_t <= 0.0 {
            let pdt = 0.14;
            self.perc_t = pdt;
            self.perceive(c, pdt);
        }
        match self.state {
            State::Patrol => self.do_patrol(dt, c),
            State::Investigate => self.do_investigate(dt, c),
            State::Search => self.do_search(dt, c),
            State::Combat => self.do_combat(dt, c),
            State::Dead => {}
        }
        let tc = if self.want_crouch { 1.0 } else { 0.0 };
        self.crouch += (tc - self.crouch) * clamp(7.0 * dt, 0.0, 1.0);
        // feet glide over steps instead of popping up and down them
        if (self.pos.y - self.vis_y).abs() > 1.2 {
            self.vis_y = self.pos.y;
        } else {
            self.vis_y += (self.pos.y - self.vis_y) * clamp(14.0 * dt, 0.0, 1.0);
        }
    }

    fn scan(&mut self, dt: f32, speed: f32, amp: f32, rate: f32) {
        self.look_t += dt;
        let want = (self.look_t * speed + self.seed).sin() * amp;
        self.look_off += (want - self.look_off) * (1.0 - (-rate * dt).exp());
    }

    fn do_patrol(&mut self, dt: f32, c: &mut Ctx) {
        // sweep the gaze left and right so they actually look around
        self.scan(dt, 0.9, 0.8, 4.0);
        if self.patrol.is_empty() {
            if self.pos.dist_xz(self.home) > 0.8 || (self.pos.y - self.home.y).abs() > 1.0 {
                let (home, w) = (self.home, self.kind.walk());
                self.go(home, w, dt, c, true);
                return;
            }
            // posted: sweep an arc
            let target = self.home_yaw + (self.look_t * 0.35 + self.seed).sin() * 1.0;
            self.yaw = turn_toward(self.yaw, target, 1.2 * dt);
            return;
        }
        if self.wait > 0.0 {
            self.wait -= dt;
            return;
        }
        let goal = self.patrol[self.pi];
        let w = self.kind.walk();
        if self.go(goal, w, dt, c, true) {
            self.pi = (self.pi + 1) % self.patrol.len();
            self.wait = c.rng.range(0.8, 2.5);
        }
    }

    fn back_to_patrol(&mut self) {
        self.state = State::Patrol;
        self.aware = 0.0;
        self.wait = 1.0;
        self.cover = None;
        self.path.clear();
        if !self.patrol.is_empty() {
            let mut best = 0;
            let mut bd = f32::MAX;
            for (i, p) in self.patrol.iter().enumerate() {
                let dd = p.dist_xz(self.pos);
                if dd < bd {
                    bd = dd;
                    best = i;
                }
            }
            self.pi = best;
        }
    }

    fn do_investigate(&mut self, dt: f32, c: &mut Ctx) {
        self.inv_t -= dt;
        self.scan(dt, 1.6, 1.0, 5.0);
        let d = self.pos.dist_xz(self.target);
        if self.holds_post() || d < 1.8 {
            let base = yaw_to(self.target - self.pos);
            self.look_t += dt;
            let sweep = if d < 1.8 { (self.look_t * 1.3).sin() * 1.4 } else { (self.look_t * 0.7).sin() * 0.35 };
            self.yaw = turn_toward(self.yaw, base + sweep, 2.5 * dt);
        } else {
            let (t, sp) = (self.target, self.kind.run() * 0.6);
            self.go(t, sp, dt, c, true);
        }
        if self.inv_t <= 0.0 {
            self.back_to_patrol();
        }
    }

    fn do_search(&mut self, dt: f32, c: &mut Ctx) {
        self.search_t -= dt;
        let threat = self.target;
        let base_yaw = yaw_to(threat - self.pos);
        if self.startle_t > 0.0 {
            // the flinch: drop, spin toward the crack
            self.startle_t -= dt;
            self.want_crouch = true;
            self.yaw = turn_toward(self.yaw, base_yaw, 7.0 * dt);
            self.look_off *= 1.0 - (6.0 * dt).min(1.0);
            return;
        }
        match self.search_phase {
            0 => {
                // hide and watch where it came from
                self.scan(dt, 1.7, 0.6, 6.0);
                self.yaw = turn_toward(self.yaw, base_yaw, 3.0 * dt);
                if !self.holds_post() && self.cover.is_none() && *c.budget > 0 && self.tactic_t > 0.0 {
                    *c.budget -= 1;
                    self.cover = self.find_cover(threat, c).or(Some(self.pos));
                }
                if let Some(cp) = self.cover {
                    let run = self.kind.run();
                    if self.go(cp, run, dt, c, false) {
                        self.want_crouch = true;
                    }
                } else {
                    // posted marksmen bob up and down behind their parapet
                    self.want_crouch = (self.look_t * 0.8).sin() > -0.3;
                }
                self.tactic_t -= dt;
                if self.tactic_t <= 0.0 && !self.holds_post() {
                    self.search_phase = 1;
                    self.cover = None;
                }
            }
            1 => {
                // move up carefully on the spot
                self.scan(dt, 1.2, 0.7, 5.0);
                let w = self.kind.walk() * 1.15;
                if self.holds_post() || self.go(threat, w, dt, c, true) {
                    self.search_phase = 2;
                    self.tactic_t = c.rng.range(2.5, 4.5);
                }
            }
            _ => {
                // look all round, then check somewhere close by
                self.look_t += dt;
                let sweep = (self.look_t * 1.1 + self.seed).sin() * 1.6;
                self.yaw = turn_toward(self.yaw, base_yaw + sweep, 2.2 * dt);
                self.scan(dt, 2.0, 0.5, 5.0);
                self.tactic_t -= dt;
                if self.tactic_t <= 0.0 && !self.holds_post() {
                    self.search_hops += 1;
                    if self.search_hops > 2 {
                        self.search_t = 0.0;
                    } else {
                        let r = 7.0;
                        let a = c.rng.range(0.0, 6.28);
                        let p = threat + v3(a.cos() * r, 0.0, a.sin() * r);
                        let g = c.world.ground_at(p.x, p.z, 0.3, threat.y + 1.0, 0.6);
                        self.target = v3(p.x, g, p.z);
                        self.search_phase = 1;
                    }
                }
            }
        }
        if self.search_t <= 0.0 {
            self.back_to_patrol();
        }
    }

    /// Debug: how a cover search against `threat` goes — (reachable spots, with an obstacle in front, result).
    pub fn debug_cover(&self, threat: V3, c: &mut Ctx) -> (usize, usize, Option<V3>) {
        let s = self.kind.scale();
        let all = c.nav.reachable(self.pos, 13.0, 420, &c.world.doors);
        let mut front = 0;
        for (p, _) in &all {
            let to = threat - *p;
            let td = to.xz().len();
            if td < 4.0 {
                continue;
            }
            if c.world.raycast(*p + v3(0.0, 0.75 * s, 0.0), to.xz() / td, 1.4).is_some() {
                front += 1;
            }
        }
        (all.len(), front, self.find_cover(threat, c))
    }

    /// Somewhere near that blocks bullets from `threat` when crouched, ideally clear to shoot from when standing.
    pub fn find_cover(&self, threat: V3, c: &mut Ctx) -> Option<V3> {
        let s = self.kind.scale();
        let teye = threat + v3(0.0, 1.6, 0.0);
        let (lo, hi) = self.kind.pref();
        let mut scored: Vec<(f32, V3)> = vec![];
        for (p, d) in c.nav.reachable(self.pos, 13.0, 420, &c.world.doors) {
            let to = threat - p;
            let td = to.xz().len();
            if td < 4.0 {
                continue;
            }
            if c.claims.iter().any(|q| q.dist_xz(p) < 1.3 && (q.y - p.y).abs() < 1.0) {
                continue;
            }
            let dir = to.xz() / td;
            if c.world.raycast(p + v3(0.0, 0.75 * s, 0.0), dir, 1.4).is_none() {
                continue;
            }
            let band = if td < lo { (lo - td) * 0.8 } else if td > hi * 1.3 { (td - hi * 1.3) * 0.3 } else { 0.0 };
            scored.push((d * 0.5 + band + c.rng.range(0.0, 0.6), p));
        }
        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut fallback = None;
        for &(_, p) in scored.iter().take(10) {
            // the whole crouched head and shoulders must be behind it, not just the middle
            let side = right_from(yaw_to(threat - p)) * 0.3;
            let hidden = [V3::ZERO, side, -side].iter().all(|off| {
                let q = p + *off + v3(0.0, 1.28 * s, 0.0);
                let dc = q - teye;
                c.world.raycast(teye, dc.norm(), dc.len() - 0.2).is_some()
            });
            if !hidden {
                continue;
            }
            let standing = p + v3(0.0, 1.62 * s, 0.0);
            let ds = standing - teye;
            if c.world.raycast(teye, ds.norm(), ds.len() - 0.2).is_none() {
                return Some(p);
            }
            if fallback.is_none() {
                fallback = Some(p);
            }
        }
        fallback
    }

    fn do_combat(&mut self, dt: f32, c: &mut Ctx) {
        let p = c.p;
        let eye = self.eye();
        let to = p.eye - eye;
        let dist = to.len();
        let want_yaw = if self.sees { yaw_to(to) } else { yaw_to(self.last_seen - self.pos) };
        self.yaw = turn_toward(self.yaw, want_yaw, 6.0 * dt);
        if self.sees {
            self.aim_dir = to.norm();
        }
        self.look_t += dt;
        let want_look = if self.sees { 0.0 } else { (self.look_t * 2.0).sin() * 0.6 };
        self.look_off += (want_look - self.look_off) * (1.0 - (-6.0 * dt).exp());
        self.tactic_t -= dt;
        let threat = if self.sees { p.pos } else { self.last_seen };
        let (minr, maxr) = self.kind.pref();
        let run = self.kind.run();

        if self.kind == Kind::Heavy {
            // the warden doesn't hide: he comes for you
            if dist > minr || !self.sees {
                self.go(threat, run, dt, c, false);
            }
        } else {
            match self.tactic {
                Tactic::Engage => {
                    if self.holds_post() {
                        self.tactic = Tactic::Cover;
                        self.cover_t = 0.6;
                    } else if self.sees && (dist > self.kind.range() * 0.95 || self.unreachable > 0.0) {
                        // can't hit you from here and can't get to you: stay low and watch until cover turns up
                        self.want_crouch = true;
                    } else if self.sees {
                        if dist > maxr {
                            self.go(p.pos, run, dt, c, false);
                        } else if dist < minr {
                            let away = self.pos - to.xz().norm() * 3.0;
                            let w = self.kind.walk();
                            self.walk_to(away, w, dt, c.world, false);
                        } else {
                            let g = self.pos + right_from(self.yaw) * self.strafe_dir * 2.0;
                            let w = self.kind.walk() * 0.7;
                            self.walk_to(g, w, dt, c.world, false);
                            if c.rng.chance(dt * 0.4) {
                                self.strafe_dir = -self.strafe_dir;
                            }
                        }
                    } else {
                        let ls = self.last_seen;
                        self.go(ls, run * 0.8, dt, c, false);
                    }
                    if self.tactic_t <= 0.0 && !self.holds_post() {
                        self.tactic_t = c.rng.range(1.5, 3.0);
                        if *c.budget > 0 {
                            *c.budget -= 1;
                            if let Some(cp) = self.find_cover(threat, c) {
                                self.cover = Some(cp);
                                self.tactic = Tactic::ToCover;
                                self.tactic_t = 7.0;
                            }
                        }
                    }
                }
                Tactic::ToCover => {
                    let cp = self.cover.unwrap_or(self.pos);
                    if self.go(cp, run, dt, c, false) {
                        self.tactic = Tactic::Cover;
                        self.cover_t = c.rng.range(0.8, 1.8);
                    } else if self.tactic_t <= 0.0 {
                        self.tactic = Tactic::Engage;
                        self.cover = None;
                    }
                }
                Tactic::Cover => {
                    self.want_crouch = true;
                    if let Some(cp) = self.cover {
                        if self.pos.dist_xz(cp) > 0.6 {
                            self.go(cp, run, dt, c, false);
                        }
                    }
                    self.cover_t -= dt;
                    if self.sees && self.crouch > 0.8 {
                        self.exposed_t += dt;
                    } else {
                        self.exposed_t = 0.0;
                    }
                    // flanked: they can see us even crouched
                    if self.exposed_t > 0.5 {
                        self.exposed_t = 0.0;
                        if self.holds_post() {
                            self.tactic = Tactic::Peek;
                            self.peek_t = c.rng.range(1.2, 2.2);
                            self.peek_shots = 0;
                        } else {
                            self.tactic = Tactic::Engage;
                            self.tactic_t = 0.0;
                            self.cover = None;
                        }
                    } else if self.cover_t <= 0.0 {
                        self.tactic = Tactic::Peek;
                        self.peek_t = c.rng.range(1.8, 3.0);
                        self.peek_shots = 0;
                    }
                }
                Tactic::Peek => {
                    self.peek_t -= dt;
                    if !self.sees && !self.holds_post() && self.peek_t < 1.0 {
                        // nothing over the top: lean round the side instead
                        if let Some(cp) = self.cover {
                            let side = cp + right_from(yaw_to(threat - cp)) * (self.strafe_dir * 1.1);
                            let w = self.kind.walk();
                            self.walk_to(side, w, dt, c.world, false);
                        }
                    }
                    if self.peek_t <= 0.0 || self.peek_shots >= 4 {
                        self.tactic = Tactic::Cover;
                        self.cover_t = c.rng.range(0.8, 1.8);
                        if c.rng.chance(0.3) {
                            self.strafe_dir = -self.strafe_dir;
                        }
                    }
                    if self.lost_t > 5.0 && !self.holds_post() {
                        self.tactic = Tactic::Engage;
                        self.tactic_t = 2.0;
                        self.cover = None;
                    }
                }
            }
        }

        // shooting
        let aimed = angle_diff(self.yaw, yaw_to(to)).abs() < 0.3;
        if self.sees && self.shoot_cd <= 0.0 && dist < self.kind.range() && aimed && self.crouch < 0.45 {
            self.fire(c, dist);
            if self.tactic == Tactic::Peek {
                self.peek_shots += 1;
            }
        }
    }

    fn fire(&mut self, c: &mut Ctx, dist: f32) {
        let p = c.p;
        let s = self.kind.scale();
        let fwd = fwd_from(self.yaw);
        let muzzle = self.base() + v3(0.0, 1.25 * s, 0.0) + fwd * 0.75 * s + right_from(self.yaw) * 0.06;
        let (base, fall) = self.kind.acc();
        let mut ac = clamp(base - dist * fall, 0.07, 0.88);
        if self.shots < 2 {
            ac *= 0.5;
        }
        if p.crouch {
            ac *= 0.8;
        }
        ac *= 1.0 - 0.35 * clamp(p.speed / 6.0, 0.0, 1.0);
        if self.moving > 0.5 {
            ac *= 0.6;
        }
        let (hit, dmg) = if self.kind == Kind::Heavy {
            let mut n = 0;
            for _ in 0..7 {
                if c.rng.chance(ac) {
                    n += 1;
                }
            }
            (n > 0, n as f32 * self.kind.dmg())
        } else {
            let h = c.rng.chance(ac);
            (h, if h { self.kind.dmg() } else { 0.0 })
        };
        let to = if hit {
            p.eye
        } else {
            let ang = c.rng.range(0.0, 6.28);
            let r = c.rng.range(0.7, 2.6) + dist * 0.01;
            p.eye + v3(ang.cos() * r, c.rng.range(-0.8, 0.5), ang.sin() * r)
        };
        c.ev.push(Ev::Shot { from: muzzle, to, hit, dmg, kind: self.kind });
        self.muzzle = 0.07;
        self.shots += 1;
        match self.kind {
            Kind::Guard => self.shoot_cd = c.rng.range(0.75, 1.3),
            Kind::Rifleman => {
                self.burst -= 1;
                if self.burst > 0 {
                    self.shoot_cd = 0.15;
                } else {
                    self.burst = 3;
                    self.shoot_cd = c.rng.range(1.7, 2.6);
                }
            }
            Kind::Heavy => self.shoot_cd = 1.7,
        }
    }

    pub fn draw(&self, m: &mut Mesh, add: &mut Mesh, alpha: &mut Mesh, cam: V3) {
        // pure black stick-men — heads included. Kind shows in size and weapon, not colour.
        let black: Col = [0.018, 0.018, 0.022, 0.0];
        let look = Look {
            body: black,
            head: black,
            s: self.kind.scale(),
            flash: self.flash,
            vest: if self.kind == Kind::Heavy { Some([0.06, 0.06, 0.07, 0.0]) } else { None },
            gun: self.kind.gun(),
            muzzle: self.muzzle > 0.0 && self.alive(),
        };
        if let Some(r) = &self.ragdoll {
            draw_pose(m, add, cam, &r.pose(), &look);
            return;
        }
        draw_pose(m, add, cam, &self.make_pose(), &look);
        alpha.blob(self.base() + v3(0.0, 0.05, 0.0), 0.85 * self.kind.scale(), 0.30);
    }
}
