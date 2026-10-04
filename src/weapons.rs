use crate::gfx::Col;
use crate::math::*;

pub struct WDef {
    pub name: &'static str,
    pub mag: i32,
    pub max_reserve: i32,
    pub cd: f32,
    pub auto: bool,
    pub dmg: f32,
    pub pellets: i32,
    pub spread: f32,
    pub ads_spread: f32,
    pub head: f32,
    pub reload: f32,
    pub recoil: f32,
    pub noise: f32,
    pub ads_fov: f32,
    pub move_mul: f32,
    pub falloff: f32,
    pub pickup: i32,
}

pub const WEAPONS: [WDef; 4] = [
    WDef {
        name: "M24 BOLT SNIPER",
        mag: 5,
        max_reserve: 40,
        cd: 1.35,
        auto: false,
        dmg: 70.0, // a body shot hurts but never kills outright…
        pellets: 1,
        spread: 3.5,
        ads_spread: 0.0,
        head: 5.0, // …a headshot always does
        reload: 2.8,
        recoil: 3.2,
        noise: 55.0,
        ads_fov: 12.0,
        move_mul: 0.9,
        falloff: 0.0,
        pickup: 8,
    },
    WDef {
        name: "AK-74 RIFLE",
        mag: 30,
        max_reserve: 150,
        cd: 0.105,
        auto: true,
        dmg: 24.0,
        pellets: 1,
        spread: 1.6,
        ads_spread: 0.55,
        head: 2.5,
        reload: 2.1,
        recoil: 0.85,
        noise: 60.0,
        ads_fov: 52.0,
        move_mul: 0.95,
        falloff: 0.0,
        pickup: 30,
    },
    WDef {
        name: "MK23 SILENCED",
        mag: 12,
        max_reserve: 72,
        cd: 0.26,
        auto: false,
        dmg: 30.0,
        pellets: 1,
        spread: 0.9,
        ads_spread: 0.3,
        head: 3.5,
        reload: 1.4,
        recoil: 1.3,
        noise: 9.0,
        ads_fov: 58.0,
        move_mul: 1.0,
        falloff: 0.0,
        pickup: 24,
    },
    WDef {
        name: "SPAS-12 SHOTGUN",
        mag: 6,
        max_reserve: 36,
        cd: 0.85,
        auto: false,
        dmg: 12.0,
        pellets: 9,
        spread: 4.6,
        ads_spread: 3.4,
        head: 1.6,
        reload: 2.4,
        recoil: 3.0,
        noise: 55.0,
        ads_fov: 60.0,
        move_mul: 0.95,
        falloff: 32.0,
        pickup: 8,
    },
];

#[derive(Clone, Copy)]
pub struct WState {
    pub mag: i32,
    pub reserve: i32,
}

pub fn initial_ammo() -> [WState; 4] {
    [
        WState { mag: 5, reserve: 25 },
        WState { mag: 30, reserve: 90 },
        WState { mag: 12, reserve: 48 },
        WState { mag: 6, reserve: 18 },
    ]
}

/// A gun is a handful of "bones": (from, to, thickness, colour) in gun space.
/// Gun space: +x right, +y up, -z forward (away from the camera).
pub type Bone = (V3, V3, f32, Col);

// everything in your hands is quiet charcoal
const DARK: Col = [0.07, 0.07, 0.08, 0.0];
const STEEL: Col = [0.17, 0.18, 0.20, 0.0];
const WOOD: Col = [0.12, 0.12, 0.14, 0.0];
const TAN: Col = [0.13, 0.13, 0.15, 0.0];

pub fn gun_bones(w: usize) -> Vec<Bone> {
    match w {
        0 => vec![
            (v3(0.0, 0.0, -0.25), v3(0.0, 0.0, -1.15), 0.028, DARK),
            (v3(0.0, 0.0, -1.05), v3(0.0, 0.0, -1.2), 0.04, DARK),
            (v3(0.0, -0.01, 0.1), v3(0.0, -0.01, -0.32), 0.05, STEEL),
            (v3(0.0, -0.04, 0.1), v3(0.0, -0.1, 0.42), 0.05, TAN),
            (v3(0.0, 0.062, -0.28), v3(0.0, 0.062, 0.02), 0.038, DARK),
            (v3(0.0, 0.062, -0.33), v3(0.0, 0.062, -0.26), 0.05, DARK),
            (v3(0.0, 0.062, 0.02), v3(0.0, 0.062, 0.07), 0.046, DARK),
            (v3(0.0, -0.03, -0.12), v3(0.0, -0.11, -0.1), 0.034, DARK),
        ],
        1 => vec![
            (v3(0.0, 0.0, -0.3), v3(0.0, 0.0, -0.78), 0.026, DARK),
            (v3(0.0, -0.01, 0.1), v3(0.0, -0.01, -0.36), 0.052, STEEL),
            (v3(0.0, -0.03, 0.1), v3(0.0, -0.08, 0.44), 0.05, WOOD),
            (v3(0.0, 0.045, -0.3), v3(0.0, 0.045, -0.05), 0.032, STEEL),
            (v3(0.0, -0.05, -0.18), v3(0.0, -0.18, -0.1), 0.038, WOOD),
            (v3(0.0, -0.04, -0.04), v3(0.0, -0.12, 0.0), 0.036, DARK),
            (v3(0.0, 0.065, -0.08), v3(0.0, 0.065, -0.02), 0.02, DARK),
        ],
        2 => vec![
            (v3(0.0, 0.0, -0.02), v3(0.0, 0.0, -0.3), 0.042, STEEL),
            (v3(0.0, 0.0, -0.3), v3(0.0, 0.0, -0.58), 0.034, DARK),
            (v3(0.0, -0.01, 0.08), v3(0.0, -0.1, 0.12), 0.04, DARK),
            (v3(0.0, 0.032, -0.02), v3(0.0, 0.032, 0.0), 0.016, DARK),
        ],
        _ => vec![
            (v3(0.0, 0.0, -0.1), v3(0.0, 0.0, -0.9), 0.034, DARK),
            (v3(0.0, -0.05, -0.1), v3(0.0, -0.05, -0.7), 0.026, STEEL),
            (v3(0.0, -0.05, -0.42), v3(0.0, -0.05, -0.62), 0.06, WOOD),
            (v3(0.0, -0.01, 0.15), v3(0.0, -0.01, -0.12), 0.055, STEEL),
            (v3(0.0, -0.03, 0.15), v3(0.0, -0.1, 0.5), 0.055, WOOD),
            (v3(0.0, -0.05, -0.0), v3(0.0, -0.13, 0.03), 0.036, DARK),
        ],
    }
}

/// Muzzle position in gun space.
pub fn muzzle_pos(w: usize) -> V3 {
    match w {
        0 => v3(0.0, 0.0, -1.22),
        1 => v3(0.0, 0.0, -0.8),
        2 => v3(0.0, 0.0, -0.6),
        _ => v3(0.0, 0.0, -0.92),
    }
}
