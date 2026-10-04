//! The map: a prison in the middle of a valley, a village on the southern approach, a farm to the east and
//! a lookout house on the hill in the south-east. You start low and far away in the southern woods.

use crate::builder::*;
use crate::enemy::Kind;
use crate::gfx::{Col, Mesh};
use crate::math::*;
use crate::world::*;

pub struct Spawn {
    pub kind: Kind,
    pub pos: V3,
    pub yaw: f32,
    pub patrol: Vec<V3>,
    pub zone: usize,
}

pub struct Lookout {
    pub pos: V3,
    pub r: f32,
    pub name: &'static str,
}

pub struct Zone {
    pub name: &'static str,
    pub packs: Vec<V3>,
}

pub struct Level {
    pub world: World,
    pub mesh: Mesh,
    pub fence_mesh: Mesh,
    pub spawns: Vec<Spawn>,
    pub zones: Vec<Zone>,
    pub lookouts: Vec<Lookout>,
    pub latches: Vec<Latch>,
    pub ammo_spots: Vec<(V3, usize)>,
    pub van: V3,
    pub cell_pos: V3,
    pub cell_door_pos: V3,
    pub spawn_pos: V3,
    pub spawn_yaw: f32,
    #[allow(dead_code)]
    pub gate: usize,
    #[allow(dead_code)]
    pub side_gate: usize,
    #[allow(dead_code)]
    pub main_door: usize,
    pub cell_door: usize,
    pub blast_rear: usize,
    pub blast_wall: usize,
    pub indoor_min: V3,
    pub indoor_max: V3,
    pub escape: Vec<V3>,
    pub reinforcements: Vec<(Kind, V3)>,
}

pub const Z_VILLAGE: usize = 0;
pub const Z_HILL: usize = 1;
pub const Z_FARM: usize = 2;
pub const Z_YARD: usize = 3;
pub const Z_BLOCK: usize = 4;
pub const Z_REINF: usize = 5;

/// Roads and trails: polylines with a half-width.
pub const ROADS: &[(&[(f32, f32)], f32)] = &[
    (&[(-28.0, 160.0), (-33.0, 128.0), (-46.0, 112.0), (-46.0, 44.0), (-22.0, 37.0), (0.0, 35.0), (0.0, 31.0)], 3.4),
    (&[(0.0, -53.0), (0.0, -120.0)], 3.4),
    (&[(-84.0, 78.0), (-10.0, 78.0)], 3.8),
    (&[(-10.0, 78.0), (14.0, 81.0), (32.0, 83.0), (47.0, 81.0)], 1.6),
    (&[(4.0, 35.0), (44.0, 25.0), (60.0, 6.0), (76.0, -6.0)], 1.8),
    (&[(-46.0, 44.0), (-53.0, 22.0), (-45.0, 0.0), (-41.0, -0.2)], 1.5),
];

fn seg_dist(px: f32, pz: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vz) = (b.0 - a.0, b.1 - a.1);
    let (wx, wz) = (px - a.0, pz - a.1);
    let t = clamp((wx * vx + wz * vz) / (vx * vx + vz * vz).max(1e-6), 0.0, 1.0);
    let (cx, cz) = (a.0 + vx * t, a.1 + vz * t);
    ((px - cx).powi(2) + (pz - cz).powi(2)).sqrt()
}

/// Distance outside the nearest road's edge (negative on the road).
pub fn road_dist(x: f32, z: f32) -> f32 {
    let mut d = 1e9f32;
    for (pts, hw) in ROADS {
        for i in 0..pts.len() - 1 {
            d = d.min(seg_dist(x, z, pts[i], pts[i + 1]) - hw);
        }
    }
    d
}

pub fn in_compound(x: f32, z: f32, pad: f32) -> bool {
    x.abs() < 41.0 + pad && z < 32.0 + pad && z > -54.5 - pad
}
pub fn in_village(x: f32, z: f32, pad: f32) -> bool {
    x > -84.0 - pad && x < -10.0 + pad && z > 40.0 - pad && z < 114.0 + pad
}
pub fn in_farm(x: f32, z: f32, pad: f32) -> bool {
    x > 52.0 - pad && x < 118.0 + pad && z > -48.0 - pad && z < 24.0 + pad
}

fn terrain_color(x: f32, z: f32, h: f32) -> Col {
    if in_compound(x, z, 0.0) {
        return [0.70, 0.70, 0.68, 0.0];
    }
    let v = 0.5 + 0.5 * (x * 0.021 + 1.7).sin() * (z * 0.017).cos();
    let mut base = [lerpf(0.55, 0.62, v), lerpf(0.62, 0.68, v), lerpf(0.54, 0.60, v)];
    // ploughed fields by the farm
    if x > 56.0 && x < 116.0 && z > 26.0 - 80.0 && z < 22.0 && !(x > 70.0 && x < 104.0 && z > -30.0 && z < 16.0) {
        let stripe = ((x * 0.9).sin() * 0.5 + 0.5) * 0.06;
        base = [0.66 + stripe, 0.62 + stripe, 0.50 + stripe];
    }
    let lift = smoothstep(4.0, 34.0, h) * 0.08;
    let rd = road_dist(x, z);
    let k = (1.0 - smoothstep(-0.6, 0.6, rd)) * 0.8;
    let road = [0.75, 0.72, 0.66];
    let k2 = if in_village(x, z, 0.0) { 0.25 } else { 0.0 };
    let k = k.max(k2);
    [lerpf(base[0] + lift, road[0], k), lerpf(base[1] + lift, road[1], k), lerpf(base[2] + lift, road[2], k), 0.0]
}

pub fn build_terrain(mesh: &mut Mesh) {
    let half = 205.0;
    let cell = 2.5;
    let n = (half * 2.0 / cell) as usize;
    let vert = |m: &mut Mesh, x: f32, z: f32| {
        let h = terrain_h(x, z);
        m.vert(v3(x, h, z), terrain_normal(x, z), terrain_color(x, z, h));
    };
    for i in 0..n {
        for j in 0..n {
            let x0 = -half + i as f32 * cell;
            let z0 = -half + j as f32 * cell;
            let (x1, z1) = (x0 + cell, z0 + cell);
            vert(mesh, x0, z0);
            vert(mesh, x0, z1);
            vert(mesh, x1, z1);
            vert(mesh, x0, z0);
            vert(mesh, x1, z1);
            vert(mesh, x1, z0);
        }
    }
}

pub fn sp(kind: Kind, x: f32, y: f32, z: f32, yaw: f32, zone: usize, patrol: &[(f32, f32)]) -> Spawn {
    let elevated = y > terrain_h(x, z) + 1.0;
    Spawn {
        kind,
        pos: v3(x, y, z),
        yaw,
        zone,
        patrol: patrol
            .iter()
            .map(|p| {
                // elevated posts (tiers, roofs) patrol at their own height; everyone else on the ground
                let g = terrain_h(p.0, p.1);
                v3(p.0, if elevated { y } else { g.max(y.min(0.5)) }, p.1)
            })
            .collect(),
    }
}

/// A plain house shell on a slab: four walls with doors (side 0 east, 1 south, 2 west, 3 north; position is
/// the coordinate along that wall) and evenly spaced windows. Returns the floor height.
#[allow(clippy::too_many_arguments)]
pub fn shell(b: &mut Builder, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, h: f32, c: Col, doors: &[(u8, f32)], windows: bool) {
    let t = 0.3;
    let ht = t * 0.5;
    for side in 0..4u8 {
        let (a, bb, len) = match side {
            0 => ((x1 - ht, z0 + t), (x1 - ht, z1 - t), z1 - z0 - 2.0 * t),
            1 => ((x0, z1 - ht), (x1, z1 - ht), x1 - x0),
            2 => ((x0 + ht, z0 + t), (x0 + ht, z1 - t), z1 - z0 - 2.0 * t),
            _ => ((x0, z0 + ht), (x1, z0 + ht), x1 - x0),
        };
        let along0 = if side % 2 == 0 { a.1 } else { a.0 };
        let mut ops: Vec<Opening> = vec![];
        for &(s, pos) in doors {
            if s == side {
                let p = pos - along0;
                ops.push((p - 0.68, p + 0.68, y, y + 2.3));
            }
        }
        if windows {
            let n = ((len - 1.6) / 3.1).floor().max(0.0) as usize;
            for i in 0..n {
                let p = len * (i as f32 + 1.0) / (n as f32 + 1.0);
                if ops.iter().any(|o| (o.0 + o.1) * 0.5 > p - 1.6 && (o.0 + o.1) * 0.5 < p + 1.6) {
                    continue;
                }
                ops.push((p - 0.55, p + 0.55, y + 1.0, y + 2.1));
            }
        }
        b.wall(a, bb, t, y, y + h, c, &ops);
    }
}

/// Parapet round a flat roof, with an optional gap (side, start, end along that side).
pub fn parapet(b: &mut Builder, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, gap: Option<(u8, f32, f32)>) {
    let t = 0.22;
    for side in 0..4u8 {
        let (a, bb) = match side {
            0 => ((x1 - t * 0.5, z0), (x1 - t * 0.5, z1)),
            1 => ((x0, z1 - t * 0.5), (x1, z1 - t * 0.5)),
            2 => ((x0 + t * 0.5, z0), (x0 + t * 0.5, z1)),
            _ => ((x0, z0 + t * 0.5), (x1, z0 + t * 0.5)),
        };
        let along0 = if side % 2 == 0 { a.1 } else { a.0 };
        let ops: Vec<Opening> = match gap {
            Some((s, g0, g1)) if s == side => vec![(g0 - along0, g1 - along0, y - 1.0, y + 9.0)],
            _ => vec![],
        };
        b.wall(a, bb, t, y, y + 0.9, CONCRETE, &ops);
    }
}

pub fn build_level() -> Level {
    let mut b = Builder::new(1337);
    build_terrain(&mut b.mesh);
    let mut doors: Vec<Door> = vec![];
    let mut spawns: Vec<Spawn> = vec![];
    let mut latches: Vec<Latch> = vec![];
    let mut lookouts: Vec<Lookout> = vec![];
    let pi = std::f32::consts::PI;

    // ------------------------------------------------ perimeter: chain-link fence with two gates
    let wh = 4.0;
    let (fx, fzs, fzn) = (40.0f32, 31.0f32, -53.0f32);
    b.fence_run((-fx, fzs), (fx, fzs), wh, &[(-4.0, 4.0)]);
    b.fence_run((-fx, fzn), (fx, fzn), wh, &[(-4.0, 4.0)]);
    b.fence_run((-fx, fzn), (-fx, fzs), wh, &[(-2.0, 1.6)]);
    b.fence_run((fx, fzn), (fx, fzs), wh, &[]);
    let gate = doors.len();
    doors.push(Door {
        b: Aabb { min: v3(-4.0, 0.0, fzs - 0.15), max: v3(4.0, wh - 0.3, fzs + 0.15) },
        kind: DoorKind::Gate,
        open: 0.0,
        opening: false,
        slide: v3(8.0, 0.0, 0.0),
        fence: true,
        locked: true,
        col: METAL,
    });
    latches.push(Latch { pos: v3(-3.66, 1.35, fzs + 0.06), half: v3(0.16, 0.2, 0.12), door: gate, broken: false, n: v3(0.0, 0.0, 1.0) });
    let side_gate = doors.len();
    doors.push(Door {
        b: Aabb { min: v3(-fx - 0.15, 0.0, -2.0), max: v3(-fx + 0.15, wh - 0.3, 1.6) },
        kind: DoorKind::SideGate,
        open: 0.0,
        opening: false,
        slide: v3(0.0, 0.0, -3.7),
        fence: true,
        locked: true,
        col: METAL,
    });
    latches.push(Latch { pos: v3(-fx - 0.06, 1.35, 1.26), half: v3(0.12, 0.2, 0.16), door: side_gate, broken: false, n: v3(-1.0, 0.0, 0.0) });
    let blast_wall = doors.len();
    doors.push(Door {
        b: Aabb { min: v3(-4.0, 0.0, fzn - 0.15), max: v3(4.0, wh, fzn + 0.15) },
        kind: DoorKind::BlastWall,
        open: 0.0,
        opening: false,
        slide: v3(0.0, -wh, 0.0),
        fence: true,
        locked: true,
        col: METAL,
    });

    // ------------------------------------------------ octagonal guard towers
    let towers = [
        (-36.0, 27.0, 8.0, 0usize, 1.0f32),
        (36.0, 27.0, 8.0, 4, -1.0),
        (-36.0, -49.0, 8.0, 0, -1.0),
        (36.0, -49.0, 8.0, 4, 1.0),
        (30.0, -3.0, 7.2, 4, -1.0),
    ];
    let names = ["south-west tower", "south-east tower", "north-west tower", "north-east tower", "yard tower"];
    for (i, &(cx, cz, ph, ks, vs)) in towers.iter().enumerate() {
        let top = b.lookout(cx, cz, 3.2, 0.0, ph, ks, vs, true);
        lookouts.push(Lookout { pos: top, r: 3.0, name: names[i] });
    }

    // ------------------------------------------------ cell block
    let (bx0, bx1, bz0, bz1) = (-22.0f32, 22.0f32, -40.0f32, -8.0f32);
    let bh = 9.0;
    b.deco(v3(bx0 + 0.8, -0.3, bz0 + 0.8), v3(bx1 - 0.8, 0.03, bz1 - 0.8), FLOOR_IN);
    // the walls as boxes (thick, axis-aligned) with the main door, firing slits and the rear blast section
    let wallx = |b: &mut Builder, x0: f32, x1: f32, z0: f32, z1: f32, gaps: &[(f32, f32, f32, f32)]| {
        b.wall(((x0), (z0 + z1) * 0.5), (x1, (z0 + z1) * 0.5), z1 - z0, 0.0, bh, CONCRETE, &gaps.iter().map(|g| (g.0 - x0, g.1 - x0, g.2, g.3)).collect::<Vec<_>>());
    };
    wallx(&mut b, bx0, bx1, bz1 - 0.8, bz1, &[(-2.5, 2.5, 0.0, 3.6), (-9.0, -7.4, 1.9, 3.0), (7.4, 9.0, 1.9, 3.0)]);
    wallx(&mut b, bx0, bx1, bz0, bz0 + 0.8, &[(-3.0, 3.0, 0.0, 4.0)]);
    b.wall((bx0 + 0.4, bz0 + 0.8), (bx0 + 0.4, bz1 - 0.8), 0.8, 0.0, bh, CONCRETE, &[]);
    b.wall((bx1 - 0.4, bz0 + 0.8), (bx1 - 0.4, bz1 - 0.8), 0.8, 0.0, bh, CONCRETE, &[]);
    b.solid(v3(bx0, bh, bz0), v3(bx1, bh + 0.5, bz1), CONCRETE_D);
    let py = bh + 0.5;
    parapet(&mut b, bx0, bz0, bx1, bz1, py, Some((2, -23.6, -21.4)));
    b.solid(v3(-6.0, py, -26.0), v3(-3.0, py + 1.3, -23.0), METAL);
    b.solid(v3(8.0, py, -18.0), v3(10.0, py + 0.9, -16.0), METAL);
    b.solid(v3(17.0, py, -33.0), v3(19.5, py + 1.1, -30.0), CRATE);
    b.solid(v3(-15.0, py, -36.0), v3(-12.6, py + 1.0, -33.6), METAL);
    // roof stair up the west face, with a little landing at the top
    let end = b.flight((bx0 - 1.1, -8.6), (0.0, -1.0), STAIR_W, 0.0, py, METAL, true);
    b.solid(v3(bx0 - 2.1, py - 0.3, end.1 - 2.1), v3(bx0, py, end.1), METAL);
    b.solid(v3(bx0 - 2.0, 0.0, end.1 - 2.0), v3(bx0 - 1.8, py - 0.3, end.1 - 1.8), METAL);
    b.rail(v3(bx0 - 2.1, 0.2, -8.6), v3(bx0 - 2.1, py, end.1));

    // interior: two raised tiers of cells, each reached by a smooth flight in the atrium
    for side in [-1.0f32, 1.0] {
        let (x0, x1) = if side < 0.0 { (-21.2, -14.0) } else { (14.0, 21.2) };
        b.solid(v3(x0, 0.0, bz0 + 0.8), v3(x1, 4.6, bz1 - 0.8), WALL_IN);
        let fxe: f32 = if side < 0.0 { -14.0 } else { 14.0 };
        for k in 0..4 {
            let zc = -20.5 - 5.0 * k as f32;
            let dx = if side < 0.0 { 0.08 } else { -0.08 };
            b.deco(v3(fxe.min(fxe + dx), 0.0, zc - 0.9), v3(fxe.max(fxe + dx), 2.4, zc + 0.9), [0.18, 0.2, 0.22, 0.0]);
            b.deco(v3(fxe.min(fxe + dx * 1.5), 0.9, zc - 0.5), v3(fxe.max(fxe + dx * 1.5), 1.5, zc + 0.5), [0.05, 0.05, 0.06, 0.0]);
        }
        let sx = if side < 0.0 { -13.0 } else { 13.0 };
        let top = b.flight((sx, -9.3), (0.0, -1.0), STAIR_W, 0.0, 4.6, METAL, true);
        b.solid(v3(sx - 1.0, 0.0, top.1 - 2.0), v3(sx + 1.0, 4.6, top.1), METAL);
        let rx = if side < 0.0 { sx + 1.0 } else { sx - 1.0 };
        b.rail(v3(rx, 0.15, -9.3), v3(rx, 4.6, top.1));
    }
    // tier rails, with a gap where each stair landing meets its tier
    for side in [-1.0f32, 1.0] {
        let x = if side < 0.0 { -14.07 } else { 14.07 };
        let ops = [(bz1 - 0.8 - (-15.7), bz1 - 0.8 - (-18.0), 4.0, 7.0)];
        // the wall runs from z = bz1-0.8 towards bz0+0.8, so measure openings from that end
        b.wall((x, bz1 - 0.8), (x, bz0 + 0.8), 0.14, 4.6, 5.6, METAL, &[(ops[0].0, ops[0].1, ops[0].2, ops[0].3)]);
    }
    for (px, pz) in [(-8.0, -18.0), (8.0, -18.0), (-8.0, -30.0), (8.0, -30.0)] {
        b.solid(v3(px - 0.5, 0.0, pz - 0.5), v3(px + 0.5, bh, pz + 0.5), CONCRETE);
    }
    b.solid(v3(-3.0, 0.0, -24.5), v3(3.0, 1.1, -22.5), WOOD);
    b.solid(v3(-1.0, 1.1, -24.2), v3(1.0, 1.5, -23.8), BLACK);
    b.solid(v3(-11.0, 0.0, -14.0), v3(-7.0, 0.5, -13.4), WOOD);
    b.solid(v3(7.0, 0.0, -14.0), v3(11.0, 0.5, -13.4), WOOD);
    b.solid(v3(-12.5, 0.0, -25.0), v3(-11.0, 1.1, -23.5), CRATE);
    b.solid(v3(10.0, 0.0, -35.0), v3(12.0, 1.1, -33.5), CRATE);
    b.solid(v3(4.0, 0.0, -37.5), v3(6.5, 1.0, -36.0), CRATE);
    b.solid(v3(-6.5, 0.0, -11.6), v3(-5.3, 1.2, -10.4), CRATE);
    for zc in [-14.0, -22.0, -30.0, -37.0] {
        b.deco(v3(-0.7, bh - 0.35, zc - 0.25), v3(0.7, bh, zc + 0.25), LAMP);
    }
    let main_door = doors.len();
    doors.push(Door {
        b: Aabb { min: v3(-2.5, 0.0, bz1 - 0.8), max: v3(2.5, 3.6, bz1) },
        kind: DoorKind::Main,
        open: 0.0,
        opening: false,
        slide: v3(0.0, 3.6, 0.0),
        fence: false,
        locked: true,
        col: [0.25, 0.27, 0.30, 0.0],
    });
    latches.push(Latch { pos: v3(1.9, 1.45, bz1 + 0.07), half: v3(0.17, 0.22, 0.08), door: main_door, broken: false, n: v3(0.0, 0.0, 1.0) });
    let blast_rear = doors.len();
    doors.push(Door {
        b: Aabb { min: v3(-3.0, 0.0, bz0), max: v3(3.0, 4.0, bz0 + 0.8) },
        kind: DoorKind::BlastRear,
        open: 0.0,
        opening: false,
        slide: v3(0.0, -4.0, 0.0),
        fence: false,
        locked: true,
        col: [0.55, 0.56, 0.58, 0.0],
    });
    // inmate 13's cell (west side, north end)
    b.wall((-13.9, -33.15), (-6.0, -33.15), 0.3, 0.0, 3.6, WALL_IN, &[(3.1, 4.7, 0.0, 3.6)]);
    b.wall((-6.15, -39.2), (-6.15, -33.3), 0.3, 0.0, 3.6, WALL_IN, &[]);
    b.deco(v3(-12.5, 0.0, -38.9), v3(-11.0, 0.5, -37.5), [0.62, 0.55, 0.45, 0.0]);
    let cell_door = doors.len();
    doors.push(Door {
        b: Aabb { min: v3(-10.8, 0.0, -33.3), max: v3(-9.2, 3.0, -33.0) },
        kind: DoorKind::Cell,
        open: 0.0,
        opening: false,
        slide: v3(0.0, 3.0, 0.0),
        fence: false,
        locked: true,
        col: [0.2, 0.22, 0.25, 0.0],
    });

    // ------------------------------------------------ the yard: huts, sheds and walls that break up sightlines
    // octagonal guard hut by the gate
    {
        let (cx, cz, r) = (13.0, 22.0, 2.8);
        b.poly(octagon(cx, cz, r), -0.3, 0.15, CONCRETE_D, SOLID);
        b.ring(cx, cz, r, 0.25, 0.15, 2.75, PLASTER, SOLID, &|k| match k {
            6 => vec![(-0.65, 0.65, 0.15, 2.3)],
            _ => vec![(-0.5, 0.5, 1.1, 2.05)],
        });
        b.poly(octagon(cx, cz, r + 0.25), 2.75, 2.95, CONCRETE_D, SOLID);
        b.hip(&octagon(cx, cz, r + 0.25), 2.95, 1.2, ROOF_R);
        b.solid(v3(cx - 0.8, 0.15, cz + 0.6), v3(cx + 0.8, 0.95, cz + 1.4), WOOD);
    }
    // workshop with two rooms and a roof you can climb onto
    {
        let (x0, z0, x1, z1) = (-34.0, 11.0, -24.0, 21.0);
        b.solid(v3(x0, -0.3, z0), v3(x1, 0.15, z1), CONCRETE_D);
        shell(&mut b, x0, z0, x1, z1, 0.15, 3.05, PLASTER, &[(0, 13.0), (3, -31.0)], true);
        b.wall((-29.0, z0 + 0.3), (-29.0, z1 - 0.3), 0.2, 0.15, 3.05, WALL_IN, &[(5.2, 6.6, 0.15, 2.3)]);
        b.solid(v3(x0, 3.0, z0), v3(x1, 3.25, z1), CONCRETE_D);
        parapet(&mut b, x0, z0, x1, z1, 3.25, Some((0, 15.0, 17.0)));
        b.flight((-19.0, 16.0), (-1.0, 0.0), STAIR_W, 0.0, 3.25, CONCRETE_D, true);
        b.solid(v3(-33.4, 0.15, 11.6), v3(-31.0, 1.0, 12.6), WOOD);
        b.solid(v3(-27.6, 0.15, 19.2), v3(-24.6, 1.0, 20.4), METAL);
        b.solid(v3(-33.3, 0.15, 17.5), v3(-32.2, 1.9, 20.3), WOOD_D);
    }
    // mess hall east of the cell block
    {
        let (x0, z0, x1, z1) = (25.0, -36.0, 37.0, -18.0);
        b.solid(v3(x0, -0.3, z0), v3(x1, 0.15, z1), CONCRETE_D);
        shell(&mut b, x0, z0, x1, z1, 0.15, 3.4, PLASTER, &[(2, -22.0), (1, 31.0), (2, -32.0)], true);
        b.wall((x0 + 0.3, -27.0), (x1 - 0.3, -27.0), 0.2, 0.15, 3.4, WALL_IN, &[(3.0, 4.4, 0.15, 2.3), (8.0, 9.4, 0.15, 2.3)]);
        b.solid(v3(x0, 3.35, z0), v3(x1, 3.6, z1), CONCRETE_D);
        for (tx, tz) in [(28.0, -22.0), (33.0, -22.0), (28.0, -31.5)] {
            b.solid(v3(tx - 1.4, 0.15, tz - 0.5), v3(tx + 1.4, 0.95, tz + 0.5), WOOD);
        }
        b.solid(v3(34.5, 0.15, -35.4), v3(36.4, 1.0, -28.0), METAL);
    }
    // low walls and a tall one — half cover, full cover, and broken sightlines
    b.wall((-10.0, 24.0), (-3.0, 24.0), 0.3, 0.0, 1.2, CONCRETE_D, &[]);
    b.wall((3.0, 19.0), (10.0, 19.0), 0.3, 0.0, 1.2, CONCRETE_D, &[]);
    b.wall((19.0, 13.0), (19.0, 3.0), 0.35, 0.0, 2.8, CONCRETE, &[]);
    b.wall((-21.0, 1.0), (-11.0, 1.0), 0.35, 0.0, 2.6, CONCRETE, &[(4.0, 5.6, 0.0, 2.2)]);
    b.wall((-21.0, 1.0), (-21.0, 7.0), 0.35, 0.0, 2.6, CONCRETE, &[]);
    // containers, crates, jersey barriers
    b.solid(v3(-34.0, 0.0, -4.0), v3(-28.0, 2.6, -1.6), [0.20, 0.22, 0.25, 0.0]);
    b.solid(v3(-28.0, 0.0, -4.0), v3(-26.6, 1.9, -2.6), CRATE);
    b.solid(v3(-26.6, 0.0, -4.0), v3(-25.4, 1.0, -2.8), CRATE);
    b.solid(v3(-34.0, 0.0, 6.0), v3(-28.0, 2.6, 8.4), ACCENT);
    b.solid(v3(24.0, 0.0, 12.0), v3(26.4, 2.6, 18.0), [0.20, 0.22, 0.25, 0.0]);
    b.solid(v3(24.0, 2.6, 12.6), v3(26.4, 5.2, 17.4), ACCENT);
    for (cx, cz, sx, sz) in [(-16.0, 10.0, 1.6, 0.4), (0.0, 5.0, 0.4, 1.6), (9.0, 1.0, 1.6, 0.4), (-6.0, 15.0, 0.4, 1.6), (30.0, 22.0, 1.6, 0.4)] {
        b.solid(v3(cx - sx, 0.0, cz - sz), v3(cx + sx, 0.9, cz + sz), CONCRETE_D);
    }
    for (cx, cz) in [(-12.0, 18.0), (6.0, 11.0), (-3.0, -4.5), (14.0, -5.5)] {
        b.solid(v3(cx - 0.8, 0.0, cz - 0.8), v3(cx + 0.8, 1.6, cz + 0.8), CRATE);
        b.solid(v3(cx - 0.6, 1.6, cz - 0.6), v3(cx + 0.6, 2.4, cz + 0.6), CRATE);
    }
    // floodlight masts
    for (lx, lz) in [(-20.0, -4.0), (20.0, 28.0)] {
        b.solid(v3(lx - 0.12, 0.0, lz - 0.12), v3(lx + 0.12, 9.0, lz + 0.12), METAL);
        b.deco(v3(lx - 0.5, 9.0, lz - 0.25), v3(lx + 0.5, 9.4, lz + 0.25), LAMP);
    }

    // ------------------------------------------------ extraction van
    let van = v3(0.0, 0.0, -72.0);
    b.solid(v3(-1.2, 0.5, -74.5), v3(1.2, 2.5, -70.5), [0.90, 0.90, 0.89, 0.0]);
    b.solid(v3(-1.15, 0.5, -70.5), v3(1.15, 1.7, -69.2), [0.90, 0.90, 0.89, 0.0]);
    b.deco(v3(-1.0, 1.4, -69.25), v3(1.0, 1.95, -69.2), METAL);
    for (wx, wz) in [(-1.25, -73.3), (1.05, -73.3), (-1.25, -70.2), (1.05, -70.2)] {
        b.deco(v3(wx, 0.0, wz - 0.5), v3(wx + 0.2, 1.0, wz + 0.5), BLACK);
    }
    b.deco(v3(-1.21, 1.6, -73.0), v3(-1.2, 2.2, -71.5), ACCENT);
    b.deco(v3(1.2, 1.6, -73.0), v3(1.21, 2.2, -71.5), ACCENT);

    // ------------------------------------------------ the rest of the valley
    let extra = crate::village::build_outside(&mut b, &mut spawns, &mut lookouts);

    // ------------------------------------------------ prison guards
    // zone: the yard and its towers
    let zy = Z_YARD;
    spawns.push(sp(Kind::Guard, -16.0, 0.0, 26.0, 0.0, zy, &[(-16.0, 26.0), (-4.0, 27.0), (-6.0, 9.0), (-17.0, 8.0)]));
    spawns.push(sp(Kind::Guard, 22.0, 0.0, 24.0, 0.0, zy, &[(22.0, 24.0), (8.0, 26.0), (6.0, 15.0), (22.0, 8.0)]));
    spawns.push(sp(Kind::Guard, -8.0, 0.0, -3.0, 0.0, zy, &[(-8.0, -3.0), (12.0, -3.0), (12.0, 6.0)]));
    spawns.push(sp(Kind::Guard, 34.0, 0.0, 2.0, 0.0, zy, &[(34.0, 2.0), (34.0, 20.0), (22.0, 20.0)]));
    spawns.push(sp(Kind::Guard, -35.0, 0.0, -10.0, 0.0, zy, &[(-35.0, -10.0), (-36.0, 4.0), (-24.0, 4.0)]));
    spawns.push(sp(Kind::Guard, 0.0, 0.0, -46.0, 0.0, zy, &[(-20.0, -46.0), (20.0, -46.0)]));
    spawns.push(sp(Kind::Guard, 13.0, 0.15, 22.0, pi * 0.5, zy, &[]));
    spawns.push(sp(Kind::Guard, 31.0, 0.15, -23.0, -pi * 0.5, zy, &[]));
    for &(cx, cz, ph, _, _) in &towers {
        let yaw = if cz > 0.0 { pi } else { 0.0 };
        let yaw = if cz.abs() < 10.0 { -pi * 0.5 + 0.5 } else { yaw };
        spawns.push(sp(Kind::Rifleman, cx, ph, cz, yaw, zy, &[]));
    }
    // zone: the cell block
    let zb = Z_BLOCK;
    spawns.push(sp(Kind::Guard, -5.0, 0.0, -14.0, pi, zb, &[(-5.0, -14.0), (5.0, -16.0), (5.0, -28.0), (-5.0, -28.0)]));
    spawns.push(sp(Kind::Guard, 10.0, 0.0, -20.0, pi, zb, &[(10.0, -20.0), (10.0, -32.0), (4.0, -33.0)]));
    spawns.push(sp(Kind::Guard, 0.0, 0.0, -34.0, 0.0, zb, &[(0.0, -34.0), (8.0, -37.0)]));
    spawns.push(sp(Kind::Guard, 0.0, 0.0, -21.5, pi, zb, &[]));
    spawns.push(sp(Kind::Rifleman, -17.5, 4.6, -22.0, 0.0, zb, &[(-17.5, -22.0), (-17.5, -37.0)]));
    spawns.push(sp(Kind::Rifleman, 17.5, 4.6, -36.0, pi, zb, &[(17.5, -36.0), (17.5, -20.0)]));
    spawns.push(sp(Kind::Heavy, -4.0, 0.0, -30.5, pi, zb, &[]));

    let mut zones = extra.zones;
    zones.push(Zone { name: "the yard", packs: vec![v3(-2.5, 0.0, 26.5), v3(2.5, 0.0, 26.5), v3(-38.0, 0.0, 3.0)] });
    zones.push(Zone { name: "the cell block", packs: vec![v3(-4.0, 0.0, -12.0), v3(4.0, 0.0, -12.0)] });
    zones.push(Zone { name: "reinforcements", packs: vec![] });

    let mut ammo_spots = extra.ammo;
    ammo_spots.push((v3(-26.0, 0.15, 12.2), 1));
    ammo_spots.push((v3(-19.0, 0.0, -11.0), 2));
    ammo_spots.push((v3(19.5, 4.6, -12.0), 0));

    let reinforcements = vec![
        (Kind::Guard, v3(-6.0, 0.0, -11.0)),
        (Kind::Guard, v3(6.0, 0.0, -11.0)),
        (Kind::Rifleman, v3(-22.0, 0.0, -46.0)),
        (Kind::Rifleman, v3(22.0, 0.0, -46.0)),
        (Kind::Guard, v3(-9.0, terrain_h(-9.0, -63.0), -63.0)),
        (Kind::Guard, v3(9.0, terrain_h(9.0, -63.0), -63.0)),
    ];

    let Builder { mesh, fence_mesh, shapes, .. } = b;
    Level {
        world: World::new(shapes, doors),
        mesh,
        fence_mesh,
        spawns,
        zones,
        lookouts,
        latches,
        ammo_spots,
        van,
        cell_pos: v3(-10.0, 0.0, -36.5),
        cell_door_pos: v3(-10.0, 0.0, -33.0),
        spawn_pos: extra.start,
        spawn_yaw: extra.start_yaw,
        gate,
        side_gate,
        main_door,
        cell_door,
        blast_rear,
        blast_wall,
        indoor_min: v3(bx0 + 0.1, -0.5, bz0 + 0.1),
        indoor_max: v3(bx1 - 0.1, bh - 0.01, bz1 - 0.1),
        escape: vec![
            v3(-10.0, 0.0, -31.0),
            v3(-4.0, 0.0, -35.0),
            v3(0.0, 0.0, -38.0),
            v3(0.0, 0.0, -45.0),
            v3(0.0, 0.0, -56.0),
            v3(0.0, 0.0, -68.0),
        ],
        reinforcements,
    }
}

/// What the outside-the-wire builder hands back.
pub struct Outside {
    pub zones: Vec<Zone>,
    pub ammo: Vec<(V3, usize)>,
    pub start: V3,
    pub start_yaw: f32,
}
