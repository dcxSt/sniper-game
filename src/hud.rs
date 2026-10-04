//! Minimal HUD: thin ink on the pale world, soft halos so it still reads on dark areas,
//! one accent colour. No panels, no boxes.

use crate::enemy::*;
use crate::math::*;
use crate::weapons::*;
use crate::*;
use web_sys::CanvasRenderingContext2d;

const FONT: &str = "'Helvetica Neue', Helvetica, Arial, sans-serif";

#[derive(Clone, Copy)]
struct Ink {
    fg: &'static str,
    halo: &'static str,
}
/// charcoal ink with a pale halo — for the bright world
const DARK: Ink = Ink { fg: "#1b1e22", halo: "rgba(255,255,255,0.75)" };
/// pale ink with a dark halo — for the black scope surround
const LIGHT: Ink = Ink { fg: "#f4f4f1", halo: "rgba(0,0,0,0.5)" };
const RED: &str = "#d4301f";
const AMBER: &str = "#c58a00";
const ORANGE: &str = "#dc6a14";
const GREEN: &str = "#2f9e5a";

fn rgba(r: f32, g: f32, b: f32, a: f32) -> String {
    format!("rgba({},{},{},{:.3})", (r * 255.0) as i32, (g * 255.0) as i32, (b * 255.0) as i32, a.clamp(0.0, 1.0))
}

/// Drawing helper: knows the current ink and the screen scale.
struct Pen<'a> {
    c: &'a CanvasRenderingContext2d,
    ink: Ink,
    u: f64,
}

impl Pen<'_> {
    #[allow(clippy::too_many_arguments)]
    fn text_col(&self, s: &str, x: f64, y: f64, size: f64, weight: u32, col: &str, align: &str) {
        let c = self.c;
        let px = (size * self.u).round().max(8.0);
        c.set_font(&format!("{} {}px {}", weight, px as i32, FONT));
        c.set_text_align(align);
        c.set_line_join("round");
        c.set_line_width((px * 0.24).max(2.0));
        c.set_stroke_style_str(self.ink.halo);
        let _ = c.stroke_text(s, x, y);
        c.set_fill_style_str(col);
        let _ = c.fill_text(s, x, y);
    }

    fn text(&self, s: &str, x: f64, y: f64, size: f64, weight: u32, align: &str) {
        self.text_col(s, x, y, size, weight, self.ink.fg, align);
    }

    fn line(&self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let c = self.c;
        c.begin_path();
        c.move_to(x0, y0);
        c.line_to(x1, y1);
        c.stroke();
    }

    fn circle(&self, x: f64, y: f64, r: f64) {
        self.c.begin_path();
        let _ = self.c.arc(x, y, r, 0.0, 6.2832);
    }

    /// Stroke the current path with a pale halo underneath, so thin lines survive any background.
    fn halo_stroke(&self, col: &str, width: f64) {
        let c = self.c;
        c.set_line_cap("round");
        c.set_line_join("round");
        c.set_stroke_style_str(self.ink.halo);
        c.set_line_width(width + 2.4 * self.u);
        c.stroke();
        c.set_stroke_style_str(col);
        c.set_line_width(width);
        c.stroke();
    }
}

impl Game {
    pub(crate) fn draw_hud(&self) {
        let c = &self.hudc;
        let (w, h) = (self.width as f64, self.height as f64);
        c.clear_rect(0.0, 0.0, w, h);
        if self.state == MENU {
            return;
        }
        let u = (h / 1080.0).max(0.66) * 1.2;
        let scope_a = self.scope_amount() as f64;
        let ink = if scope_a > 0.5 { LIGHT } else { DARK };
        let pen = Pen { c, ink, u };
        let (cx, cy) = (w / 2.0, h / 2.0);
        let p = &self.player;
        let t = self.anim_t as f64;
        let (yaw, _) = self.view_angles();
        let eye = self.eye();

        // ---- awareness glyphs above each bot
        for e in &self.enemies {
            // only for people you can actually see — no peeking through walls
            if !e.alive() || !e.in_view {
                continue;
            }
            let head = e.pos + v3(0.0, 2.2 * e.kind.scale(), 0.0);
            let Some((nx, ny, wc)) = project(&self.vp, head) else { continue };
            if wc > 240.0 || nx.abs() > 1.1 || ny.abs() > 1.1 {
                continue;
            }
            let sx = (nx as f64 * 0.5 + 0.5) * w;
            let sy = (1.0 - (ny as f64 * 0.5 + 0.5)) * h;
            let k = (30.0 / wc as f64).clamp(0.5, 1.0);
            if e.state == State::Combat {
                pen.text_col("!", sx, sy - 4.0 * u * k, 24.0 * k, 700, RED, "center");
            } else if e.state == State::Search {
                pen.text_col("?", sx, sy - 4.0 * u * k, 23.0 * k, 700, ORANGE, "center");
            } else if e.state == State::Investigate || e.aware > 0.2 {
                pen.text_col("?", sx, sy - 4.0 * u * k, 21.0 * k, 600, AMBER, "center");
            }
            if e.aware > 0.05 && e.state != State::Combat {
                c.begin_path();
                let _ = c.arc(sx, sy + 7.0 * u * k, 8.0 * u * k, -1.5708, -1.5708 + 6.2832 * (e.aware.min(1.0) as f64));
                pen.halo_stroke(AMBER, 1.8 * u * k);
            }
            if e.hp_show > 0.0 {
                let bw = 38.0 * u * k;
                let f = (e.hp / e.kind.hp()).clamp(0.0, 1.0) as f64;
                c.set_fill_style_str("rgba(255,255,255,0.7)");
                c.fill_rect(sx - bw / 2.0 - 1.0, sy + 17.0 * u * k, bw + 2.0, 4.0 * u * k + 2.0);
                c.set_fill_style_str("rgba(28,31,36,0.35)");
                c.fill_rect(sx - bw / 2.0, sy + 18.0 * u * k, bw, 4.0 * u * k);
                c.set_fill_style_str(if f > 0.35 { "#1b1e22" } else { RED });
                c.fill_rect(sx - bw / 2.0, sy + 18.0 * u * k, bw * f, 4.0 * u * k);
            }
        }

        // ---- scope reticle (the black surround itself is drawn on the GPU)
        if scope_a > 0.02 {
            c.set_global_alpha(scope_a);
            let r = h * 0.47;
            let z = if p.zoom == 1 { 0.6 } else { 1.0 };
            c.set_stroke_style_str("rgba(255,255,255,0.88)");
            c.set_line_width(1.3 * u);
            let gap = 15.0 * u;
            pen.line(cx - r, cy, cx - gap, cy);
            pen.line(cx + gap, cy, cx + r, cy);
            pen.line(cx, cy + gap, cx, cy + r);
            pen.line(cx, cy - r, cx, cy - gap);
            // range ticks
            for i in 1..=9 {
                let d = i as f64 * 30.0 * u * z;
                let l = if i % 2 == 0 { 7.0 * u } else { 4.0 * u };
                pen.line(cx - d, cy - l, cx - d, cy + l);
                pen.line(cx + d, cy - l, cx + d, cy + l);
                pen.line(cx - l, cy + d, cx + l, cy + d);
            }
            c.set_fill_style_str("rgba(255,80,60,0.95)");
            pen.circle(cx, cy, 1.8 * u);
            c.fill();
            let lens = Pen { c, ink: LIGHT, u };
            lens.text(if p.zoom == 1 { "10×" } else { "5×" }, cx - r * 0.66, cy + r * 0.8, 13.0, 300, "left");
            // breath
            let bw = 150.0 * u;
            let by = cy + r * 0.86;
            c.set_stroke_style_str("rgba(255,255,255,0.22)");
            c.set_line_width(2.0 * u);
            pen.line(cx - bw / 2.0, by, cx + bw / 2.0, by);
            c.set_stroke_style_str(if p.breath_lock { "rgba(255,90,70,0.95)" } else { "rgba(255,255,255,0.9)" });
            pen.line(cx - bw / 2.0, by, cx - bw / 2.0 + bw * p.breath as f64, by);
            lens.text("SHIFT · BREATH", cx, by + 17.0 * u, 9.5, 500, "center");
            c.set_global_alpha(1.0);
        }

        // ---- crosshair
        if scope_a < 0.5 && self.state != DEAD {
            let def = &WEAPONS[p.cur];
            let sp = (if p.ads_t > 0.5 { def.ads_spread } else { def.spread })
                + p.speed * 0.25
                + if p.grounded { 0.0 } else { 2.5 };
            let gap = (5.0 + sp as f64 * 4.5) * u;
            let len = 7.0 * u;
            c.begin_path();
            for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                c.move_to(cx + dx * gap, cy + dy * gap);
                c.line_to(cx + dx * (gap + len), cy + dy * (gap + len));
            }
            pen.halo_stroke(ink.fg, 1.6 * u);
            pen.circle(cx, cy, 1.4 * u);
            c.set_fill_style_str(ink.fg);
            c.fill();
        }

        // ---- hit marker
        if self.hit_marker.0 > 0.0 {
            let a = (self.hit_marker.0 / 0.25) as f64;
            let col = match self.hit_marker.1 {
                2 => RED,
                1 => ORANGE,
                _ => ink.fg,
            };
            c.set_global_alpha(a);
            c.begin_path();
            let (a0, a1) = (7.0 * u, 16.0 * u);
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                c.move_to(cx + sx * a0, cy + sy * a0);
                c.line_to(cx + sx * a1, cy + sy * a1);
            }
            pen.halo_stroke(col, 2.2 * u);
            c.set_global_alpha(1.0);
        }

        // ---- being hurt / healed
        {
            let low = if p.hp < 35.0 { (1.0 - p.hp / 35.0) * (0.6 + 0.4 * (t * 5.0).sin() as f32) * 0.45 } else { 0.0 };
            let a = (p.dmg_flash * 0.5).max(low);
            if a > 0.01 {
                if let Ok(g) = c.create_radial_gradient(cx, cy, h * 0.32, cx, cy, h * 0.95) {
                    let _ = g.add_color_stop(0.0, "rgba(200,30,20,0)");
                    let _ = g.add_color_stop(1.0, &rgba(0.78, 0.1, 0.06, a * 1.3));
                    c.set_fill_style_canvas_gradient(&g);
                    c.fill_rect(0.0, 0.0, w, h);
                }
            }
            if p.heal_flash > 0.01 {
                if let Ok(g) = c.create_radial_gradient(cx, cy, h * 0.38, cx, cy, h * 0.95) {
                    let _ = g.add_color_stop(0.0, "rgba(60,200,120,0)");
                    let _ = g.add_color_stop(1.0, &rgba(0.2, 0.8, 0.45, p.heal_flash * 0.35));
                    c.set_fill_style_canvas_gradient(&g);
                    c.fill_rect(0.0, 0.0, w, h);
                }
            }
            if self.state == DEAD {
                let k = (self.death_t / 1.2).min(1.0);
                c.set_fill_style_str(&rgba(0.1, 0.0, 0.0, 0.5 * k));
                c.fill_rect(0.0, 0.0, w, h);
            }
        }
        for (src, life) in &self.indicators {
            let rel = angle_diff(yaw, yaw_to(*src - eye));
            let a = (life / 2.0).min(1.0);
            let ang = rel as f64 - 1.5708;
            c.set_global_alpha(a as f64 * 0.9);
            c.begin_path();
            let _ = c.arc(cx, cy, 150.0 * u, ang - 0.26, ang + 0.26);
            pen.halo_stroke(RED, 5.0 * u);
            c.set_global_alpha(1.0);
        }

        self.draw_radar(&pen, yaw);
        if self.state == PLAYING || self.state == PAUSED {
            self.draw_stealth(&pen);
        }

        // ---- health: a number and a hairline
        {
            let x = 40.0 * u;
            let y = h - 46.0 * u;
            let f = (p.hp / 100.0).clamp(0.0, 1.0) as f64;
            let col = if f > 0.3 { ink.fg } else { RED };
            pen.text_col(&format!("{}", p.hp.max(0.0).ceil() as i32), x, y - 14.0 * u, 34.0, 200, col, "left");
            pen.text("HEALTH", x + 74.0 * u, y - 16.0 * u, 9.5, 600, "left");
            let bw = 190.0 * u;
            c.set_fill_style_str("rgba(255,255,255,0.55)");
            c.fill_rect(x - 1.0, y - 1.5 * u, bw + 2.0, 4.0 * u + 1.0);
            c.set_fill_style_str("rgba(28,31,36,0.22)");
            c.fill_rect(x, y - 0.5 * u, bw, 2.0 * u);
            c.set_fill_style_str(col);
            c.fill_rect(x, y - 0.5 * u, bw * f, 2.0 * u);
            // elevation: the high ground matters
            let alt = p.pos.y;
            let hi = p.pos.y - crate::world::terrain_h(p.pos.x, p.pos.z) > 4.0 || alt > 14.0;
            pen.text_col(
                &format!("{:+.0} m{}", alt, if hi { "  ·  high ground" } else { "" }),
                x,
                y + 20.0 * u,
                10.5,
                500,
                if hi { ORANGE } else { ink.fg },
                "left",
            );
        }

        // ---- ammunition
        {
            let def = &WEAPONS[p.cur];
            let ws = p.ws[p.cur];
            let x = w - 40.0 * u;
            let y = h - 46.0 * u;
            let low = ws.mag <= def.mag / 4;
            let reserve = format!("{}", ws.reserve);
            pen.text_col(&reserve, x, y - 14.0 * u, 20.0, 300, ink.fg, "right");
            pen.text_col(
                &format!("{}", ws.mag),
                x - (text_width(&reserve, 20.0, u) + 14.0 * u),
                y - 14.0 * u,
                50.0,
                200,
                if low { RED } else { ink.fg },
                "right",
            );
            pen.text(def.name, x, y + 20.0 * u, 10.0, 600, "right");
            if p.reload_t > 0.0 {
                let f = 1.0 - p.reload_t / def.reload;
                let bw = 190.0 * u;
                c.set_fill_style_str("rgba(28,31,36,0.22)");
                c.fill_rect(x - bw, y - 0.5 * u, bw, 2.0 * u);
                c.set_fill_style_str(ORANGE);
                c.fill_rect(x - bw, y - 0.5 * u, bw * f as f64, 2.0 * u);
            } else if ws.mag == 0 {
                pen.text_col(if ws.reserve > 0 { "press R to reload" } else { "out of ammo" }, cx, h * 0.72, 15.0, 500, RED, "center");
            }
            // weapon slots, tucked above the numbers
            for i in 0..4 {
                let sx = x - (3 - i) as f64 * 24.0 * u;
                let act = i == p.cur;
                pen.text_col(
                    &format!("{}", i + 1),
                    sx,
                    y - 68.0 * u,
                    11.5,
                    if act { 700 } else { 400 },
                    if act { ink.fg } else { "rgba(27,30,34,0.4)" },
                    "right",
                );
                if act {
                    c.set_fill_style_str(ORANGE);
                    c.fill_rect(sx - 8.0 * u, y - 63.0 * u, 8.0 * u, 2.0 * u);
                }
            }
        }

        // ---- objective
        pen.text(&self.objective().to_lowercase(), cx, 40.0 * u, 14.0, 500, "center");

        // ---- banner
        if self.banner.t > 0.0 {
            let a = (self.banner.t.min(1.0) * (4.5 - self.banner.t).min(0.5) * 2.0).clamp(0.0, 1.0);
            c.set_global_alpha(a as f64);
            pen.text(&self.banner.title.to_lowercase(), cx, h * 0.21, 32.0, 300, "center");
            pen.text(&self.banner.sub, cx, h * 0.21 + 30.0 * u, 13.5, 400, "center");
            c.set_global_alpha(1.0);
        }

        // ---- kill / pickup feedback
        if self.tag.1 > 0.0 {
            let a = (self.tag.1 / 0.6).min(1.0) as f64;
            c.set_global_alpha(a);
            let col = if self.tag.0.starts_with("HEADSHOT") {
                ORANGE
            } else if self.tag.0.starts_with('+') {
                GREEN
            } else {
                ink.fg
            };
            pen.text_col(&self.tag.0.to_lowercase(), cx, h * 0.62 + (1.0 - a) * 10.0, 16.0, 600, col, "center");
            c.set_global_alpha(1.0);
        }
        if !self.prompt.is_empty() {
            pen.text(&self.prompt.to_lowercase(), cx, h * 0.7, 15.0, 500, "center");
        }
        if self.state == PLAYING || self.state == PAUSED {
            pen.text_col("esc · controls", cx, h - 16.0 * u, 10.0, 400, "rgba(27,30,34,0.45)", "center");
        }
    }

    fn draw_radar(&self, pen: &Pen, yaw: f32) {
        let (c, u) = (pen.c, pen.u);
        let r = 74.0 * u;
        let (cx, cy) = (self.width as f64 - r - 34.0 * u, r + 34.0 * u);
        let range = 80.0f32;
        let scale = r / range as f64;
        let p = &self.player;
        // lens
        pen.circle(cx, cy, r);
        c.set_fill_style_str("rgba(255,255,255,0.30)");
        c.fill();
        c.save();
        pen.circle(cx, cy, r);
        c.clip();
        c.set_stroke_style_str("rgba(27,30,34,0.16)");
        c.set_line_width(1.0);
        pen.circle(cx, cy, r * 0.5);
        c.stroke();
        let a = 0.6f64;
        c.set_stroke_style_str("rgba(27,30,34,0.28)");
        pen.line(cx, cy, cx + (-1.5708f64 - a).cos() * r, cy + (-1.5708f64 - a).sin() * r);
        pen.line(cx, cy, cx + (-1.5708f64 + a).cos() * r, cy + (-1.5708f64 + a).sin() * r);

        let fwd = fwd_from(yaw);
        let right = right_from(yaw);
        let to_radar = |wp: V3| -> (f64, f64) {
            let d = (wp - p.pos).xz();
            (cx + d.dot(right) as f64 * scale, cy - d.dot(fwd) as f64 * scale)
        };
        for pk in &self.pickups {
            let (x, y) = to_radar(pk.pos);
            match pk.kind {
                PKind::Health => {
                    c.set_fill_style_str(GREEN);
                    c.fill_rect(x - 3.5 * u, y - 1.0 * u, 7.0 * u, 2.0 * u);
                    c.fill_rect(x - 1.0 * u, y - 3.5 * u, 2.0 * u, 7.0 * u);
                }
                PKind::Ammo(_) => {
                    c.set_fill_style_str("rgba(27,30,34,0.45)");
                    c.fill_rect(x - 1.8 * u, y - 1.8 * u, 3.6 * u, 3.6 * u);
                }
            }
        }
        for e in &self.enemies {
            if !e.alive() || e.spotted <= 0.0 {
                continue;
            }
            c.set_global_alpha((e.spotted as f64 / 2.0).min(1.0));
            let (x, y) = to_radar(e.pos);
            let dy = e.pos.y - p.pos.y;
            let hot = e.state == State::Combat;
            c.set_fill_style_str(if hot { RED } else { "#1b1e22" });
            c.begin_path();
            if dy > 3.0 {
                c.move_to(x, y - 4.5 * u);
                c.line_to(x - 3.8 * u, y + 3.0 * u);
                c.line_to(x + 3.8 * u, y + 3.0 * u);
                c.close_path();
            } else if dy < -3.0 {
                c.move_to(x, y + 4.5 * u);
                c.line_to(x - 3.8 * u, y - 3.0 * u);
                c.line_to(x + 3.8 * u, y - 3.0 * u);
                c.close_path();
            } else {
                let _ = c.arc(x, y, 3.0 * u, 0.0, 6.2832);
            }
            c.fill();
            if hot {
                c.set_stroke_style_str("rgba(212,48,31,0.5)");
                c.set_line_width(1.2);
                pen.circle(x, y, (5.5 + (self.anim_t as f64 * 6.0).sin().abs() * 3.0) * u);
                c.stroke();
            }
        }
        c.set_global_alpha(1.0);
        if let Some(op) = self.objective_pos() {
            let (mut x, mut y) = to_radar(op);
            let (dx, dy) = (x - cx, y - cy);
            let dl = (dx * dx + dy * dy).sqrt();
            if dl > r - 8.0 * u {
                x = cx + dx / dl * (r - 8.0 * u);
                y = cy + dy / dl * (r - 8.0 * u);
            }
            c.set_fill_style_str(ORANGE);
            c.begin_path();
            c.move_to(x, y - 5.0 * u);
            c.line_to(x + 4.2 * u, y);
            c.line_to(x, y + 5.0 * u);
            c.line_to(x - 4.2 * u, y);
            c.close_path();
            c.fill();
        }
        c.restore();
        pen.circle(cx, cy, r);
        c.set_stroke_style_str("rgba(27,30,34,0.55)");
        c.set_line_width(1.3 * u);
        c.stroke();
        // you
        c.set_fill_style_str("#ffffff");
        c.set_stroke_style_str("#1b1e22");
        c.set_line_width(1.2 * u);
        c.begin_path();
        c.move_to(cx, cy - 5.5 * u);
        c.line_to(cx - 4.0 * u, cy + 4.0 * u);
        c.line_to(cx + 4.0 * u, cy + 4.0 * u);
        c.close_path();
        c.fill();
        c.stroke();
        let nx = cx + (-yaw.sin()) as f64 * (r + 9.0 * u);
        let ny = cy - yaw.cos() as f64 * (r + 9.0 * u);
        pen.text("N", nx, ny + 3.5 * u, 9.5, 600, "center");
        let alive = self.enemies.iter().filter(|e| e.alive()).count();
        pen.text(&format!("{} hostile", alive), cx, cy + r + 22.0 * u, 10.0, 500, "center");
    }

    /// Top-left: the detection eye, and how visible you currently are.
    fn draw_stealth(&self, pen: &Pen) {
        let (c, u) = (pen.c, pen.u);
        let t = self.anim_t as f64;
        let st = self.stealth;
        let (ox, oy) = (40.0 * u, 36.0 * u);
        let tone = match st {
            0 => "#1b1e22",
            1 => AMBER,
            2 => ORANGE,
            _ => RED,
        };
        let (cr, cg, cb): (f32, f32, f32) = match st {
            0 => (0.11, 0.12, 0.14),
            1 => (0.77, 0.54, 0.0),
            2 => (0.86, 0.42, 0.08),
            _ => (0.83, 0.19, 0.12),
        };
        let col = |a: f32| rgba(cr, cg, cb, a);

        // ---- the eye
        let (cx, cy) = (ox + 26.0 * u, oy + 16.0 * u);
        let (w, h) = (24.0 * u, 14.0 * u);
        let open = (self.eye_open as f64).clamp(0.08, 1.0);
        if self.eye_pulse > 0.0 {
            let k = self.eye_pulse as f64;
            pen.circle(cx, cy, w + 5.0 * u + (1.0 - k) * 20.0 * u);
            pen.halo_stroke(&col(k as f32 * 0.8), 1.6 * u);
        }
        eye_path(c, cx, cy, w, h, open);
        c.set_fill_style_str(if open > 0.3 { "rgba(255,255,255,0.94)" } else { "rgba(255,255,255,0.45)" });
        c.fill();
        if open > 0.3 {
            c.save();
            eye_path(c, cx, cy, w, h, open);
            c.clip();
            let look = match st {
                2 => (t * 2.3).sin() * w * 0.42,
                1 => (t * 1.2).sin() * w * 0.2,
                _ => 0.0,
            };
            let ir = (h * open * 1.12).min(w * 0.42);
            pen.circle(cx + look, cy, ir);
            c.set_fill_style_str(tone);
            c.fill();
            pen.circle(cx + look, cy, ir * if st == 3 { 0.36 } else { 0.5 });
            c.set_fill_style_str("#0b0c0e");
            c.fill();
            c.restore();
        }
        eye_path(c, cx, cy, w, h, open);
        pen.halo_stroke(tone, 1.8 * u);
        if open < 0.35 {
            c.begin_path();
            for k in 0..5 {
                let f = k as f64 / 4.0;
                let x = cx - w * 0.62 + f * w * 1.24;
                let ly = cy + h * open * (1.0 - ((x - cx) / w).powi(2));
                c.move_to(x, ly);
                c.line_to(x + (x - cx) * 0.1, ly + 4.5 * u);
            }
            pen.halo_stroke(tone, 1.4 * u);
        }
        if st == 3 {
            let k = 0.5 + 0.5 * (t * 9.0).sin();
            c.begin_path();
            for i in 0..12 {
                let a = i as f64 * 0.5236;
                let (s, co) = a.sin_cos();
                let r0 = w + 6.0 * u;
                let r1 = r0 + (3.0 + 4.0 * k) * u;
                c.move_to(cx + co * r0, cy + s * r0 * 0.55);
                c.line_to(cx + co * r1, cy + s * r1 * 0.55);
            }
            pen.halo_stroke(&col(0.5 + 0.45 * k as f32), 1.5 * u);
        } else if st == 1 || st == 2 {
            let f = if st == 1 { (self.eye_aware as f64).max(0.08) } else { (self.eye_search as f64).clamp(0.05, 1.0) };
            c.begin_path();
            let _ = c.arc(cx, cy, w + 6.0 * u, -1.5708, -1.5708 + 6.2832 * f);
            pen.halo_stroke(&col(0.95), 1.8 * u);
        }

        // ---- words
        let (label, sub) = match st {
            0 => ("NOT DETECTED", "hidden"),
            1 => ("SUSPICIOUS", "they sense something"),
            2 => ("SEARCHING", "lost you — stay down"),
            _ => ("DETECTED", "break line of sight"),
        };
        let tx = ox + 62.0 * u;
        pen.text_col(label, tx, oy + 14.0 * u, 13.0, 700, tone, "left");
        pen.text(sub, tx, oy + 30.0 * u, 10.5, 400, "left");

        // ---- stance and visibility, quietly
        let crouched = self.player.crouch > 0.5;
        let vis = self.player.vis;
        let (lvl, vname) = if vis <= 0.7 {
            (1, "low")
        } else if vis <= 1.05 {
            (2, "medium")
        } else {
            (3, "high")
        };
        let sy = oy + 62.0 * u;
        pen.text(if crouched { "CROUCHED" } else { "STANDING" }, ox, sy, 10.5, 600, "left");
        for i in 0..3 {
            let x = ox + 84.0 * u + i as f64 * 9.0 * u;
            pen.circle(x, sy - 3.5 * u, 2.6 * u);
            c.set_fill_style_str(if i < lvl { "#1b1e22" } else { "rgba(27,30,34,0.2)" });
            c.fill();
        }
        pen.text(&format!("{} visibility", vname), ox + 116.0 * u, sy, 10.5, 400, "left");
    }
}

/// rough text width, to line the ammo numbers up
fn text_width(s: &str, size: f64, u: f64) -> f64 {
    s.chars().count() as f64 * size * u * 0.56
}

/// Almond-shaped eye outline; `o` is how far the lids are open (0 = shut, 1 = wide).
fn eye_path(c: &CanvasRenderingContext2d, cx: f64, cy: f64, w: f64, h: f64, o: f64) {
    c.begin_path();
    c.move_to(cx - w, cy);
    c.quadratic_curve_to(cx, cy - 2.0 * h * o, cx + w, cy);
    c.quadratic_curve_to(cx, cy + 2.0 * h * o, cx - w, cy);
    c.close_path();
}
