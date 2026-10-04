mod audio;
mod body;
mod builder;
mod country;
mod enemy;
mod gfx;
mod hud;
mod level;
mod math;
mod nav;
mod render;
mod village;
mod weapons;
mod world;

use audio::*;
use enemy::*;
use gfx::*;
use level::*;
use math::*;
use nav::Nav;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use weapons::*;
use world::*;

pub const MENU: u8 = 0;
pub const PLAYING: u8 = 1;
pub const PAUSED: u8 = 2;
pub const DEAD: u8 = 3;
pub const WON: u8 = 4;

const DT: f32 = 1.0 / 60.0;
const PR: f32 = 0.35; // player radius
const BASE_FOV: f32 = 72.0;
const SNIPER_ZOOMS: [f32; 2] = [12.0, 4.5];

#[derive(Default)]
pub(crate) struct Input {
    fwd: bool,
    back: bool,
    left: bool,
    right: bool,
    jump: bool,
    sprint: bool,
    crouch: bool,        // held (Ctrl)
    crouch_toggle: bool, // edge: C pressed
    fire: bool,
    aim_toggle: bool,    // edge: RMB / Q pressed
    zoom_cycle: bool,    // edge: X pressed
    reload: bool,
    interact: bool,
    select: Option<usize>,
    wheel: f32,
    mdx: f32,
    mdy: f32,
}

pub(crate) struct Player {
    pos: V3,
    vel: V3,
    yaw: f32,
    pitch: f32,
    hp: f32,
    grounded: bool,
    crouch: f32,
    cur: usize,
    ws: [WState; 4],
    cd: f32,
    reload_t: f32,
    switch_t: f32,
    ads: bool,
    ads_t: f32,
    zoom: usize,
    breath: f32,
    breath_lock: bool,
    sway: (f32, f32),
    sway_t: f32,
    kick: f32,
    kick_yaw: f32,
    bob: f32,
    step_acc: f32,
    shake: f32,
    dmg_flash: f32,
    muzzle: f32,
    recoil_anim: f32,
    speed: f32,
    fire_buf: f32,
    heal_flash: f32,
    aim_on: bool,    // aim/scope is toggled on (stays on until toggled off)
    crouch_on: bool, // crouch is toggled on
    sprinting: bool,
    vis: f32,        // how easy you are to spot (1.0 = walking in the open)
    reveal: f32,     // seconds left of being lit up by a loud shot
    eye_off: f32,    // camera lag that soaks up little step-ups so the view glides
}

impl Player {
    fn new(pos: V3) -> Player {
        Player {
            pos,
            vel: V3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            hp: 100.0,
            grounded: true,
            crouch: 0.0,
            cur: 0,
            ws: initial_ammo(),
            cd: 0.5,
            reload_t: 0.0,
            switch_t: 0.5,
            ads: false,
            ads_t: 0.0,
            zoom: 0,
            breath: 1.0,
            breath_lock: false,
            sway: (0.0, 0.0),
            sway_t: 0.0,
            kick: 0.0,
            kick_yaw: 0.0,
            bob: 0.0,
            step_acc: 0.0,
            shake: 0.0,
            dmg_flash: 0.0,
            muzzle: 0.0,
            recoil_anim: 0.0,
            speed: 0.0,
            fire_buf: 0.0,
            heal_flash: 0.0,
            aim_on: false,
            crouch_on: false,
            sprinting: false,
            vis: 1.0,
            reveal: 0.0,
            eye_off: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PKind {
    Health,
    Ammo(usize),
}

pub(crate) struct Pickup {
    kind: PKind,
    pos: V3,
    t: f32,
    taken: bool,
}

pub(crate) struct Particle {
    p: V3,
    v: V3,
    life: f32,
    max: f32,
    size: f32,
    col: [f32; 3],
    grav: f32,
    add: bool,
}

pub(crate) struct Tracer {
    a: V3,
    b: V3,
    t: f32,
    max: f32,
    col: [f32; 3],
    w: f32,
}

pub(crate) struct Decal {
    p: V3,
    n: V3,
    t: f32,
}

pub(crate) struct Prisoner {
    pos: V3,
    yaw: f32,
    mode: u8, // 0 caged, 1 escaping, 2 gone
    wp: usize,
    phase: f32,
}

#[derive(Default)]
pub(crate) struct Stats {
    kills: u32,
    headshots: u32,
    shots: u32,
    hits: u32,
    high_kills: u32,
    longest: f32,
    damage_taken: f32,
    time: f32,
}

pub(crate) struct Banner {
    title: String,
    sub: String,
    t: f32,
}

#[wasm_bindgen]
pub struct Game {
    r: Renderer,
    hudc: CanvasRenderingContext2d,
    audio: Audio,
    world: World,
    nav: Nav,
    doors0: Vec<Door>,
    spawn_defs: Vec<Spawn>,
    van: V3,
    zones: Vec<Zone>,
    zone_clear: Vec<bool>,
    lookouts: Vec<Lookout>,
    lookout_seen: Vec<bool>,
    latches: Vec<Latch>,
    latches0: Vec<Latch>,
    ammo_spots: Vec<(V3, usize)>,
    cell_pos: V3,
    cell_door_pos: V3,
    spawn_pos: V3,
    spawn_yaw: f32,
    door_cell: usize,
    door_rear: usize,
    door_wall: usize,
    indoor_min: V3,
    indoor_max: V3,
    escape: Vec<V3>,
    reinforcements: Vec<(Kind, V3)>,
    nav_ms: f32,

    light_vp: M4,
    b_static: Buf,
    b_fence: Buf,
    b_fence_dyn: Buf,
    b_scope: Buf,
    b_sky: Buf,
    b_dyn: Buf,
    b_alpha: Buf,
    b_add: Buf,
    b_view: Buf,
    b_view_add: Buf,
    m_dyn: Mesh,
    m_fence: Mesh,
    m_alpha: Mesh,
    m_add: Mesh,
    m_view: Mesh,
    m_view_add: Mesh,

    player: Player,
    enemies: Vec<Enemy>,
    pickups: Vec<Pickup>,
    parts: Vec<Particle>,
    tracers: Vec<Tracer>,
    decals: Vec<Decal>,
    prisoner: Prisoner,
    input: Input,
    rng: Rng,

    state: u8,
    last_t: f64,
    anim_t: f32,
    stage: usize,
    banner: Banner,
    stats: Stats,
    hit_marker: (f32, u8),
    indicators: Vec<(V3, f32)>,
    charges_t: f32,
    rear_blown: bool,
    wall_blown: bool,
    prompt: String,
    vp: M4,
    cam_pos: V3,
    cam_right: V3,
    cam_up: V3,
    tag: (String, f32),
    death_t: f32,
    width: f32,
    height: f32,
    empty_click: f32,
    step_phase: f32,
    stealth: u8, // 0 hidden, 1 suspicious, 2 searching, 3 detected
    eye_open: f32,
    eye_aware: f32,
    eye_search: f32,
    eye_pulse: f32,
    alert_cd: f32,
    aim_cd: f32,
    foot_cd: f32,
    view_rr: usize,
}

// ------------------------------------------------------------ JS-facing ----

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement, hud: HtmlCanvasElement) -> Result<Game, JsValue> {
        std::panic::set_hook(Box::new(|info| {
            web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
        }));
        let r = Renderer::new(&canvas)?;
        let lvl = build_level();
        let t0 = js_sys::Date::now();
        let mut lvl = lvl;
        let saved = lvl.world.doors.clone();
        for d in lvl.world.doors.iter_mut() {
            d.open = 1.0;
        }
        let nav = Nav::build(&lvl.world);
        lvl.world.doors = saved;
        let nav_ms = (js_sys::Date::now() - t0) as f32;
        let mut b_static = r.make_buf();
        r.upload(&mut b_static, &lvl.mesh.v, false);
        // bake the sun's shadows for the static world once; it never moves
        let light_vp = render::light_matrix();
        r.render_shadow(&light_vp, &[&b_static]);
        let mut b_fence = r.make_buf();
        r.upload(&mut b_fence, &lvl.fence_mesh.v, false);
        let mut b_scope = r.make_buf();
        let mut quad = Mesh::default();
        quad.quad_n(
            v3(-1.0, -1.0, 0.0),
            v3(1.0, -1.0, 0.0),
            v3(1.0, 1.0, 0.0),
            v3(-1.0, 1.0, 0.0),
            v3(0.0, 0.0, 1.0),
            [0.0, 0.0, 0.0, 1.0],
        );
        r.upload(&mut b_scope, &quad.v, false);
        let mut b_sky = r.make_buf();
        let sky = render::build_sky();
        r.upload(&mut b_sky, &sky.v, false);
        let hudc: CanvasRenderingContext2d = hud
            .get_context("2d")?
            .ok_or("no 2d context")?
            .dyn_into()?;
        let spawn_pos = lvl.spawn_pos;
        let mut g = Game {
            b_dyn: r.make_buf(),
            b_fence_dyn: r.make_buf(),
            b_alpha: r.make_buf(),
            b_add: r.make_buf(),
            b_view: r.make_buf(),
            b_view_add: r.make_buf(),
            r,
            hudc,
            audio: Audio::new(),
            doors0: lvl.world.doors.clone(),
            world: lvl.world,
            nav,
            nav_ms,
            spawn_defs: lvl.spawns,
            van: lvl.van,
            zone_clear: vec![false; lvl.zones.len()],
            zones: lvl.zones,
            lookout_seen: vec![false; lvl.lookouts.len()],
            lookouts: lvl.lookouts,
            latches0: lvl.latches.clone(),
            latches: lvl.latches,
            ammo_spots: lvl.ammo_spots,
            cell_pos: lvl.cell_pos,
            cell_door_pos: lvl.cell_door_pos,
            spawn_pos,
            spawn_yaw: lvl.spawn_yaw,
            door_cell: lvl.cell_door,
            door_rear: lvl.blast_rear,
            door_wall: lvl.blast_wall,
            indoor_min: lvl.indoor_min,
            indoor_max: lvl.indoor_max,
            escape: lvl.escape,
            reinforcements: lvl.reinforcements,
            light_vp,
            b_static,
            b_fence,
            b_scope,
            b_sky,
            m_dyn: Mesh::default(),
            m_fence: Mesh::default(),
            m_alpha: Mesh::default(),
            m_add: Mesh::default(),
            m_view: Mesh::default(),
            m_view_add: Mesh::default(),
            player: Player::new(spawn_pos),
            enemies: vec![],
            pickups: vec![],
            parts: vec![],
            tracers: vec![],
            decals: vec![],
            prisoner: Prisoner { pos: V3::ZERO, yaw: 0.0, mode: 0, wp: 0, phase: 0.0 },
            input: Input::default(),
            rng: Rng::new(99),
            state: MENU,
            last_t: 0.0,
            anim_t: 0.0,
            stage: 0,
            banner: Banner { title: String::new(), sub: String::new(), t: 0.0 },
            stats: Stats::default(),
            hit_marker: (0.0, 0),
            indicators: vec![],
            charges_t: 0.0,
            rear_blown: false,
            wall_blown: false,
            prompt: String::new(),
            vp: [0.0; 16],
            cam_pos: V3::ZERO,
            cam_right: v3(1.0, 0.0, 0.0),
            cam_up: V3::Y,
            tag: (String::new(), 0.0),
            death_t: 0.0,
            width: canvas.width() as f32,
            height: canvas.height() as f32,
            empty_click: 0.0,
            step_phase: 0.0,
            stealth: 0,
            eye_open: 0.1,
            eye_aware: 0.0,
            eye_search: 0.0,
            eye_pulse: 0.0,
            alert_cd: 0.0,
            aim_cd: 0.0,
            foot_cd: 0.0,
            view_rr: 0,
        };
        g.reset();
        Ok(g)
    }

    /// Begin (or restart) a mission. Call from a user gesture so audio can start.
    pub fn start(&mut self) {
        self.audio.init();
        self.reset();
        self.state = PLAYING;
    }

    pub fn resume(&mut self) {
        self.audio.init();
        if self.state == PAUSED {
            self.state = PLAYING;
        }
    }

    pub fn pause(&mut self) {
        if self.state == PLAYING {
            self.state = PAUSED;
            self.input = Input::default();
        }
    }

    pub fn state(&self) -> u8 {
        self.state
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.r.resize(w, h);
        self.width = w as f32;
        self.height = h as f32;
    }

    pub fn key(&mut self, code: &str, down: bool) {
        let i = &mut self.input;
        match code {
            "KeyW" | "ArrowUp" => i.fwd = down,
            "KeyS" | "ArrowDown" => i.back = down,
            "KeyA" | "ArrowLeft" => i.left = down,
            "KeyD" | "ArrowRight" => i.right = down,
            "Space" => i.jump = down,
            "ShiftLeft" | "ShiftRight" => i.sprint = down,
            "ControlLeft" | "ControlRight" => i.crouch = down,
            "KeyC" => {
                if down {
                    i.crouch_toggle = true
                }
            }
            "KeyQ" => {
                if down {
                    i.aim_toggle = true
                }
            }
            "KeyX" => {
                if down {
                    i.zoom_cycle = true
                }
            }
            "KeyR" => {
                if down {
                    i.reload = true
                }
            }
            "KeyE" | "KeyF" => {
                if down {
                    i.interact = true
                }
            }
            "Digit1" => {
                if down {
                    i.select = Some(0)
                }
            }
            "Digit2" => {
                if down {
                    i.select = Some(1)
                }
            }
            "Digit3" => {
                if down {
                    i.select = Some(2)
                }
            }
            "Digit4" => {
                if down {
                    i.select = Some(3)
                }
            }
            _ => {}
        }
    }

    pub fn mouse_move(&mut self, dx: f32, dy: f32) {
        self.input.mdx += dx;
        self.input.mdy += dy;
    }

    pub fn mouse_button(&mut self, b: i32, down: bool) {
        match b {
            0 => {
                self.input.fire = down;
                if down {
                    self.player.fire_buf = 0.2;
                }
            }
            2 => {
                // right-click TOGGLES aim so it works on trackpads and stays on while you play
                if down {
                    self.input.aim_toggle = true;
                }
            }
            _ => {}
        }
    }

    pub fn wheel(&mut self, dy: f32) {
        self.input.wheel += dy;
    }

    /// Current objective line (shown on the pause screen).
    pub fn objective_text(&self) -> String {
        self.objective()
    }

    pub fn summary(&self) -> String {
        let s = &self.stats;
        let acc = if s.shots > 0 { s.hits as f32 / s.shots as f32 * 100.0 } else { 0.0 };
        let secs = s.time as u32;
        format!(
            "TIME {:02}:{:02}   KILLS {}   HEADSHOTS {}   HIGH-GROUND KILLS {}   ACCURACY {:.0}%   LONGEST SHOT {:.0}m",
            secs / 60,
            secs % 60,
            s.kills,
            s.headshots,
            s.high_kills,
            acc,
            s.longest
        )
    }

    /// Debug/test hooks (used by automated playtests).
    pub fn dbg_teleport(&mut self, x: f32, y: f32, z: f32, yaw: f32, pitch: f32) {
        self.player.pos = v3(x, y, z);
        self.player.vel = V3::ZERO;
        self.player.yaw = yaw;
        self.player.pitch = pitch;
    }

    /// Jump to lookout i (see dbg_lookouts) facing the prison.
    pub fn dbg_goto_lookout(&mut self, i: usize) -> String {
        if let Some(l) = self.lookouts.get(i) {
            self.player.pos = l.pos + v3(0.0, 0.05, 0.0);
            self.player.vel = V3::ZERO;
            self.player.yaw = yaw_to(v3(0.0, 0.0, -10.0) - l.pos);
            self.player.pitch = -0.08;
            return l.name.to_string();
        }
        String::new()
    }

    pub fn dbg_lookouts(&self) -> String {
        self.lookouts.iter().enumerate().map(|(i, l)| format!("{} {} ({:.1},{:.1},{:.1})", i, l.name, l.pos.x, l.pos.y, l.pos.z)).collect::<Vec<_>>().join("\n")
    }

    /// Every patrol leg that the nav grid can't route (should be none).
    pub fn dbg_patrol_check(&self) -> String {
        let mut bad = vec![];
        for (i, sp) in self.spawn_defs.iter().enumerate() {
            let mut from = sp.pos;
            for (k, w) in sp.patrol.iter().enumerate() {
                if self.nav.path(from, *w, 40000, &self.doors0).is_none() {
                    bad.push(format!("spawn {} ({:.0},{:.0}) leg {} -> ({:.1},{:.1})", i, sp.pos.x, sp.pos.z, k, w.x, w.z));
                }
                from = *w;
            }
            if self.nav.nearest(sp.pos).is_none() {
                bad.push(format!("spawn {} off-grid ({:.1},{:.1},{:.1})", i, sp.pos.x, sp.pos.y, sp.pos.z));
            }
        }
        if bad.is_empty() { "all patrols routable".to_string() } else { bad.join("\n") }
    }

    pub fn dbg_cover(&mut self, i: usize) -> String {
        let pv = PlayerView { pos: self.player.pos, eye: self.eye(), crouch: false, vis: 1.0, speed: 0.0, alive: true };
        let mut evs = vec![];
        let mut budget = 99;
        let claims: Vec<V3> = vec![];
        let corpses: Vec<(usize, V3)> = vec![];
        let Some(e) = self.enemies.get(i) else { return String::new() };
        let mut ctx = Ctx { world: &self.world, nav: &self.nav, p: &pv, rng: &mut self.rng, ev: &mut evs, corpses: &corpses, budget: &mut budget, claims: &claims };
        let (n, f, r) = e.debug_cover(self.player.pos, &mut ctx);
        format!("reachable={} front={} cover={:?} nearest_node={:?}", n, f, r.map(|p| (p.x, p.y, p.z)), self.nav.nearest(e.pos))
    }

    /// Nav route between two feet positions as [x,y,z, x,y,z, ...] (empty if none).
    pub fn dbg_route(&self, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> Vec<f32> {
        match self.nav.path(v3(x0, y0, z0), v3(x1, y1, z1), 40000, &self.world.doors) {
            Some(p) => p.iter().flat_map(|q| [q.x, q.y, q.z]).collect(),
            None => vec![],
        }
    }

    pub fn dbg_look(&mut self, yaw: f32, pitch: f32) {
        self.player.yaw = yaw;
        self.player.pitch = pitch;
    }

    pub fn dbg_pos(&self) -> Vec<f32> {
        let p = &self.player;
        vec![p.pos.x, p.pos.y, p.pos.z, self.eye().y]
    }

    pub fn dbg_aim_point(&mut self, x: f32, y: f32, z: f32) {
        let d = v3(x, y, z) - self.eye();
        self.player.yaw = yaw_to(d);
        self.player.pitch = (d.y / d.xz().len()).atan();
    }

    /// Aim at latch i; returns its distance.
    pub fn dbg_aim_latch(&mut self, i: usize) -> f32 {
        let eye = self.eye();
        if let Some(l) = self.latches.get(i) {
            let d = l.pos - eye;
            self.player.yaw = yaw_to(d);
            self.player.pitch = (d.y / d.xz().len()).atan();
            return d.len();
        }
        -1.0
    }

    pub fn dbg_world(&self) -> String {
        format!("shapes={} nav_nodes={} nav_links={} nav_ms={:.0} enemies={}", self.world.shapes.len(), self.nav.nodes.len(), self.nav.link_count(), self.nav_ms, self.enemies.len())
    }

    pub fn dbg_path(&self, x0: f32, z0: f32, x1: f32, z1: f32) -> String {
        let a = v3(x0, self.world.ground_at(x0, z0, 0.3, 50.0, 0.0), z0);
        let b = v3(x1, self.world.ground_at(x1, z1, 0.3, 50.0, 0.0), z1);
        match self.nav.path(a, b, 20000, &self.world.doors) {
            Some(p) => format!("{} pts, len {:.1}", p.len(), p.windows(2).map(|w| w[0].dist(w[1])).sum::<f32>()),
            None => "no path".to_string(),
        }
    }

    /// yaw, pitch (rad), fov (deg), eye height of the camera used for the last frame
    pub fn dbg_view(&self) -> Vec<f32> {
        let (y, p) = self.view_angles();
        vec![y, p, self.cur_fov_deg(), self.cam_pos.y, self.cam_pos.x, self.cam_pos.z, self.scope_amount(), self.player.ads_t]
    }

    pub fn dbg_set_hp(&mut self, hp: f32) {
        self.player.hp = hp;
    }

    pub fn dbg_kill_enemy(&mut self, i: usize) {
        if let Some(e) = self.enemies.get_mut(i) {
            e.damage(1e6, V3::ZERO);
        }
    }

    pub fn dbg_kill_zone(&mut self, zone: usize) {
        for e in self.enemies.iter_mut() {
            if e.zone == zone && e.alive() {
                e.damage(1e6, V3::ZERO);
            }
        }
    }

    /// Aim at a point `height` metres above enemy i's feet (head ≈ 1.66, chest ≈ 1.1).
    pub fn dbg_aim_at(&mut self, i: usize, height: f32) {
        let eye = self.eye();
        if let Some(e) = self.enemies.get(i) {
            let d = (e.pos + v3(0.0, height * e.kind.scale(), 0.0)) - eye;
            self.player.yaw = yaw_to(d);
            self.player.pitch = (d.y / d.xz().len()).atan();
        }
    }

    /// Angular error (degrees) between where the camera points and a point `height` above enemy i's feet.
    pub fn dbg_aim_error(&self, i: usize, height: f32) -> f32 {
        let eye = self.eye();
        let (yaw, pitch) = self.view_angles();
        match self.enemies.get(i) {
            Some(e) => {
                let to = (e.pos + v3(0.0, height * e.kind.scale(), 0.0) - eye).norm();
                look_dir(yaw, pitch).dot(to).clamp(-1.0, 1.0).acos().to_degrees()
            }
            None => 999.0,
        }
    }

    pub fn dbg_enemy(&self, i: usize) -> String {
        match self.enemies.get(i) {
            Some(e) => format!(
                "{:?} {:?} {:?} hp={:.0}/{:.0} aware={:.2} sees={} crouch={:.2} zone={} pos=({:.1},{:.1},{:.1})",
                e.kind, e.state, e.tactic, e.hp, e.kind.hp(), e.aware, e.sees, e.crouch, e.zone, e.pos.x, e.pos.y, e.pos.z
            ),
            None => String::new(),
        }
    }

    pub fn dbg_aim_at_enemy(&mut self, i: usize) -> String {
        let eye = self.eye();
        if let Some(e) = self.enemies.get(i) {
            let d = e.head_center() - eye;
            self.player.yaw = yaw_to(d);
            self.player.pitch = (d.y / d.xz().len()).atan();
            return format!("{:?} {:?} at ({:.1},{:.1},{:.1}) dist {:.1}", e.kind, e.state, e.pos.x, e.pos.y, e.pos.z, d.len());
        }
        String::new()
    }

    pub fn dbg_info(&self) -> String {
        let p = &self.player;
        format!(
"state={} stage={} hp={:.0} pos=({:.1},{:.1},{:.1}) yaw={:.2} alive={} wpn={} mag={}/{} aim={} crouch={:.2} stealth={} vis={:.2} latches={}",
            self.state,
            self.stage,
            p.hp,
            p.pos.x,
            p.pos.y,
            p.pos.z,
            p.yaw,
            self.enemies.iter().filter(|e| e.alive()).count(),
            p.cur,
            p.ws[p.cur].mag,
            p.ws[p.cur].reserve,
            p.aim_on,
            p.crouch,
            self.stealth,
            p.vis,
            self.latches.iter().map(|l| if l.broken { '1' } else { '0' }).collect::<String>()
        )
    }

    pub fn frame(&mut self, t: f64) {
        let dt = if self.last_t == 0.0 {
            0.016
        } else {
            ((t - self.last_t) / 1000.0) as f32
        }
        .clamp(0.0, 0.1);
        self.last_t = t;
        self.anim_t += dt;
        if self.state == PLAYING {
            self.apply_look();
            // Advance the world by exactly the time that passed, in slices of at most 1/60 s.
            // (A fixed 60 Hz tick ran on only every other frame of a 120 Hz display, so anything
            // driven by the sim — scope sway, recoil, zoom — visibly stepped. Not any more.)
            let n = ((dt / DT).ceil() as usize).clamp(1, 6);
            let h = dt / n as f32;
            for _ in 0..n {
                self.update(h);
            }
        } else {
            self.input.mdx = 0.0;
            self.input.mdy = 0.0;
            if self.state == DEAD || self.state == WON {
                self.death_t += dt;
                self.update_fx(dt);
            }
        }
        self.render(dt);
    }
}

// --------------------------------------------------------------- logic -----

impl Game {
    fn reset(&mut self) {
        self.world.doors = self.doors0.clone();
        self.latches = self.latches0.clone();
        self.player = Player::new(self.spawn_pos);
        self.player.yaw = self.spawn_yaw;
        self.zone_clear.iter_mut().for_each(|z| *z = false);
        self.lookout_seen.iter_mut().for_each(|z| *z = false);
        let mut enemies = vec![];
        for s in &self.spawn_defs {
            enemies.push(Enemy::new(s.kind, s.pos, s.yaw, s.patrol.clone(), s.zone, &mut self.rng));
        }
        self.enemies = enemies;
        self.pickups.clear();
        for (i, (p, w)) in self.ammo_spots.iter().enumerate() {
            self.pickups.push(Pickup { kind: PKind::Ammo(*w), pos: *p, t: i as f32, taken: false });
        }
        self.parts.clear();
        self.tracers.clear();
        self.decals.clear();
        self.indicators.clear();
        self.prisoner = Prisoner { pos: self.cell_pos, yaw: std::f32::consts::PI, mode: 0, wp: 0, phase: 0.0 };
        self.stage = 0;
        self.stats = Stats::default();
        self.charges_t = 0.0;
        self.rear_blown = false;
        self.wall_blown = false;
        self.death_t = 0.0;
        self.hit_marker = (0.0, 0);
        self.input = Input::default();
        self.stealth = 0;
        self.eye_open = 0.1;
        self.eye_aware = 0.0;
        self.eye_search = 0.0;
        self.eye_pulse = 0.0;
        self.alert_cd = 0.0;
        self.say("INFILTRATE THE PRISON", "Shoot the latch on a prison gate · a lookout gives you a clear shot");
    }

    fn say(&mut self, title: &str, sub: &str) {
        self.banner = Banner { title: title.to_string(), sub: sub.to_string(), t: 4.5 };
    }

    fn tag(&mut self, s: &str) {
        self.tag = (s.to_string(), 1.8);
    }

    fn eye_height(&self) -> f32 {
        lerpf(1.65, 1.0, self.player.crouch)
    }

    pub(crate) fn eye(&self) -> V3 {
        self.player.pos + v3(0.0, self.eye_height() + self.player.eye_off, 0.0)
    }

    pub(crate) fn view_angles(&self) -> (f32, f32) {
        let p = &self.player;
        (p.yaw + p.sway.0 + p.kick_yaw, (p.pitch + p.kick + p.sway.1).clamp(-1.55, 1.55))
    }

    pub(crate) fn cur_fov_deg(&self) -> f32 {
        let p = &self.player;
        let target = if p.cur == 0 { SNIPER_ZOOMS[p.zoom] } else { WEAPONS[p.cur].ads_fov };
        let t = p.ads_t * p.ads_t * (3.0 - 2.0 * p.ads_t);
        lerpf(BASE_FOV, target, t)
    }

    pub(crate) fn scoped(&self) -> bool {
        self.player.cur == 0 && self.player.ads_t > 0.85
    }

    /// 0..1: how far we are into looking down the scope (fades the overlay in, the gun out).
    pub(crate) fn scope_amount(&self) -> f32 {
        if self.player.cur == 0 {
            smoothstep(0.45, 0.92, self.player.ads_t)
        } else {
            0.0
        }
    }

    fn apply_look(&mut self) {
        let sens = 0.0022 * (self.cur_fov_deg() / BASE_FOV).powf(0.8);
        let p = &mut self.player;
        p.yaw += self.input.mdx * sens;
        p.pitch = (p.pitch - self.input.mdy * sens).clamp(-1.5, 1.5);
        self.input.mdx = 0.0;
        self.input.mdy = 0.0;
    }

    fn spatial(&self, pos: V3) -> (f32, f32) {
        let eye = self.eye();
        let d = pos.dist(eye);
        let vol = (1.3 / (1.0 + (d / 14.0).powf(1.3))).clamp(0.0, 1.0);
        let (yaw, _) = self.view_angles();
        let to = (pos - eye).xz().norm();
        (vol, to.dot(right_from(yaw)) * 0.9)
    }

    fn update(&mut self, dt: f32) {
        self.stats.time += dt;
        self.update_player(dt);
        self.update_doors(dt);
        self.update_enemies(dt);
        self.update_stealth(dt);
        self.update_pickups(dt);
        self.update_fx(dt);
        self.update_mission(dt);
        if self.player.hp <= 0.0 && self.state == PLAYING {
            self.state = DEAD;
            self.death_t = 0.0;
            self.input = Input::default();
        }
    }

    // ----------------------------------------------------------- player ----

    fn select_weapon(&mut self, i: usize) {
        if i != self.player.cur && i < 4 {
            self.player.cur = i;
            self.player.switch_t = 0.45;
            self.player.reload_t = 0.0;
            self.player.zoom = 0;
            self.player.aim_on = false;
            self.audio.play(Snd::Switch, 1.0, 0.0);
        }
    }

    fn start_reload(&mut self) {
        let p = &mut self.player;
        let def = &WEAPONS[p.cur];
        let ws = p.ws[p.cur];
        if p.reload_t > 0.0 || ws.mag >= def.mag || ws.reserve <= 0 || p.switch_t > 0.0 {
            return;
        }
        p.reload_t = def.reload;
        p.ads = false;
        self.audio.play(Snd::Reload, 1.0, 0.0);
    }

    fn update_player(&mut self, dt: f32) {
        if let Some(i) = self.input.select.take() {
            self.select_weapon(i);
        }
        let wh = std::mem::take(&mut self.input.wheel);
        if wh != 0.0 {
            if self.player.cur == 0 && self.player.aim_on {
                self.player.zoom = if wh < 0.0 { 1 } else { 0 };
            } else {
                let n = if wh > 0.0 { 1 } else { 3 };
                let next = (self.player.cur + n) % 4;
                self.select_weapon(next);
            }
        }
        if std::mem::take(&mut self.input.zoom_cycle) && self.player.cur == 0 && self.player.aim_on {
            self.player.zoom = 1 - self.player.zoom;
            self.audio.play(Snd::Switch, 0.6, 0.0);
        }
        self.aim_cd = (self.aim_cd - dt).max(0.0);
        if std::mem::take(&mut self.input.aim_toggle) && self.aim_cd <= 0.0 {
            self.player.aim_on = !self.player.aim_on;
            self.aim_cd = 0.15;
            self.audio.play(Snd::Switch, 0.5, 0.0);
        }
        if std::mem::take(&mut self.input.crouch_toggle) {
            self.player.crouch_on = !self.player.crouch_on;
        }
        if std::mem::take(&mut self.input.reload) {
            self.start_reload();
        }

        {
            let p = &mut self.player;
            p.cd -= dt;
            p.switch_t = (p.switch_t - dt).max(0.0);
            p.fire_buf = (p.fire_buf - dt).max(0.0);
            p.muzzle = (p.muzzle - dt).max(0.0);
            p.recoil_anim = (p.recoil_anim - dt * 7.0).max(0.0);
            p.kick *= (1.0 - 9.0 * dt).max(0.0);
            p.kick_yaw *= (1.0 - 9.0 * dt).max(0.0);
            p.shake = (p.shake - dt * 2.5).max(0.0);
            p.dmg_flash = (p.dmg_flash - dt * 1.4).max(0.0);
            p.heal_flash = (p.heal_flash - dt * 1.5).max(0.0);
            self.empty_click = (self.empty_click - dt).max(0.0);
            if p.reload_t > 0.0 {
                p.reload_t -= dt;
                if p.reload_t <= 0.0 {
                    let def = &WEAPONS[p.cur];
                    let ws = &mut p.ws[p.cur];
                    let need = def.mag - ws.mag;
                    let take = need.min(ws.reserve);
                    ws.mag += take;
                    ws.reserve -= take;
                }
            }
        }

        // sprinting stands you up and drops your aim (behind the scope Shift holds breath instead)
        let sprinting = self.input.sprint
            && self.input.fwd
            && !self.input.crouch
            && !(self.player.cur == 0 && self.player.aim_on);
        if sprinting {
            self.player.crouch_on = false;
            self.player.aim_on = false;
        }
        // aiming down sights: toggled with RMB / Q, suspended while reloading or swapping guns
        {
            let p = &mut self.player;
            let want = p.aim_on && p.reload_t <= 0.0 && p.switch_t <= 0.0;
            p.ads = want;
            let rate = if p.cur == 0 { 6.0 } else { 10.0 };
            let tgt = if want { 1.0 } else { 0.0 };
            // move toward the target and STOP there (never overshoot, never back off once arrived)
            if p.ads_t < tgt {
                p.ads_t = (p.ads_t + rate * dt).min(tgt);
            } else if p.ads_t > tgt {
                p.ads_t = (p.ads_t - rate * 1.4 * dt).max(tgt);
            }
        }

        self.move_player(dt, sprinting);

        // how easy are we to spot right now?
        {
            let p = &mut self.player;
            p.sprinting = sprinting;
            p.reveal = (p.reveal - dt).max(0.0);
            let mut vis = 1.0;
            if p.crouch > 0.5 {
                vis *= 0.65;
            }
            if p.speed < 0.6 {
                vis *= 0.8;
            } else if sprinting {
                vis *= 1.3;
            }
            if p.reveal > 0.0 {
                vis *= 1.6;
            }
            p.vis = vis;
        }

        // sway / breath for the scope
        {
            let scoped = self.scoped();
            let p = &mut self.player;
            if scoped {
                let moving = clamp(p.speed / 3.0, 0.0, 1.0);
                let hold = self.input.sprint && p.breath > 0.0 && !p.breath_lock;
                if hold {
                    p.breath -= dt * 0.26;
                    if p.breath <= 0.0 {
                        p.breath = 0.0;
                        p.breath_lock = true;
                    }
                } else {
                    p.breath = (p.breath + dt * 0.2).min(1.0);
                    if p.breath > 0.45 {
                        p.breath_lock = false;
                    }
                }
                let crouchk = lerpf(1.0, 0.5, p.crouch);
                let zoomk = if p.zoom == 1 { 0.45 } else { 1.0 };
                let amp = 0.5 * crouchk * zoomk * (1.0 + moving * 2.5) * if hold { 0.07 } else { 1.0 };
                p.sway_t += dt;
                let t = p.sway_t;
                let a = amp.to_radians();
                p.sway = (
                    ((t * 1.3).sin() + 0.4 * (t * 2.9 + 1.0).sin()) * a,
                    ((t * 1.1 + 2.0).cos() * 0.8 + 0.3 * (t * 3.3).sin()) * a,
                );
            } else {
                p.sway.0 *= (1.0 - 8.0 * dt).max(0.0);
                p.sway.1 *= (1.0 - 8.0 * dt).max(0.0);
                p.breath = (p.breath + dt * 0.3).min(1.0);
            }
        }

        // firing
        let def = &WEAPONS[self.player.cur];
        let trigger = (def.auto && self.input.fire) || self.player.fire_buf > 0.0;
        if trigger && self.try_fire() {
            self.player.fire_buf = 0.0;
        }

        // interaction + prompts
        self.prompt.clear();
        let d = self.player.pos.dist_xz(self.cell_door_pos);
        if self.stage == 2 && d < 3.2 && (self.player.pos.y - self.cell_door_pos.y).abs() < 2.0 {
            self.prompt = "PRESS E — FREE INMATE 13".to_string();
            if self.input.interact {
                self.free_prisoner();
            }
        }
        self.input.interact = false;
    }

    fn move_player(&mut self, dt: f32, sprinting: bool) {
        let scoped = self.scoped();
        let inp = &self.input;
        let p = &mut self.player;
        let w = &self.world;
        if inp.jump {
            p.crouch_on = false;
        }
        let want_c = if p.crouch_on || inp.crouch { 1.0 } else { 0.0 };
        p.crouch += (want_c - p.crouch) * clamp(9.0 * dt, 0.0, 1.0);
        let h = lerpf(1.8, 1.1, p.crouch);

        let f = fwd_from(p.yaw);
        let r = right_from(p.yaw);
        let mut wish = f * ((inp.fwd as i32 - inp.back as i32) as f32)
            + r * ((inp.right as i32 - inp.left as i32) as f32);
        if wish.len2() > 0.0 {
            wish = wish.norm();
        }
        let mut speed = if sprinting {
            8.0
        } else if p.crouch > 0.5 {
            2.7
        } else {
            5.2
        };
        if scoped {
            speed = if p.crouch > 0.5 { 1.2 } else { 2.0 };
        } else if p.ads {
            speed *= 0.65;
        }
        speed *= WEAPONS[p.cur].move_mul;
        let tgt = wish * speed;
        let accel = if p.grounded { 14.0 } else { 2.2 };
        let k = clamp(accel * dt, 0.0, 1.0);
        p.vel.x += (tgt.x - p.vel.x) * k;
        p.vel.z += (tgt.z - p.vel.z) * k;

        // move, then slide out of whatever we walked into (walls at any angle, not just the axes)
        let start = p.pos;
        let try_move = |from: V3, to: V3| -> Option<V3> {
            let dist = (to - from).xz().len().max(1e-4);
            if terrain_h(to.x, to.z) - from.y > 1.15 * dist + 0.03 {
                return None; // too steep to climb
            }
            let mut q = to;
            w.resolve(&mut q, PR, h);
            if q.x * q.x + q.z * q.z > 158.0 * 158.0 {
                return None;
            }
            if terrain_h(q.x, q.z) - from.y > 1.15 * (q - from).xz().len().max(1e-4) + 0.03 {
                return None;
            }
            Some(q)
        };
        let full = p.pos + v3(p.vel.x * dt, 0.0, p.vel.z * dt);
        if let Some(q) = try_move(p.pos, full) {
            p.pos = q;
        } else if let Some(q) = try_move(p.pos, p.pos + v3(p.vel.x * dt, 0.0, 0.0)) {
            p.pos = q;
            p.vel.z = 0.0;
        } else if let Some(q) = try_move(p.pos, p.pos + v3(0.0, 0.0, p.vel.z * dt)) {
            p.pos = q;
            p.vel.x = 0.0;
        } else {
            p.vel.x = 0.0;
            p.vel.z = 0.0;
        }
        // keep only the speed we actually achieved (so we don't build up push into a wall)
        if dt > 0.0 {
            let got = (p.pos - start).xz() / dt;
            let cap = (p.vel.x * p.vel.x + p.vel.z * p.vel.z).sqrt() + 0.01;
            let gl = got.len();
            let got = if gl > cap { got * (cap / gl) } else { got };
            p.vel.x = got.x;
            p.vel.z = got.z;
        }

        // vertical
        let g = w.ground_at(p.pos.x, p.pos.z, PR, p.pos.y, STEP);
        if p.grounded {
            if inp.jump {
                p.vel.y = 7.4;
                p.grounded = false;
            } else if p.pos.y - g <= STEP + 0.01 {
                // a curb or the first step of a stair: let the camera glide instead of popping
                let dy = g - p.pos.y;
                if dy.abs() > 0.07 {
                    p.eye_off = (p.eye_off - dy).clamp(-0.7, 0.7);
                }
                p.pos.y = g;
                p.vel.y = 0.0;
            } else {
                p.grounded = false;
            }
        }
        if !p.grounded {
            p.vel.y -= GRAV * dt;
            let mut ny = p.pos.y + p.vel.y * dt;
            if p.vel.y > 0.0 {
                let c = w.ceiling_at(p.pos.x, p.pos.z, PR, p.pos.y + h);
                if ny + h > c {
                    ny = c - h;
                    p.vel.y = 0.0;
                }
            }
            if ny <= g && p.vel.y <= 0.0 {
                let impact = -p.vel.y;
                p.pos.y = g;
                p.vel.y = 0.0;
                p.grounded = true;
                if impact > 5.0 {
                    self.audio.play(Snd::Land, (impact / 10.0).min(1.0), 0.0);
                    p.shake = p.shake.max(0.15);
                }
                if impact > 13.5 {
                    let dmg = (impact - 13.5) * 9.0;
                    p.hp -= dmg;
                    p.dmg_flash = 1.0;
                    self.stats.damage_taken += dmg;
                    self.audio.play(Snd::Hurt, 1.0, 0.0);
                }
            } else {
                p.pos.y = ny;
            }
        }

        p.eye_off *= (-12.0 * dt).exp();
        let hs = (p.vel.x * p.vel.x + p.vel.z * p.vel.z).sqrt();
        p.speed = hs;
        if p.grounded && hs > 0.5 {
            p.bob += dt * hs * 1.7;
            p.step_acc += hs * dt;
            let stride = if sprinting { 2.6 } else { 2.0 };
            if p.step_acc > stride {
                p.step_acc = 0.0;
                let v = if p.crouch > 0.5 { 0.4 } else { 1.0 };
                self.audio.play(Snd::Step, v, 0.0);
            }
        }
    }

    fn try_fire(&mut self) -> bool {
        let wi = self.player.cur;
        let def = &WEAPONS[wi];
        {
            let p = &self.player;
            if p.cd > 0.0 || p.reload_t > 0.0 || p.switch_t > 0.0 {
                return false;
            }
        }
        if self.player.ws[wi].mag <= 0 {
            if self.empty_click <= 0.0 {
                self.audio.play(Snd::Empty, 1.0, 0.0);
                self.empty_click = 0.35;
            }
            if self.player.ws[wi].reserve > 0 {
                self.start_reload();
            }
            return true;
        }
        self.player.ws[wi].mag -= 1;
        self.player.cd = def.cd;
        self.stats.shots += 1;

        let (yaw, pitch) = self.view_angles();
        let dir0 = look_dir(yaw, pitch);
        let eye = self.eye();
        let scoped = self.scoped();
        let zoom_k = (self.cur_fov_deg() / BASE_FOV).sqrt();
        let ads = self.player.ads_t > 0.6;
        let mut spread = if ads { def.ads_spread } else { def.spread };
        spread += clamp(self.player.speed / 6.0, 0.0, 1.0) * if wi == 0 && !scoped { 4.0 } else { def.spread * 0.6 };
        if !self.player.grounded {
            spread += 3.0;
        }
        let spread_rad = spread.to_radians();
        let right = dir0.cross(V3::Y).norm();
        let up = right.cross(dir0).norm();
        let (vol, _) = (1.0f32, 0.0);
        let mut any_hit = false;
        for _ in 0..def.pellets {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let rr = spread_rad.tan() * self.rng.f().sqrt();
            let d = (dir0 + right * (a.cos() * rr) + up * (a.sin() * rr)).norm();
            if self.shoot_ray(eye, d, wi, def.pellets > 1) {
                any_hit = true;
            }
        }
        if any_hit {
            self.stats.hits += 1;
        }

        // feedback
        let snd = match wi {
            0 => Snd::Sniper,
            1 => Snd::Rifle,
            2 => Snd::Pistol,
            _ => Snd::Shotgun,
        };
        self.audio.play(snd, vol, 0.0);
        if wi == 0 {
            self.audio.play(Snd::Bolt, 1.0, 0.0);
        }
        let p = &mut self.player;
        p.muzzle = 0.06;
        p.recoil_anim = 1.0;
        if def.noise > 20.0 {
            p.reveal = 2.2; // muzzle flash + crack give away your position
        }
        let rec = def.recoil.to_radians() * if p.ads_t > 0.6 { 0.7 } else { 1.0 } * zoom_k;
        p.kick += rec;
        p.kick_yaw += self.rng.range(-0.3, 0.3) * rec;
        p.shake = p.shake.max(if wi == 0 { 0.4 } else { 0.12 });
        // noise
        let noise = def.noise;
        let rng = &mut self.rng;
        for e in self.enemies.iter_mut() {
            if e.alive() && e.pos.dist(eye) < noise {
                e.hear(eye, rng);
            }
        }
        if p.ws[wi].mag == 0 && p.ws[wi].reserve > 0 && wi != 0 {
            // auto-reload on empty for fast weapons
            self.start_reload();
        }
        true
    }

    /// Cast one bullet. Returns whether it hit an enemy or a latch.
    fn shoot_ray(&mut self, o: V3, d: V3, wi: usize, pellet: bool) -> bool {
        let def = &WEAPONS[wi];
        let wh = self.world.raycast(o, d, 500.0);
        let tw = wh.as_ref().map(|h| h.t).unwrap_or(500.0);
        // padlocked latches on the gates and doors
        let mut latch: Option<(usize, f32)> = None;
        for (i, l) in self.latches.iter().enumerate() {
            if l.broken {
                continue;
            }
            if let Some((t, _)) = l.aabb().ray(o, d, tw) {
                if latch.map(|b| t < b.1).unwrap_or(true) {
                    latch = Some((i, t));
                }
            }
        }
        let tl = latch.map(|l| l.1).unwrap_or(f32::INFINITY);
        let tmax = tw.min(tl);
        // enemies
        let mut best: Option<(usize, f32, bool)> = None;
        for (i, e) in self.enemies.iter().enumerate() {
            if !e.alive() {
                continue;
            }
            let hc = e.head_center();
            let hr = e.head_radius();
            let oc = o - hc;
            let b = oc.dot(d);
            let c = oc.dot(oc) - hr * hr;
            let disc = b * b - c;
            let mut t_head = f32::INFINITY;
            if disc >= 0.0 {
                let t = -b - disc.sqrt();
                if t > 0.0 {
                    t_head = t;
                }
            }
            let t_body = e.body_box().ray(o, d, tmax).map(|x| x.0).unwrap_or(f32::INFINITY);
            let (t, head) = if t_head < tmax && t_head <= t_body + 0.05 {
                (t_head, true)
            } else if t_body < tmax {
                (t_body, false)
            } else {
                continue;
            };
            if best.map(|b| t < b.1).unwrap_or(true) {
                best = Some((i, t, head));
            }
        }
        let tr_col = if wi == 0 { [1.0, 0.95, 0.7] } else { [1.0, 0.85, 0.5] };
        if let Some((i, t, head)) = best {
            let hit_p = o + d * t;
            let mut dmg = def.dmg * if head { def.head } else { 1.0 };
            if def.falloff > 0.0 {
                dmg *= clamp(1.0 - t / def.falloff, 0.12, 1.0);
            }
            let killed = self.enemies[i].damage(dmg, o);
            if !killed {
                self.rally(i, o);
            } else {
                let shove = match wi {
                    0 => 7.5,
                    1 => 3.5,
                    2 => 3.0,
                    _ => 6.0,
                };
                self.enemies[i].shove(hit_p, d, shove * if head { 1.15 } else { 1.0 });
            }
            self.startle_near(o, hit_p, Some(i));
            // blood / stick-man "ink"
            self.burst(hit_p, 7, 3.0, [0.10, 0.10, 0.12], 0.07, 0.5, 9.0, false);
            self.hit_marker = (0.25, if killed { 2 } else if head { 1 } else { 0 });
            if head {
                self.audio.play(Snd::Headshot, 1.0, 0.0);
            } else {
                self.audio.play(Snd::Hit, 1.0, 0.0);
            }
            if killed {
                self.audio.play(Snd::Kill, 1.0, 0.0);
                self.stats.kills += 1;
                if head {
                    self.stats.headshots += 1;
                }
                if t > self.stats.longest {
                    self.stats.longest = t;
                }
                let mut tag = String::from(if head { "HEADSHOT" } else { "ELIMINATED" });
                if o.y - self.enemies[i].pos.y > 5.0 {
                    self.stats.high_kills += 1;
                    tag.push_str(" · HIGH GROUND");
                }
                if t > 60.0 {
                    tag.push_str(&format!(" · {:.0}m", t));
                }
                self.tag(&tag);
                self.maybe_drop(i);
            }
            if !pellet || self.rng.chance(0.3) {
                self.tracers.push(Tracer { a: self.muzzle_world(), b: hit_p, t: 0.1, max: 0.1, col: tr_col, w: 0.05 });
            }
            return true;
        }
        if let Some((li, t)) = latch {
            let p = o + d * t;
            self.break_latch(li);
            self.startle_near(o, p, None);
            self.tracers.push(Tracer { a: self.muzzle_world(), b: p, t: 0.1, max: 0.1, col: tr_col, w: 0.05 });
            return true; // counts as a hit for accuracy
        }
        let end = o + d * tw;
        if let Some(h) = wh {
            self.burst(end, 4, 2.5, [1.0, 0.85, 0.5], 0.04, 0.2, 6.0, true);
            self.burst(end, 4, 1.6, [0.62, 0.6, 0.55], 0.1, 0.6, 1.0, false);
            if !h.terrain {
                if self.decals.len() > 90 {
                    self.decals.remove(0);
                }
                self.decals.push(Decal { p: end, n: h.n, t: 0.0 });
            }
        }
        self.startle_near(o, end, None);
        if !pellet || self.rng.chance(0.4) {
            self.tracers.push(Tracer { a: self.muzzle_world(), b: end, t: 0.08, max: 0.08, col: tr_col, w: 0.04 });
        }
        false
    }

    /// A bullet that cracks past someone, or smacks into something beside them, makes them duck and look.
    fn startle_near(&mut self, o: V3, end: V3, skip: Option<usize>) {
        let seg = end - o;
        let l2 = seg.len2().max(1e-6);
        let rng = &mut self.rng;
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if Some(i) == skip || !e.alive() {
                continue;
            }
            let c = e.pos + v3(0.0, 1.2, 0.0);
            let t = ((c - o).dot(seg) / l2).clamp(0.0, 1.0);
            let miss = (o + seg * t - c).len();
            if miss < 3.0 || end.dist(c) < 4.5 {
                e.startle(o, rng);
            }
        }
    }

    fn break_latch(&mut self, i: usize) {
        if self.latches[i].broken {
            return;
        }
        self.latches[i].broken = true;
        let (pos, door) = (self.latches[i].pos, self.latches[i].door);
        self.open_door(door);
        self.burst(pos, 16, 5.0, [1.0, 0.8, 0.4], 0.05, 0.35, 9.0, true);
        self.burst(pos, 6, 2.0, [0.3, 0.3, 0.32], 0.06, 0.8, 9.0, false);
        let (vol, pan) = self.spatial(pos);
        self.audio.play(Snd::Latch, vol.max(0.4), pan);
        self.hit_marker = (0.25, 1);
        self.tag("LATCH BROKEN");
        let rng = &mut self.rng;
        for e in self.enemies.iter_mut() {
            if e.alive() && e.pos.dist(pos) < 24.0 {
                e.hear(pos, rng);
            }
        }
    }

    fn muzzle_world(&self) -> V3 {
        let (yaw, pitch) = self.view_angles();
        let f = look_dir(yaw, pitch);
        let r = f.cross(V3::Y).norm();
        let u = r.cross(f).norm();
        let ads = self.player.ads_t;
        self.eye() + f * 0.9 + r * (0.16 * (1.0 - ads)) - u * (0.14 * (1.0 - ads) + 0.02)
    }

    /// Somebody just got shot (and lived): the people around them know something is very wrong.
    fn rally(&mut self, victim: usize, target: V3) {
        let pos = self.enemies[victim].pos;
        let rng = &mut self.rng;
        for (j, e) in self.enemies.iter_mut().enumerate() {
            if j == victim || !e.alive() || e.state == State::Combat {
                continue;
            }
            let d = e.pos.dist(pos);
            if d < 14.0 {
                e.alarm(target);
            } else if d < 40.0 {
                e.hear(target, rng);
            }
        }
    }

    fn maybe_drop(&mut self, i: usize) {
        if !self.rng.chance(0.55) {
            return;
        }
        let mut best = 0;
        let mut br = f32::MAX;
        for k in 0..4 {
            let ratio = (self.player.ws[k].reserve as f32 + self.rng.range(0.0, 4.0)) / WEAPONS[k].max_reserve as f32;
            if ratio < br {
                br = ratio;
                best = k;
            }
        }
        let a = self.rng.range(0.0, std::f32::consts::TAU);
        let p0 = self.enemies[i].pos;
        let p = p0 + v3(a.cos() * 0.9, 0.0, a.sin() * 0.9);
        let g = self.world.ground_at(p.x, p.z, 0.2, p0.y + 0.4, 0.0);
        let pos = if self.world.blocked(v3(p.x, g, p.z), 0.2, 0.5) { p0 } else { v3(p.x, g, p.z) };
        self.pickups.push(Pickup { kind: PKind::Ammo(best), pos, t: 0.0, taken: false });
    }

    pub(crate) fn burst(&mut self, pos: V3, n: usize, speed: f32, col: [f32; 3], size: f32, life: f32, grav: f32, add: bool) {
        for _ in 0..n {
            let v = v3(self.rng.range(-1.0, 1.0), self.rng.range(-0.2, 1.0), self.rng.range(-1.0, 1.0)).norm()
                * (speed * self.rng.range(0.4, 1.0));
            let l = life * self.rng.range(0.6, 1.0);
            self.parts.push(Particle { p: pos, v, life: l, max: l, size, col, grav, add });
        }
        if self.parts.len() > 900 {
            let n = self.parts.len() - 900;
            self.parts.drain(0..n);
        }
    }

    fn hurt_player(&mut self, dmg: f32, from: V3) {
        let p = &mut self.player;
        p.hp -= dmg;
        p.dmg_flash = 1.0;
        p.shake = p.shake.max(0.5);
        self.stats.damage_taken += dmg;
        self.indicators.push((from, 2.0));
        self.audio.play(Snd::Hurt, 1.0, 0.0);
    }

    // ------------------------------------------------------------ world ----

    fn update_doors(&mut self, dt: f32) {
        for d in self.world.doors.iter_mut() {
            if d.opening && d.open < 1.0 {
                let speed = match d.kind {
                    DoorKind::BlastRear | DoorKind::BlastWall => 1.5,
                    _ => 0.45,
                };
                d.open = (d.open + dt * speed).min(1.0);
            }
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let pv = PlayerView {
            pos: self.player.pos,
            eye: self.eye(),
            crouch: self.player.crouch > 0.5,
            vis: self.player.vis,
            speed: self.player.speed,
            alive: self.player.hp > 0.0,
        };
        // footsteps carry: sprinting is loud, walking upright is audible up close, crouching is silent
        self.foot_cd -= dt;
        if self.foot_cd <= 0.0 {
            self.foot_cd = 0.4;
            let p = &self.player;
            let radius = if p.sprinting {
                18.0
            } else if p.speed > 0.6 && p.crouch < 0.5 {
                8.0
            } else {
                0.0
            };
            if radius > 0.0 {
                let pos = p.pos;
                let rng = &mut self.rng;
                for e in self.enemies.iter_mut() {
                    if e.alive() && e.state == State::Patrol && e.pos.dist(pos) < radius {
                        e.hear(pos, rng);
                    }
                }
            }
        }
        let corpses: Vec<(usize, V3)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.alive())
            .map(|(i, e)| (i, e.pos))
            .collect();
        {
            let eye = self.eye();
            let n = self.enemies.len();
            for k in 0..4 {
                if n == 0 {
                    break;
                }
                let i = (self.view_rr + k) % n;
                let e = &mut self.enemies[i];
                let head = e.head_center();
                let d = head.dist(eye);
                e.in_view = e.alive() && d < 260.0 && (self.world.los(eye, head) || self.world.los(eye, e.pos + v3(0.0, 0.9, 0.0)));
            }
            self.view_rr = (self.view_rr + 4) % n.max(1);
            for e in self.enemies.iter_mut() {
                if e.in_view || (e.alive() && e.pos.dist(self.player.pos) < 12.0) || e.muzzle > 0.0 {
                    e.spotted = 6.0;
                } else {
                    e.spotted = (e.spotted - dt).max(0.0);
                }
            }
        }
        let claims: Vec<V3> = self.enemies.iter().filter(|e| e.alive()).filter_map(|e| e.cover).collect();
        let mut evs: Vec<Ev> = vec![];
        {
            let world = &self.world;
            let nav = &self.nav;
            let rng = &mut self.rng;
            let mut budget = 3i32;
            for e in self.enemies.iter_mut() {
                let mut ctx = Ctx { world, nav, p: &pv, rng: &mut *rng, ev: &mut evs, corpses: &corpses, budget: &mut budget, claims: &claims };
                e.update(dt, &mut ctx);
            }
        }
        for ev in evs {
            match ev {
                Ev::Shot { from, to, hit, dmg, kind } => {
                    self.tracers.push(Tracer { a: from, b: to, t: 0.12, max: 0.12, col: [1.0, 0.55, 0.3], w: 0.07 });
                    let (vol, pan) = self.spatial(from);
                    let snd = if kind == Kind::Heavy { Snd::EnemyHeavy } else { Snd::EnemyShot };
                    self.audio.play(snd, vol, pan);
                    if hit {
                        self.hurt_player(dmg, from);
                    } else {
                        self.burst(to, 3, 2.0, [0.8, 0.75, 0.6], 0.08, 0.5, 2.0, false);
                    }
                }
                Ev::Shout { pos, target } => {
                    // whoever's close joins the fight at once; the rest come looking
                    let rng = &mut self.rng;
                    for e in self.enemies.iter_mut() {
                        if !e.alive() || e.state == State::Combat {
                            continue;
                        }
                        let d = e.pos.dist(pos);
                        if d < 9.0 {
                            e.alarm(target);
                        } else if d < 45.0 {
                            e.hear(target, rng);
                        }
                    }
                }
                Ev::Body { pos } => {
                    let rng = &mut self.rng;
                    for e in self.enemies.iter_mut() {
                        if e.alive() && e.state == State::Patrol && e.pos.dist(pos) < 45.0 {
                            e.hear(pos, rng);
                        }
                    }
                }
            }
        }
    }

    /// Work out what the "eye" on the HUD should say: hidden / suspicious / searching / detected.
    fn update_stealth(&mut self, dt: f32) {
        let (mut detected, mut searching, mut suspicious) = (false, false, false);
        let (mut aware, mut search) = (0.0f32, 0.0f32);
        for e in &self.enemies {
            match e.state {
                State::Combat => {
                    if e.sees {
                        detected = true;
                    } else {
                        searching = true;
                        search = search.max(1.0 - e.lost_t / 7.0);
                    }
                }
                State::Investigate => {
                    suspicious = true;
                    aware = aware.max(e.aware);
                }
                State::Search => {
                    searching = true;
                    search = search.max((e.aware).clamp(0.3, 1.0));
                }
                State::Patrol => {
                    if e.aware > 0.12 {
                        suspicious = true;
                        aware = aware.max(e.aware);
                    }
                }
                State::Dead => {}
            }
        }
        let new = if detected {
            3
        } else if searching {
            2
        } else if suspicious {
            1
        } else {
            0
        };
        self.alert_cd = (self.alert_cd - dt).max(0.0);
        if new != self.stealth {
            if new == 3 && self.alert_cd <= 0.0 {
                self.audio.play(Snd::Alert, 1.0, 0.0);
                self.alert_cd = 3.0;
            }
            if new == 0 && self.stealth >= 2 {
                self.audio.play(Snd::Clear, 1.0, 0.0);
            }
            self.eye_pulse = 1.0;
            self.stealth = new;
        }
        self.eye_pulse = (self.eye_pulse - dt * 1.6).max(0.0);
        let open = [0.10, 0.55, 0.85, 1.0][new as usize];
        let k = clamp(8.0 * dt, 0.0, 1.0);
        self.eye_open += (open - self.eye_open) * k;
        self.eye_aware += (aware.min(1.0) - self.eye_aware) * clamp(10.0 * dt, 0.0, 1.0);
        self.eye_search += (search - self.eye_search) * clamp(10.0 * dt, 0.0, 1.0);
    }

    fn update_pickups(&mut self, dt: f32) {
        let pp = self.player.pos;
        let mut msgs: Vec<(String, bool)> = vec![];
        for pk in self.pickups.iter_mut() {
            if pk.taken {
                continue;
            }
            pk.t += dt;
            if pk.pos.dist_xz(pp) < 1.4 && (pk.pos.y - pp.y).abs() < 2.0 {
                match pk.kind {
                    PKind::Health => {
                        if self.player.hp < 100.0 {
                            self.player.hp = (self.player.hp + 50.0).min(100.0);
                            self.player.heal_flash = 1.0;
                            pk.taken = true;
                            msgs.push(("+50 HEALTH".to_string(), true));
                        }
                    }
                    PKind::Ammo(w) => {
                        let ws = &mut self.player.ws[w];
                        let def = &WEAPONS[w];
                        if ws.reserve < def.max_reserve {
                            ws.reserve = (ws.reserve + def.pickup).min(def.max_reserve);
                            pk.taken = true;
                            msgs.push((format!("+{} AMMO · {}", def.pickup, def.name), false));
                        }
                    }
                }
            }
        }
        for (m, heal) in msgs {
            self.tag(&m);
            self.audio.play(if heal { Snd::Heal } else { Snd::Pickup }, 1.0, 0.0);
        }
        self.pickups.retain(|p| !p.taken);
    }

    pub(crate) fn update_fx(&mut self, dt: f32) {
        for p in self.parts.iter_mut() {
            p.life -= dt;
            p.v.y -= p.grav * dt;
            p.p += p.v * dt;
            if p.p.y < terrain_h(p.p.x, p.p.z) && p.grav > 0.0 {
                p.v = p.v * 0.2;
                p.p.y = terrain_h(p.p.x, p.p.z) + 0.01;
            }
        }
        self.parts.retain(|p| p.life > 0.0);
        for t in self.tracers.iter_mut() {
            t.t -= dt;
        }
        self.tracers.retain(|t| t.t > 0.0);
        for d in self.decals.iter_mut() {
            d.t += dt;
        }
        self.banner.t = (self.banner.t - dt).max(0.0);
        self.tag.1 = (self.tag.1 - dt).max(0.0);
        self.hit_marker.0 = (self.hit_marker.0 - dt).max(0.0);
        for i in self.indicators.iter_mut() {
            i.1 -= dt;
        }
        self.indicators.retain(|i| i.1 > 0.0);
        let _ = self.step_phase;
        // prisoner escape run
        if self.prisoner.mode == 1 {
            let path = &self.escape;
            let pr = &mut self.prisoner;
            if pr.wp >= path.len() {
                pr.mode = 2;
            } else {
                let goal = path[pr.wp];
                // wait for blast doors before going through them
                let gate_closed = (pr.wp == 3 && self.world.doors[self.door_rear].solid())
                    || (pr.wp == 5 && self.world.doors[self.door_wall].solid());
                let to = goal - pr.pos;
                if to.xz().len() < 0.6 {
                    pr.wp += 1;
                } else if !gate_closed {
                    let dir = to.xz().norm();
                    pr.pos += dir * 4.2 * dt;
                    pr.yaw = turn(pr.yaw, yaw_to(dir), 8.0 * dt);
                    pr.phase += dt * 13.0;
                }
            }
        }
    }

    fn zone_alive(&self, z: usize) -> usize {
        self.enemies.iter().filter(|e| e.zone == z && e.alive()).count()
    }

    pub(crate) fn objective(&self) -> String {
        match self.stage {
            0 => "SHOOT THE LATCH ON A PRISON GATE — SOUTH OR WEST".to_string(),
            1 => "SHOOT THE LATCH ON THE CELL BLOCK DOOR".to_string(),
            2 => "FREE INMATE 13 — CELL AT THE NORTH-WEST END".to_string(),
            _ => "ESCAPE — REACH THE EXTRACTION VAN".to_string(),
        }
    }

    pub(crate) fn objective_pos(&self) -> Option<V3> {
        match self.stage {
            0 => {
                let pp = self.player.pos;
                self.latches.iter().take(2).filter(|l| !l.broken).min_by(|a, b| a.pos.dist(pp).partial_cmp(&b.pos.dist(pp)).unwrap()).map(|l| l.pos)
            }
            1 => Some(self.latches[2].pos),
            2 => Some(self.cell_door_pos),
            _ => Some(self.van),
        }
    }

    /// Unbroken latches that are part of the current objective (drawn with a highlight).
    pub(crate) fn latch_is_target(&self, i: usize) -> bool {
        !self.latches[i].broken && ((self.stage == 0 && i < 2) || (self.stage == 1 && i == 2))
    }

    fn spawn_pack_spots(&mut self, zone: usize) {
        let spots = self.zones[zone].packs.clone();
        for p in spots {
            self.pickups.push(Pickup { kind: PKind::Health, pos: p, t: 0.0, taken: false });
        }
    }

    fn open_door(&mut self, idx: usize) {
        let d = &mut self.world.doors[idx];
        d.opening = true;
        d.locked = false;
        let c = (d.b.min + d.b.max) * 0.5;
        let (vol, pan) = self.spatial(c);
        self.audio.play(Snd::Door, vol.max(0.5), pan);
    }

    fn update_mission(&mut self, dt: f32) {
        // the first time you stand on each lookout
        let pp = self.player.pos;
        let mut reached: Option<usize> = None;
        for (i, l) in self.lookouts.iter().enumerate() {
            if !self.lookout_seen[i] && pp.dist_xz(l.pos) < l.r && (pp.y - l.pos.y).abs() < 1.3 {
                reached = Some(i);
            }
        }
        if let Some(i) = reached {
            self.lookout_seen[i] = true;
            let ws = &mut self.player.ws[0];
            ws.reserve = (ws.reserve + 4).min(WEAPONS[0].max_reserve);
            let name = self.lookouts[i].name;
            if self.lookout_seen.iter().filter(|s| **s).count() == 1 {
                self.say("HIGH GROUND", &format!("{} · +4 sniper rounds · right-click scopes, Shift steadies", name));
            } else {
                self.tag(&format!("HIGH GROUND · {} · +4 ROUNDS", name.to_uppercase()));
            }
            self.audio.play(Snd::Zone, 0.6, 0.0);
        }
        // an area cleared: health packs appear there
        for z in 0..self.zones.len() {
            if self.zone_clear[z] {
                continue;
            }
            let any = self.enemies.iter().any(|e| e.zone == z);
            if any && self.zone_alive(z) == 0 {
                self.zone_clear[z] = true;
                if !self.zones[z].packs.is_empty() {
                    self.spawn_pack_spots(z);
                    let name = self.zones[z].name;
                    self.say(&format!("{} clear", name), "Health packs are waiting there");
                    self.audio.play(Snd::Zone, 0.8, 0.0);
                }
            }
        }
        match self.stage {
            0 => {
                if self.latches[0].broken || self.latches[1].broken {
                    self.stage = 1;
                    self.say("GATE OPEN", "Now shoot the latch on the cell block door");
                    self.audio.play(Snd::Zone, 1.0, 0.0);
                }
            }
            1 => {
                if self.latches[2].broken {
                    self.stage = 2;
                    self.say("CELL BLOCK OPEN", "Inmate 13 is in the north-west cell, ground floor");
                    self.audio.play(Snd::Zone, 1.0, 0.0);
                }
            }
            3 => {
                if self.player.pos.dist_xz(self.van) < 5.0 {
                    self.state = WON;
                    self.death_t = 0.0;
                    self.input = Input::default();
                    self.audio.play(Snd::Zone, 1.0, 0.0);
                }
            }
            _ => {}
        }

        // blast sequence after the break-out
        if self.charges_t > 0.0 {
            self.charges_t -= dt;
            if self.charges_t <= 2.0 && !self.rear_blown {
                self.rear_blown = true;
                self.blast(self.door_rear);
            }
            if self.charges_t <= 0.0 && !self.wall_blown {
                self.wall_blown = true;
                self.blast(self.door_wall);
            }
        }
    }

    fn blast(&mut self, idx: usize) {
        let d = &mut self.world.doors[idx];
        d.opening = true;
        d.locked = false;
        let c = (d.b.min + d.b.max) * 0.5;
        self.burst(c, 70, 9.0, [1.0, 0.65, 0.2], 0.5, 1.1, 2.0, true);
        self.burst(c, 60, 6.0, [0.5, 0.48, 0.45], 0.8, 2.2, -0.5, false);
        let (vol, pan) = self.spatial(c);
        self.audio.play(Snd::Explosion, vol.max(0.45), pan);
        let dist = c.dist(self.eye());
        self.player.shake = self.player.shake.max((1.0 - dist / 60.0).max(0.0));
    }

    fn free_prisoner(&mut self) {
        if self.stage != 2 {
            return;
        }
        self.stage = 3;
        self.world.doors[self.door_cell].opening = true;
        self.world.doors[self.door_cell].locked = false;
        self.prisoner.mode = 1;
        self.charges_t = 4.5;
        self.audio.play(Snd::Alarm, 1.0, 0.0);
        self.audio.play(Snd::Door, 0.8, 0.0);
        self.say("INMATE FREED — ALARM!", "Charges set on the rear wall · reinforcements inbound · reach the van");
        let pp = self.player.pos;
        let defs = self.reinforcements.clone();
        for (k, p) in defs {
            let mut e = Enemy::new(k, p, 0.0, vec![], Z_REINF, &mut self.rng);
            e.stationary = false;
            e.alarm(pp);
            self.enemies.push(e);
        }
        // the guards close by come running; the rest of the prison starts searching
        let rng = &mut self.rng;
        for e in self.enemies.iter_mut() {
            if e.alive() && (e.zone == Z_YARD || e.zone == Z_BLOCK) {
                if e.pos.dist(pp) < 25.0 {
                    e.alarm(pp);
                } else {
                    e.hear(pp, rng);
                }
            }
        }
    }
}

fn turn(cur: f32, target: f32, max: f32) -> f32 {
    cur + clamp(angle_diff(cur, target), -max, max)
}
