//! Outside the wire: the village on the southern approach (little houses with rooms, a church and its
//! octagonal bell tower), plus the hand-off to the hill, the farm and the woods (see `country.rs`).

use crate::builder::*;
use crate::enemy::Kind;
use crate::gfx::Col;
use crate::level::*;
use crate::math::*;
use crate::world::*;

pub const STOREY: f32 = 3.2;

pub enum Roof {
    Flat,
    Gable,
    Hip,
}

/// A small house: one or two storeys, doors, ground-floor partitions, an inside stair (two storeys) or
/// an outside stair onto a flat-roof terrace.
pub struct House {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
    pub storeys: u8,
    pub doors: Vec<(u8, f32)>,
    /// partition centre lines with doorways (start, end along the line)
    pub parts: Vec<((f32, f32), (f32, f32), Vec<(f32, f32)>)>,
    /// inside flight (start, direction) for two storeys
    pub stair: Option<((f32, f32), (f32, f32))>,
    /// outside flight (start, direction) up to a flat roof, and the parapet gap (side, from, to)
    pub terrace: Option<((f32, f32), (f32, f32), (u8, f32, f32))>,
    pub roof: Roof,
    pub col: Col,
}

impl House {
    pub fn new(x0: f32, z0: f32, x1: f32, z1: f32) -> House {
        House { x0, z0, x1, z1, storeys: 1, doors: vec![], parts: vec![], stair: None, terrace: None, roof: Roof::Flat, col: PLASTER }
    }
}

/// Builds a house; returns (ground floor height, top of the roof slab / upper floor height).
pub fn build_house(b: &mut Builder, h: &House) -> (f32, f32) {
    let pts = [(h.x0, h.z0), (h.x1, h.z0), (h.x1, h.z1), (h.x0, h.z1), ((h.x0 + h.x1) * 0.5, (h.z0 + h.z1) * 0.5)];
    let tmax = pts.iter().map(|p| terrain_h(p.0, p.1)).fold(f32::MIN, f32::max);
    let tmin = pts.iter().map(|p| terrain_h(p.0, p.1)).fold(f32::MAX, f32::min);
    let y0 = tmax + 0.15;
    b.solid(v3(h.x0, tmin - 0.4, h.z0), v3(h.x1, y0, h.z1), STONE);
    shell(b, h.x0, h.z0, h.x1, h.z1, y0, STOREY, h.col, &h.doors, true);
    for (a, bb, gaps) in &h.parts {
        let ops: Vec<Opening> = gaps.iter().map(|g| (g.0, g.1, y0, y0 + 2.3)).collect();
        b.wall(*a, *bb, 0.2, y0, y0 + STOREY, WALL_IN, &ops);
    }
    let mut top = y0 + STOREY;
    if h.storeys >= 2 {
        let (p0, dir) = h.stair.unwrap_or(((h.x0 + 1.5, h.z0 + 1.4), (1.0, 0.0)));
        let l = Builder::flight_len(STOREY);
        let hole = Hole { c: add2(p0, mul2(dir, (1.3 + l) * 0.5)), u: dir, hu: (l - 1.3) * 0.5 + 0.05, hv: STAIR_W * 0.5 + 0.06 };
        b.floor(&rect(h.x0 + 0.05, h.z0 + 0.05, h.x1 - 0.05, h.z1 - 0.05), top - 0.25, top, FLOOR_IN, Some(hole));
        b.flight(p0, dir, STAIR_W, y0, top, WOOD, true);
        // a rail along the open edge of the hole upstairs
        let v = mul2(perp2(dir), STAIR_W * 0.5 + 0.06);
        let (ra, rb) = (add2(add2(p0, mul2(dir, 1.3)), v), add2(add2(p0, mul2(dir, l)), v));
        let inside = |p: (f32, f32)| p.0 > h.x0 + 0.4 && p.0 < h.x1 - 0.4 && p.1 > h.z0 + 0.4 && p.1 < h.z1 - 0.4;
        let (ra, rb) = if inside(ra) && inside(rb) { (ra, rb) } else { (sub2(ra, mul2(v, 2.0)), sub2(rb, mul2(v, 2.0))) };
        b.railing(ra, rb, top);
        let up_doors: Vec<(u8, f32)> = vec![];
        shell(b, h.x0, h.z0, h.x1, h.z1, top, STOREY, h.col, &up_doors, true);
        top += STOREY;
    }
    match h.roof {
        Roof::Flat => {
            b.solid(v3(h.x0, top, h.z0), v3(h.x1, top + 0.25, h.z1), CONCRETE_D);
            parapet(b, h.x0, h.z0, h.x1, h.z1, top + 0.25, h.terrace.map(|t| t.2));
            if let Some((p0, dir, _)) = h.terrace {
                let g = terrain_h(p0.0, p0.1);
                b.flight(p0, dir, STAIR_W, g, top + 0.25, CONCRETE_D, true);
            }
            top += 0.25;
        }
        Roof::Gable => {
            b.solid(v3(h.x0, top - 0.2, h.z0), v3(h.x1, top, h.z1), WALL_IN);
            b.gable(h.x0, h.z0, h.x1, h.z1, top, 2.4, TILE);
        }
        Roof::Hip => {
            b.solid(v3(h.x0, top - 0.2, h.z0), v3(h.x1, top, h.z1), WALL_IN);
            b.hip(&rect(h.x0 - 0.35, h.z0 - 0.35, h.x1 + 0.35, h.z1 + 0.35), top, 2.6, TILE);
        }
    }
    (y0, top)
}

pub fn table(b: &mut Builder, x: f32, z: f32, y: f32, along_x: bool) {
    let (hx, hz) = if along_x { (0.9, 0.5) } else { (0.5, 0.9) };
    b.solid(v3(x - hx, y, z - hz), v3(x + hx, y + 0.78, z + hz), WOOD);
}

pub fn build_outside(b: &mut Builder, spawns: &mut Vec<Spawn>, lookouts: &mut Vec<Lookout>) -> Outside {
    let mut ammo: Vec<(V3, usize)> = vec![];
    village(b, spawns, lookouts, &mut ammo);
    let zones_hill_farm = crate::country::build_country(b, spawns, lookouts, &mut ammo);
    let mut zones = vec![Zone { name: "the village", packs: vec![v3(-46.0, 0.0, 75.5), v3(-43.5, 0.0, 80.5), v3(-31.0, 0.15, 49.0)] }];
    zones.extend(zones_hill_farm);
    let start = v3(-30.0, terrain_h(-30.0, 136.0), 136.0);
    ammo.push((v3(-28.5, terrain_h(-28.5, 133.5), 133.5), 0));
    Outside { zones, ammo, start, start_yaw: 0.0 }
}

fn village(b: &mut Builder, spawns: &mut Vec<Spawn>, lookouts: &mut Vec<Lookout>, ammo: &mut Vec<(V3, usize)>) {
    let pi = std::f32::consts::PI;
    let zv = Z_VILLAGE;

    // ---------------------------------------------------------------- church + bell tower
    {
        let h = House {
            doors: vec![(0, 55.5), (1, -66.0)],
            roof: Roof::Gable,
            col: PLASTER_W,
            ..House::new(-78.0, 50.0, -60.0, 61.0)
        };
        // one tall storey: build the shell by hand so it can be taller than a house
        let y0 = 0.15;
        b.solid(v3(h.x0, -0.4, h.z0), v3(h.x1, y0, h.z1), STONE);
        shell(b, h.x0, h.z0, h.x1, h.z1, y0, 6.4, h.col, &h.doors, true);
        b.wall((-74.0, 50.3), (-74.0, 60.7), 0.2, y0, 6.4, WALL_IN, &[(4.5, 5.9, y0, y0 + 2.3)]);
        b.solid(v3(h.x0, 6.35, h.z0), v3(h.x1, 6.55, h.z1), WALL_IN);
        b.gable(h.x0, h.z0, h.x1, h.z1, 6.55, 3.4, TILE);
        for row in 0..5 {
            let x = -71.5 + row as f32 * 2.0;
            for (z0, z1) in [(51.2, 54.6), (56.4, 59.8)] {
                b.solid(v3(x - 0.25, y0, z0), v3(x + 0.25, y0 + 0.55, z1), WOOD_D);
            }
        }
        b.solid(v3(-73.4, y0, 54.2), v3(-72.4, y0 + 1.1, 56.8), STONE);
        b.solid(v3(-77.4, y0, 51.0), v3(-75.0, y0 + 0.9, 52.0), WOOD);
        // churchyard: low stone wall and a few headstones
        b.wall((-82.0, 62.5), (-82.0, 70.5), 0.4, -0.2, 1.0, STONE, &[]);
        b.wall((-82.0, 70.5), (-62.0, 70.5), 0.4, -0.2, 1.0, STONE, &[(8.0, 10.0, -0.2, 2.0)]);
        b.wall((-82.0, 62.5), (-78.0, 62.5), 0.4, -0.2, 1.0, STONE, &[]);
        for i in 0..8 {
            let (x, z) = (-79.0 + (i % 4) as f32 * 3.4, 64.5 + (i / 4) as f32 * 3.0);
            b.solid(v3(x - 0.35, 0.0, z - 0.1), v3(x + 0.35, 0.9, z + 0.1), STONE);
        }

        // bell tower: an octagon with a four-flight stair folded up inside it, open belfry on top
        let (cx, cz, r) = (-55.5f32, 47.5f32, 5.0f32);
        let fl = y0;
        b.poly(octagon(cx, cz, r), -0.4, fl, STONE, SOLID);
        let rpf = 2.6;
        let flights = 4;
        let top = fl + rpf * flights as f32;
        b.ring(cx, cz, r, 0.4, fl, top, STONE, SOLID, &|k| {
            let mut o = vec![];
            if k == 0 {
                o.push((-0.7, 0.7, fl, fl + 2.4));
            } else if k % 2 == 1 {
                o.push((-0.25, 0.25, fl + 3.4, fl + 4.4));
                o.push((-0.25, 0.25, fl + 7.4, fl + 8.4));
            }
            o
        });
        let (lu, lv) = Builder::zigzag_size(rpf, flights);
        let u = (1.0, 0.0);
        let v = (0.0, 1.0);
        let origin = (cx - lu * 0.5, cz - lv * 0.5);
        let (exit, ytop, _) = b.zigzag(origin, u, v, fl, rpf, flights, STONE, false);
        let _ = ytop;
        // belfry floor with the hole over the last flight
        let l = Builder::flight_len(rpf);
        let hole = Hole { c: (origin.0 + LANDING + l * 0.5, origin.1 + lv - STAIR_W * 0.5), u, hu: l * 0.5 + 0.1, hv: STAIR_W * 0.5 + 0.08 };
        b.floor(&octagon(cx, cz, r - 0.3), top - 0.3, top, STONE, Some(hole));
        b.railing((hole.c.0 - hole.hu, hole.c.1 - hole.hv), (hole.c.0 + hole.hu - 0.1, hole.c.1 - hole.hv), top);
        b.railing((hole.c.0 - hole.hu, hole.c.1 + hole.hv), (hole.c.0 + hole.hu - 0.1, hole.c.1 + hole.hv), top);
        let _ = exit;
        // open arches on every side
        b.ring(cx, cz, r, 0.4, top, top + 3.0, STONE, SOLID, &|_| vec![(-1.15, 1.15, top + 1.0, top + 2.6)]);
        b.poly(octagon(cx, cz, r + 0.3), top + 3.0, top + 3.25, STONE, SOLID);
        b.hip(&octagon(cx, cz, r + 0.3), top + 3.25, 3.6, TILE);
        b.mesh.cylinder(v3(cx, top + 2.0, cz + 2.4), 0.5, 0.75, [0.55, 0.45, 0.25, 0.0], 12);
        lookouts.push(Lookout { pos: v3(cx, top, cz), r: 4.0, name: "bell tower" });
        spawns.push(sp(Kind::Rifleman, cx - 1.0, top, cz - 3.0, pi * 0.25, zv, &[]));
        ammo.push((v3(cx + 2.4, top, cz - 1.0), 0));
    }

    // ---------------------------------------------------------------- houses
    // north-east block
    let ha = House {
        storeys: 2,
        doors: vec![(2, 49.5)],
        parts: vec![((-32.0, 47.6), (-32.0, 53.7), vec![(3.4, 4.8)])],
        stair: Some(((-35.6, 46.4), (1.0, 0.0))),
        roof: Roof::Gable,
        ..House::new(-37.0, 45.0, -28.0, 54.0)
    };
    let (ya, ya2) = build_house(b, &ha);
    table(b, -34.5, 51.5, ya, true);
    b.solid(v3(-30.9, ya2, 51.4), v3(-28.5, ya2 + 0.5, 53.5), WOOD);
    spawns.push(sp(Kind::Guard, -33.0, ya2, 51.0, -pi * 0.5, zv, &[]));
    ammo.push((v3(-29.5, ya, 46.5), 1));

    let hb = House {
        doors: vec![(2, 50.0)],
        parts: vec![((-19.0, 46.3), (-19.0, 53.7), vec![(4.6, 6.0)])],
        terrace: Some(((-16.0, 54.0 + Builder::flight_len(3.6)), (0.0, -1.0), (1, -17.05, -14.95))),
        ..House::new(-24.0, 46.0, -14.0, 54.0)
    };
    let (yb0, yb) = build_house(b, &hb);
    table(b, -21.5, 48.0, yb0, false);
    spawns.push(sp(Kind::Rifleman, -20.0, yb, 49.0, pi * 0.85, zv, &[]));

    let hc = House {
        doors: vec![(2, 64.0), (1, -30.0)],
        parts: vec![((-32.0, 60.3), (-32.0, 67.7), vec![(1.2, 2.6)]), ((-31.9, 64.0), (-27.3, 64.0), vec![(1.5, 2.9)])],
        roof: Roof::Hip,
        col: PLASTER_W,
        ..House::new(-37.0, 60.0, -27.0, 68.0)
    };
    let (yc, _) = build_house(b, &hc);
    table(b, -35.0, 66.0, yc, true);
    b.solid(v3(-29.6, yc, 60.4), v3(-27.4, yc + 0.6, 62.5), WOOD);

    // a timber shed
    b.solid(v3(-22.0, -0.3, 62.0), v3(-16.0, 0.12, 67.0), WOOD_D);
    shell(b, -22.0, 62.0, -16.0, 67.0, 0.12, 2.6, WOOD_D, &[(1, -19.0)], false);
    b.gable(-22.0, 62.0, -16.0, 67.0, 2.72, 1.5, ROOF_R);
    b.solid(v3(-21.6, 0.12, 62.4), v3(-20.2, 1.2, 64.0), CRATE);

    // south-west block
    let hd = House {
        storeys: 2,
        doors: vec![(0, 92.5)],
        parts: vec![((-61.0, 88.3), (-61.0, 94.3), vec![(0.6, 2.0)])],
        stair: Some(((-57.0, 95.6), (-1.0, 0.0))),
        roof: Roof::Gable,
        col: PLASTER_W,
        ..House::new(-66.0, 88.0, -56.0, 97.0)
    };
    let (yd, yd2) = build_house(b, &hd);
    table(b, -63.5, 90.5, yd, false);
    spawns.push(sp(Kind::Guard, -63.0, yd2, 91.0, pi * 0.5, zv, &[]));
    ammo.push((v3(-64.8, yd2, 89.0), 0));

    let he = House {
        doors: vec![(3, -74.0), (0, 103.5)],
        parts: vec![((-76.0, 100.3), (-76.0, 106.7), vec![(2.6, 4.0)]), ((-71.5, 100.3), (-71.5, 106.7), vec![(3.8, 5.2)])],
        roof: Roof::Hip,
        ..House::new(-80.0, 100.0, -68.0, 107.0)
    };
    let (ye, _) = build_house(b, &he);
    table(b, -78.0, 104.5, ye, false);
    table(b, -69.8, 105.6, ye, true);

    let hf = House {
        doors: vec![(0, 106.0)],
        terrace: Some(((-59.5, 103.0 - Builder::flight_len(3.6)), (0.0, 1.0), (3, -60.55, -58.45))),
        ..House::new(-63.0, 103.0, -56.0, 109.0)
    };
    build_house(b, &hf);

    // south-east block
    let hg = House {
        doors: vec![(2, 91.5)],
        parts: vec![((-32.5, 88.3), (-32.5, 94.7), vec![(2.4, 3.8)])],
        terrace: Some(((-30.0, 88.0 - Builder::flight_len(3.6)), (0.0, 1.0), (3, -31.05, -28.95))),
        ..House::new(-37.0, 88.0, -28.0, 95.0)
    };
    let (yg0, yg) = build_house(b, &hg);
    table(b, -35.0, 93.0, yg0, true);
    spawns.push(sp(Kind::Guard, -33.5, yg, 92.0, -pi * 0.3, zv, &[]));

    let hh = House {
        storeys: 2,
        doors: vec![(3, -19.5)],
        parts: vec![((-23.7, 94.0), (-17.6, 94.0), vec![(1.4, 2.8)])],
        stair: Some(((-16.4, 97.3), (0.0, -1.0))),
        roof: Roof::Flat,
        col: PLASTER_W,
        ..House::new(-24.0, 89.0, -15.0, 98.0)
    };
    let (yh, _) = build_house(b, &hh);
    table(b, -21.5, 91.5, yh, true);

    let hi = House { doors: vec![(1, -30.5), (2, 106.0)], roof: Roof::Gable, col: WOOD, ..House::new(-36.0, 102.0, -25.0, 110.0) };
    let (yi, _) = build_house(b, &hi);
    for (x, z) in [(-34.5, 103.5), (-33.0, 103.5), (-27.0, 108.5)] {
        b.solid(v3(x - 0.6, yi, z - 0.6), v3(x + 0.6, yi + 1.2, z + 0.6), CRATE);
    }

    // ---------------------------------------------------------------- the square
    {
        let (wx, wz) = (-46.0, 78.0);
        b.ring(wx, wz, 1.2, 0.3, -0.2, 0.85, STONE, SOLID, &|_| vec![]);
        for k in [1usize, 5] {
            let p = octagon(wx, wz, 1.05)[k];
            b.solid(v3(p.0 - 0.08, 0.85, p.1 - 0.08), v3(p.0 + 0.08, 2.5, p.1 + 0.08), WOOD_D);
        }
        b.hip(&octagon(wx, wz, 1.5), 2.5, 0.9, TILE);
        // market stalls
        for (sx, sz) in [(-52.0, 72.5), (-40.0, 72.5), (-52.0, 84.0)] {
            b.solid(v3(sx - 1.4, 0.0, sz - 0.5), v3(sx + 1.4, 0.9, sz + 0.5), WOOD);
            for dx in [-1.3f32, 1.3] {
                b.deco(v3(sx + dx - 0.05, 0.9, sz - 0.05), v3(sx + dx + 0.05, 2.3, sz + 0.05), WOOD_D);
            }
            b.mesh.slab(v3(sx - 1.6, 2.3, sz - 0.9), v3(sx + 1.6, 2.3, sz - 0.9), v3(sx + 1.6, 2.0, sz + 0.9), v3(sx - 1.6, 2.0, sz + 0.9), ACCENT);
        }
        // a cart and some barrels
        b.solid(v3(-38.5, 0.3, 83.2), v3(-35.5, 1.0, 84.8), WOOD_D);
        for (x, z) in [(-38.8, 74.0), (-38.0, 75.0), (-55.0, 79.0)] {
            b.poly(octagon(x, z, 0.4), 0.0, 1.0, WOOD_D, SOLID);
        }
        // benches
        b.solid(v3(-49.5, 0.0, 75.0), v3(-48.8, 0.45, 77.0), WOOD);
        b.solid(v3(-43.2, 0.0, 79.0), v3(-42.5, 0.45, 81.0), WOOD);
    }

    // ---------------------------------------------------------------- hedges, walls, trees, lamps
    b.hedge((-82.0, 84.0), (-82.0, 112.0), 1.7);
    b.hedge((-82.0, 42.0), (-82.0, 49.0), 1.7);
    b.hedge((-38.5, 56.5), (-38.5, 59.0), 1.5);
    b.hedge((-12.0, 46.0), (-12.0, 70.0), 1.6);
    b.hedge((-12.0, 86.0), (-12.0, 110.0), 1.6);
    b.hedge((-26.0, 85.0), (-26.0, 88.0), 1.4);
    b.wall((-53.0, 85.5), (-53.0, 100.0), 0.4, -0.2, 1.1, STONE, &[(6.0, 8.0, -0.2, 2.0)]);
    b.wall((-39.0, 85.5), (-39.0, 99.0), 0.4, -0.2, 1.1, STONE, &[(5.0, 7.0, -0.2, 2.0)]);
    b.wall((-53.0, 42.5), (-53.0, 44.0), 0.4, -0.2, 1.1, STONE, &[]);
    for (x, z) in [
        (-70.0, 94.0),
        (-74.0, 89.0),
        (-60.0, 110.0),
        (-17.0, 104.0),
        (-30.0, 99.0),
        (-24.0, 58.5),
        (-14.5, 72.0),
        (-62.0, 66.0),
        (-30.0, 72.5),
        (-68.0, 44.5),
        (-77.0, 46.0),
    ] {
        b.round_tree(x, z);
    }
    for (x, z) in [(-57.0, 99.0), (-40.5, 66.0), (-25.5, 71.0), (-58.0, 72.0), (-13.5, 80.0), (-80.0, 80.0), (-36.0, 99.5)] {
        b.bush(x, z, 1.1);
    }
    for (x, z) in [(-50.6, 60.0), (-41.4, 96.0), (-50.6, 104.0), (-41.4, 50.0), (-34.0, 82.6), (-62.0, 73.4)] {
        b.solid(v3(x - 0.08, 0.0, z - 0.08), v3(x + 0.08, 4.2, z + 0.08), METAL);
        b.deco(v3(x - 0.25, 4.2, z - 0.25), v3(x + 0.25, 4.45, z + 0.25), LAMP);
    }

    // ---------------------------------------------------------------- village guards
    spawns.push(sp(Kind::Guard, -46.0, 0.0, 104.0, 0.0, zv, &[(-46.0, 104.0), (-46.0, 86.0), (-30.0, 80.0), (-46.0, 86.0)]));
    spawns.push(sp(Kind::Guard, -46.0, 0.0, 48.0, pi, zv, &[(-46.0, 48.0), (-46.0, 70.0), (-66.0, 78.0), (-46.0, 70.0)]));
    spawns.push(sp(Kind::Guard, -70.0, 0.0, 78.0, 0.0, zv, &[(-75.0, 78.0), (-55.0, 78.0), (-56.0, 98.0), (-70.0, 98.0)]));
    spawns.push(sp(Kind::Guard, -45.0, 0.0, 74.5, pi * 0.5, zv, &[]));
    spawns.push(sp(Kind::Guard, -68.0, 0.15, 55.5, pi * 0.5, zv, &[]));
}
