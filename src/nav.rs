//! Navigation: a 1 m grid of standing spots (several per cell where there are floors above each
//! other), linked wherever a stick-man can actually walk between them — through doorways, up stairs.
//! Bots use A* over it to find their way around the intricate map.

use crate::math::*;
use crate::world::*;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub const NAV_R: f32 = 0.34;
const NAV_H: f32 = 1.7;

const X0: f32 = -126.0;
const Z0: f32 = -96.0;
const CS: f32 = 1.0;
const NX: i32 = 252;
const NZ: i32 = 240;

pub struct Nav {
    cell_start: Vec<u32>, // nodes of cell c are nodes[cell_start[c]..cell_start[c+1]]
    pub nodes: Vec<V3>,
    link_start: Vec<u32>,
    links: Vec<u32>,
    /// for each link: the door it passes through (-1 = none); closed doors cut the link
    link_door: Vec<i8>,
    scratch: RefCell<Scratch>,
}

/// Does the segment a→b (xz) pass through the box (grown by `m`)?
fn seg_hits_box(a: V3, b: V3, bx: &Aabb, m: f32) -> bool {
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (o, d, lo, hi) in [(a.x, b.x - a.x, bx.min.x - m, bx.max.x + m), (a.z, b.z - a.z, bx.min.z - m, bx.max.z + m)] {
        if d.abs() < 1e-6 {
            if o < lo || o > hi {
                return false;
            }
        } else {
            let (mut u0, mut u1) = ((lo - o) / d, (hi - o) / d);
            if u0 > u1 {
                std::mem::swap(&mut u0, &mut u1);
            }
            t0 = t0.max(u0);
            t1 = t1.min(u1);
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

struct Scratch {
    g: Vec<f32>,
    from: Vec<u32>,
    seen: Vec<u32>,
    stamp: u32,
}

#[derive(PartialEq)]
struct Item(f32, u32);
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        // min-heap on cost
        o.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
    }
}

/// Could a bot walk in a straight line from a to b (feet positions)? Mirrors how bots really move:
/// stepping up at most STEP, never dropping more than STEP, never through anything solid.
pub fn walk_ok(w: &World, a: V3, b: V3, r: f32) -> bool {
    let d = (b - a).xz();
    let l = d.len();
    if l < 1e-3 {
        return (a.y - b.y).abs() < 0.3;
    }
    let n = (l / 0.25).ceil().max(1.0) as usize;
    let seg = l / n as f32;
    let mut y = a.y;
    for i in 1..=n {
        let q = a + d * (i as f32 / n as f32);
        let g = w.ground_at(q.x, q.z, r, y, STEP);
        if y - g > STEP + 0.05 || g - y > STEP + 0.05 {
            return false;
        }
        // steep bare hillside
        if g - y > 1.15 * seg + 0.05 && (g - terrain_h(q.x, q.z)).abs() < 0.01 {
            return false;
        }
        if w.blocked(v3(q.x, g, q.z), r, NAV_H) {
            return false;
        }
        y = g;
    }
    (y - b.y).abs() < 0.3
}

fn cell_idx(ix: i32, iz: i32) -> usize {
    (iz * NX + ix) as usize
}

impl Nav {
    /// Build with every door open; links through a door are tagged so a closed door can cut them later.
    pub fn build(w: &World) -> Nav {
        let offs: [(f32, f32); 9] = [
            (0.0, 0.0),
            (0.32, 0.0),
            (-0.32, 0.0),
            (0.0, 0.32),
            (0.0, -0.32),
            (0.3, 0.3),
            (-0.3, 0.3),
            (0.3, -0.3),
            (-0.3, -0.3),
        ];
        let mut cell_start = Vec::with_capacity((NX * NZ + 1) as usize);
        let mut nodes: Vec<V3> = Vec::new();
        let mut levels: Vec<f32> = Vec::new();
        for iz in 0..NZ {
            for ix in 0..NX {
                cell_start.push(nodes.len() as u32);
                let cx = X0 + (ix as f32 + 0.5) * CS;
                let cz = Z0 + (iz as f32 + 0.5) * CS;
                // candidate floor heights: the ground, plus the top of anything walkable here
                levels.clear();
                for &(ox, oz) in &offs {
                    let (x, z) = (cx + ox, cz + oz);
                    levels.push(terrain_h(x, z));
                    w.visit(x, z, 0.0, |s| {
                        if s.flags & WALK != 0 && s.sep(x, z).0 < 0.0 {
                            levels.push(s.top_at(x, z));
                        }
                    });
                }
                levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mut last = f32::MIN;
                let mut uniq: Vec<f32> = vec![];
                for &y in &levels {
                    if y - last > 0.6 {
                        uniq.push(y);
                        last = y;
                    }
                }
                for y in uniq {
                    for &(ox, oz) in &offs {
                        let (x, z) = (cx + ox, cz + oz);
                        let g = w.ground_at(x, z, NAV_R * 0.5, y + 0.5, 0.0);
                        if (g - y).abs() > 0.6 {
                            continue;
                        }
                        let p = v3(x, g, z);
                        if w.blocked(p, NAV_R, NAV_H) {
                            continue;
                        }
                        if (g - terrain_h(x, z)).abs() < 0.01 && terrain_normal(x, z).y < 0.72 {
                            continue;
                        }
                        if !nodes[*cell_start.last().unwrap() as usize..].iter().any(|n: &V3| (n.y - g).abs() < 0.5) {
                            nodes.push(p);
                        }
                        break;
                    }
                }
            }
        }
        cell_start.push(nodes.len() as u32);

        // links
        let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nodes.len()];
        for iz in 0..NZ {
            for ix in 0..NX {
                let c = cell_idx(ix, iz);
                for a in cell_start[c]..cell_start[c + 1] {
                    let pa = nodes[a as usize];
                    for (dx, dz) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                        let (jx, jz) = (ix + dx, iz + dz);
                        if jx < 0 || jz < 0 || jx >= NX || jz >= NZ {
                            continue;
                        }
                        let c2 = cell_idx(jx, jz);
                        for b in cell_start[c2]..cell_start[c2 + 1] {
                            let pb = nodes[b as usize];
                            if (pb.y - pa.y).abs() > 1.3 {
                                continue;
                            }
                            if walk_ok(w, pa, pb, NAV_R) {
                                adj[a as usize].push(b);
                                adj[b as usize].push(a);
                            }
                        }
                    }
                }
            }
        }
        let mut link_start = Vec::with_capacity(nodes.len() + 1);
        let mut links = Vec::new();
        let mut link_door = Vec::new();
        let door_boxes: Vec<Aabb> = w.doors.iter().map(|d| d.b).collect();
        for (i, l) in adj.iter().enumerate() {
            link_start.push(links.len() as u32);
            for &j in l {
                links.push(j);
                let (a, b) = (nodes[i], nodes[j as usize]);
                let mut tag = -1i8;
                for (k, bx) in door_boxes.iter().enumerate() {
                    if a.y < bx.max.y && b.y < bx.max.y && a.y + 1.7 > bx.min.y && seg_hits_box(a, b, bx, NAV_R) {
                        tag = k as i8;
                        break;
                    }
                }
                link_door.push(tag);
            }
        }
        link_start.push(links.len() as u32);
        let n = nodes.len();
        Nav {
            cell_start,
            nodes,
            link_start,
            links,
            link_door,
            scratch: RefCell::new(Scratch { g: vec![0.0; n], from: vec![0; n], seen: vec![0; n], stamp: 0 }),
        }
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Links out of node i as (target, door tag).
    fn neighbours(&self, i: u32) -> impl Iterator<Item = (u32, i8)> + '_ {
        let (a, b) = (self.link_start[i as usize] as usize, self.link_start[i as usize + 1] as usize);
        self.links[a..b].iter().copied().zip(self.link_door[a..b].iter().copied())
    }

    /// The nav spot nearest a feet position (same floor), if any.
    pub fn nearest(&self, p: V3) -> Option<u32> {
        let ix = ((p.x - X0) / CS).floor() as i32;
        let iz = ((p.z - Z0) / CS).floor() as i32;
        let mut best: Option<(f32, u32)> = None;
        for r in 0..3 {
            for jz in iz - r..=iz + r {
                for jx in ix - r..=ix + r {
                    if (jx - ix).abs() != r && (jz - iz).abs() != r {
                        continue;
                    }
                    if jx < 0 || jz < 0 || jx >= NX || jz >= NZ {
                        continue;
                    }
                    let c = cell_idx(jx, jz);
                    for k in self.cell_start[c]..self.cell_start[c + 1] {
                        let q = self.nodes[k as usize];
                        let dy = (q.y - p.y).abs();
                        if dy > 1.4 {
                            continue;
                        }
                        let d = q.dist_xz(p) + dy * 2.0;
                        if best.map(|b| d < b.0).unwrap_or(true) {
                            best = Some((d, k));
                        }
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|b| b.1)
    }

    fn begin(&self) -> std::cell::RefMut<'_, Scratch> {
        let mut s = self.scratch.borrow_mut();
        s.stamp = s.stamp.wrapping_add(1);
        if s.stamp == 0 {
            s.seen.iter_mut().for_each(|v| *v = 0);
            s.stamp = 1;
        }
        s
    }

    /// A* from one feet position to another, not through closed doors. Returns the waypoints (excluding the start), or None.
    pub fn path(&self, from: V3, to: V3, max_expand: usize, doors: &[Door]) -> Option<Vec<V3>> {
        let a = self.nearest(from)?;
        let b = self.nearest(to)?;
        if a == b {
            return Some(vec![to]);
        }
        let goal = self.nodes[b as usize];
        let mut s = self.begin();
        let st = s.stamp;
        let mut heap = BinaryHeap::new();
        s.g[a as usize] = 0.0;
        s.from[a as usize] = a;
        s.seen[a as usize] = st;
        heap.push(Item(self.nodes[a as usize].dist(goal), a));
        let mut expanded = 0;
        let mut found = false;
        while let Some(Item(_, i)) = heap.pop() {
            if i == b {
                found = true;
                break;
            }
            expanded += 1;
            if expanded > max_expand {
                break;
            }
            let gi = s.g[i as usize];
            let pi = self.nodes[i as usize];
            for (j, door) in self.neighbours(i) {
                if door >= 0 && doors.get(door as usize).map(|d| d.solid()).unwrap_or(false) {
                    continue;
                }
                let pj = self.nodes[j as usize];
                let ng = gi + pi.dist(pj) + (pj.y - pi.y).abs() * 0.5;
                let ju = j as usize;
                if s.seen[ju] != st || ng < s.g[ju] - 1e-4 {
                    s.seen[ju] = st;
                    s.g[ju] = ng;
                    s.from[ju] = i;
                    heap.push(Item(ng + pj.dist(goal), j));
                }
            }
        }
        if !found {
            return None;
        }
        let mut out = vec![];
        let mut k = b;
        while k != a {
            out.push(self.nodes[k as usize]);
            k = s.from[k as usize];
        }
        out.reverse();
        // finish at the exact goal if it's on the same floor as the last spot
        if let Some(last) = out.last_mut() {
            if last.dist_xz(to) < 1.2 && (last.y - to.y).abs() < 0.4 {
                *last = to;
            }
        }
        Some(out)
    }

    /// Every spot reachable from `from` within `max_d` metres of walking: (spot, walking distance).
    pub fn reachable(&self, from: V3, max_d: f32, cap: usize, doors: &[Door]) -> Vec<(V3, f32)> {
        let Some(a) = self.nearest(from) else { return vec![] };
        let mut s = self.begin();
        let st = s.stamp;
        let mut heap = BinaryHeap::new();
        s.g[a as usize] = 0.0;
        s.seen[a as usize] = st;
        heap.push(Item(0.0, a));
        let mut out = vec![];
        while let Some(Item(gi, i)) = heap.pop() {
            if gi > s.g[i as usize] + 1e-4 {
                continue;
            }
            out.push((self.nodes[i as usize], gi));
            if out.len() >= cap {
                break;
            }
            let pi = self.nodes[i as usize];
            for (j, door) in self.neighbours(i) {
                if door >= 0 && doors.get(door as usize).map(|d| d.solid()).unwrap_or(false) {
                    continue;
                }
                let pj = self.nodes[j as usize];
                let ng = gi + pi.dist(pj);
                if ng > max_d {
                    continue;
                }
                let ju = j as usize;
                if s.seen[ju] != st || ng < s.g[ju] - 1e-4 {
                    s.seen[ju] = st;
                    s.g[ju] = ng;
                    heap.push(Item(ng, j));
                }
            }
        }
        out
    }
}
