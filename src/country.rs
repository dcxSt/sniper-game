//! The countryside round the prison: the octagonal lookout house on the hill, the farm (barn with a hay
//! loft, farmhouse, water tower, silo) and the woods and scrub that hide you from sight.

use crate::builder::*;
use crate::enemy::Kind;
use crate::level::*;
use crate::math::*;
use crate::village::*;
use crate::world::*;

pub fn build_country(b: &mut Builder, spawns: &mut Vec<Spawn>, lookouts: &mut Vec<Lookout>, ammo: &mut Vec<(V3, usize)>) -> Vec<Zone> {
    let hill_packs = hill(b, spawns, lookouts, ammo);
    let farm_packs = farm(b, spawns, lookouts, ammo);
    forest(b);
    vec![Zone { name: "the hill", packs: hill_packs }, Zone { name: "the farm", packs: farm_packs }]
}

fn hill(b: &mut Builder, spawns: &mut Vec<Spawn>, lookouts: &mut Vec<Lookout>, ammo: &mut Vec<(V3, usize)>) -> Vec<V3> {
    let (cx, cz, r) = (HILL.0, HILL.1, 5.6f32);
    let oct = octagon(cx, cz, r);
    let mut tmax = terrain_h(cx, cz);
    let mut tmin = tmax;
    for p in &oct {
        tmax = tmax.max(terrain_h(p.0, p.1));
        tmin = tmin.min(terrain_h(p.0, p.1));
    }
    let base = tmax + 0.2;
    let fl1 = base + 3.6;
    b.poly(octagon(cx, cz, r + 0.25), tmin - 1.2, base, STONE, SOLID);
    // a couple of steps up to the door if the hill drops away there
    let door_g = terrain_h(cx - r - 1.2, cz);
    if base - door_g > 0.35 {
        b.flight((cx - r - 0.25 - Builder::flight_len(base - door_g), cz), (1.0, 0.0), STAIR_W, door_g, base, STONE, true);
    }
    b.ring(cx, cz, r, 0.35, base, fl1, PLASTER, SOLID, &|k| match k {
        4 => vec![(-0.7, 0.7, base, base + 2.3)],
        _ => vec![(-0.6, 0.6, base + 1.0, base + 2.1)],
    });
    b.wall((cx + 0.5, cz - 1.2), (cx + 0.5, cz + 4.5), 0.2, base, fl1, WALL_IN, &[(2.0, 3.4, base, base + 2.3)]);
    b.flight((cx - 2.6, cz - 2.4), (1.0, 0.0), STAIR_W, base, fl1, WOOD, true);
    let l = Builder::flight_len(fl1 - base);
    let hole = Hole { c: (cx - 2.6 + (1.3 + l) * 0.5, cz - 2.4), u: (1.0, 0.0), hu: (l - 1.3) * 0.5 + 0.05, hv: STAIR_W * 0.5 + 0.06 };
    b.floor(&octagon(cx, cz, r - 0.2), fl1 - 0.25, fl1, FLOOR_IN, Some(hole));
    b.railing((cx - 1.35, cz - 1.34), (cx + 2.6, cz - 1.34), fl1);
    // upstairs: a lookout room with wide windows all round
    b.ring(cx, cz, r, 0.35, fl1, fl1 + 3.0, PLASTER, SOLID, &|_| vec![(-1.1, 1.1, fl1 + 0.95, fl1 + 2.4)]);
    b.poly(octagon(cx, cz, r + 0.3), fl1 + 3.0, fl1 + 3.25, CONCRETE_D, SOLID);
    b.hip(&octagon(cx, cz, r + 0.3), fl1 + 3.25, 2.4, ROOF_R);
    table(b, cx - 3.0, cz + 2.2, base, false);
    b.solid(v3(cx + 3.0, base, cz + 2.6), v3(cx + 4.0, base + 1.0, cz + 3.6), CRATE);
    table(b, cx + 1.5, cz + 2.8, fl1, true);
    // sandbags and a low stone wall round the crown — cover for whoever holds it
    for (k, (x0, z0, x1, z1)) in [(cx - 11.0, cz - 3.0, cx - 10.2, cz + 3.0), (cx - 3.0, cz - 11.0, cx + 3.0, cz - 10.2), (cx + 7.0, cz + 7.0, cx + 9.0, cz + 7.8)].iter().enumerate() {
        let g = terrain_h((x0 + x1) * 0.5, (z0 + z1) * 0.5);
        let _ = k;
        b.solid(v3(*x0, g - 0.3, *z0), v3(*x1, g + 0.95, *z1), SAND);
    }
    lookouts.push(Lookout { pos: v3(cx, fl1, cz), r: 4.6, name: "hill lookout" });
    let yaw_prison = yaw_to(v3(0.0, 0.0, -10.0) - v3(cx, 0.0, cz));
    let zh = Z_HILL;
    spawns.push(sp(Kind::Rifleman, cx - 2.0, fl1, cz + 2.0, yaw_prison, zh, &[]));
    spawns.push(sp(Kind::Guard, cx + 2.6, base, cz + 1.0, std::f32::consts::PI, zh, &[]));
    let ring_r = 9.5;
    let g = |x: f32, z: f32| (x, z);
    spawns.push(sp(
        Kind::Guard,
        cx - ring_r,
        terrain_h(cx - ring_r, cz),
        cz,
        0.0,
        zh,
        &[g(cx - ring_r, cz), g(cx, cz - ring_r), g(cx + ring_r, cz), g(cx, cz + ring_r)],
    ));
    spawns.push(sp(Kind::Guard, 34.0, terrain_h(34.0, 83.0), 83.0, -1.5, zh, &[(34.0, 83.0), (47.0, 81.5), (20.0, 81.5)]));
    ammo.push((v3(cx + 2.6, fl1, cz + 2.4), 0));
    vec![v3(cx - 3.2, base, cz + 0.4), v3(cx + 2.8, base, cz + 3.6)]
}

fn farm(b: &mut Builder, spawns: &mut Vec<Spawn>, lookouts: &mut Vec<Lookout>, ammo: &mut Vec<(V3, usize)>) -> Vec<V3> {
    let pi = std::f32::consts::PI;
    let zf = Z_FARM;
    // ---- barn with a hay loft whose big window looks straight at the prison
    let (x0, z0, x1, z1) = (76.0f32, -26.0f32, 92.0f32, -10.0f32);
    let y = 0.12;
    let wh = 6.0;
    b.solid(v3(x0, -0.3, z0), v3(x1, y, z1), CONCRETE_D);
    b.wall((x0, z0 + 0.15), (x1, z0 + 0.15), 0.3, y, wh, BARN, &[(3.0, 4.2, y + 1.4, y + 2.4), (11.8, 13.0, y + 1.4, y + 2.4)]);
    b.wall((x0, z1 - 0.15), (x1, z1 - 0.15), 0.3, y, wh, BARN, &[(6.0, 10.0, y, y + 4.0)]);
    b.wall((x0 + 0.15, z0 + 0.3), (x0 + 0.15, z1 - 0.3), 0.3, y, wh, BARN, &[(9.7, 13.3, y, y + 3.0), (2.2, 5.2, 3.9, 5.4)]);
    b.wall((x1 - 0.15, z0 + 0.3), (x1 - 0.15, z1 - 0.3), 0.3, y, wh, BARN, &[(5.7, 9.3, y, y + 3.0)]);
    let loft = 3.4;
    b.floor(&rect(x0 + 0.3, z0 + 0.3, x1 - 0.3, -18.0), loft - 0.25, loft, WOOD, None);
    b.flight((84.0, -18.0 + Builder::flight_len(loft - y)), (0.0, -1.0), STAIR_W, y, loft, WOOD, true);
    b.wall((x0 + 0.3, -18.05), (82.9, -18.05), 0.1, loft, loft + 1.0, WOOD_D, &[]);
    b.wall((85.1, -18.05), (x1 - 0.3, -18.05), 0.1, loft, loft + 1.0, WOOD_D, &[]);
    b.solid(v3(x0, wh, z0), v3(x1, wh + 0.12, z1), WOOD_D);
    b.gable(x0, z0, x1, z1, wh + 0.12, 3.6, BARN);
    for (hx, hz, hy) in [(78.0, -12.0, y), (79.6, -12.0, y), (78.8, -12.0, y + 1.4), (90.0, -24.0, loft), (88.4, -24.2, loft), (89.0, -12.5, y)] {
        b.poly(octagon(hx, hz, 0.72), hy, hy + 1.35, [0.83, 0.74, 0.52, 0.0], SOLID);
    }
    for sz in [-24.0f32, -21.0] {
        b.solid(v3(x1 - 3.4, y, sz - 0.08), v3(x1 - 0.3, y + 1.3, sz + 0.08), WOOD_D);
    }
    lookouts.push(Lookout { pos: v3(78.5, loft, -22.0), r: 2.2, name: "barn loft" });
    spawns.push(sp(Kind::Guard, 77.6, loft, -22.0, -pi * 0.5, zf, &[]));
    ammo.push((v3(90.4, loft, -21.0), 0));

    // ---- farmhouse
    let h = House {
        storeys: 2,
        doors: vec![(2, 6.5), (1, 63.0)],
        parts: vec![((61.0, 4.6), (61.0, 10.7), vec![(2.8, 4.2)])],
        stair: Some(((66.7, 3.4), (-1.0, 0.0))),
        roof: Roof::Hip,
        col: PLASTER_W,
        ..House::new(58.0, 2.0, 68.0, 11.0)
    };
    let (yh, yh2) = build_house(b, &h);
    table(b, 59.8, 9.0, yh, false);
    b.solid(v3(65.0, yh2, 8.6), v3(67.4, yh2 + 0.5, 10.6), WOOD);
    spawns.push(sp(Kind::Guard, 64.0, yh, 8.0, -pi * 0.5, zf, &[]));
    ammo.push((v3(66.5, yh2, 5.6), 3));

    // ---- water tower: an octagonal tank on a deck, with a switchback up
    let deck = b.lookout(100.0, 6.0, 3.6, terrain_h(100.0, 6.0), 10.0, 4, -1.0, false);
    b.poly(octagon(100.0, 6.0, 2.0), deck.y, deck.y + 3.4, [0.80, 0.80, 0.78, 0.0], SOLID);
    b.hip(&octagon(100.0, 6.0, 2.1), deck.y + 3.4, 1.0, ROOF_R);
    lookouts.push(Lookout { pos: deck, r: 3.4, name: "water tower" });
    let yaw_w = yaw_to(v3(0.0, 0.0, -10.0) - v3(97.4, 0.0, 6.0));
    spawns.push(sp(Kind::Rifleman, 97.4, deck.y, 6.0, yaw_w, zf, &[]));

    // ---- silo, tractor, bales, fences
    b.poly(octagon(98.0, -34.0, 2.4), -0.3, 13.0, [0.82, 0.82, 0.80, 0.0], SOLID);
    b.hip(&octagon(98.0, -34.0, 2.5), 13.0, 2.0, ROOF_R);
    b.solid(v3(68.5, 0.5, -3.2), v3(71.5, 1.8, -1.6), [0.62, 0.30, 0.24, 0.0]);
    b.solid(v3(70.2, 1.8, -3.0), v3(71.4, 2.8, -1.8), METAL);
    for (wx, wz, r) in [(68.9, -3.4, 0.5), (68.9, -1.4, 0.5), (71.2, -3.5, 0.7), (71.2, -1.3, 0.7)] {
        b.poly(octagon(wx, wz, r), 0.0, 0.25, BLACK, 0);
        b.mesh.cylinder(v3(wx, 0.0, wz), r, 0.1, BLACK, 10);
    }
    for (hx, hz) in [(70.0, -16.0), (71.5, -15.2), (72.0, -32.0), (104.0, -20.0), (106.0, -19.0), (62.0, -28.0), (88.0, 14.0), (89.4, 14.6)] {
        let g = terrain_h(hx, hz);
        b.poly(octagon(hx, hz, 0.72), g - 0.1, g + 1.35, [0.83, 0.74, 0.52, 0.0], SOLID);
    }
    b.wood_fence((56.0, 22.0), (72.0, 22.0));
    b.wood_fence((80.0, 22.0), (116.0, 22.0));
    b.wood_fence((116.0, 22.0), (116.0, -46.0));
    b.wood_fence((116.0, -46.0), (56.0, -46.0));
    b.wood_fence((56.0, -46.0), (56.0, -12.0));
    for (x, z) in [(60.0, -6.0), (108.0, 16.0), (64.0, 18.0), (110.0, -42.0)] {
        b.round_tree(x, z);
    }
    for (x, z) in [(74.0, -28.0), (95.0, -8.0), (57.5, 0.0), (102.0, -44.0)] {
        b.bush(x, z, 1.2);
    }
    spawns.push(sp(Kind::Guard, 72.0, 0.0, 15.0, 0.0, zf, &[(72.0, 15.0), (104.0, 16.0), (106.0, -6.0), (72.0, -6.0)]));
    spawns.push(sp(Kind::Guard, 60.0, 0.0, -24.0, 0.0, zf, &[(60.0, -24.0), (66.0, -42.0), (110.0, -40.0), (104.0, -26.0)]));
    vec![v3(80.0, y, -14.0), v3(87.5, y, -14.5)]
}

pub const START: (f32, f32) = (-30.0, 136.0);

fn forest(b: &mut Builder) {
    let mut placed = 0;
    let mut tries = 0;
    while placed < 300 && tries < 14000 {
        tries += 1;
        let x = b.rng.range(-188.0, 188.0);
        let z = b.rng.range(-188.0, 188.0);
        if (x * x + z * z).sqrt() > 192.0 {
            continue;
        }
        if in_compound(x, z, 11.0) || in_village(x, z, 2.0) || in_farm(x, z, 3.0) {
            continue;
        }
        if road_dist(x, z) < 2.5 || hill_mask(x, z) > 0.78 {
            continue;
        }
        if (x - START.0).hypot(z - START.1) < 6.0 {
            continue;
        }
        let edge = z > 114.0 || x < -88.0 || x > 120.0 || z < -92.0;
        let mut p = if edge { 0.9 } else { 0.3 };
        if z > 31.0 && z < 46.0 && x > -40.0 && x < 48.0 {
            p = 0.06; // open ground in front of the fence
        }
        if hill_mask(x, z) > 0.15 {
            p = 0.5;
        }
        if !b.rng.chance(p) {
            continue;
        }
        if terrain_normal(x, z).y < 0.82 {
            continue;
        }
        b.tree(x, z);
        placed += 1;
    }
    let mut placed = 0;
    let mut tries = 0;
    while placed < 150 && tries < 8000 {
        tries += 1;
        let x = b.rng.range(-150.0, 150.0);
        let z = b.rng.range(-110.0, 160.0);
        if in_compound(x, z, 6.0) || in_village(x, z, 0.0) || in_farm(x, z, 0.0) || road_dist(x, z) < 1.0 || hill_mask(x, z) > 0.8 {
            continue;
        }
        let r = b.rng.range(0.8, 1.4);
        b.bush(x, z, r);
        placed += 1;
    }
}
