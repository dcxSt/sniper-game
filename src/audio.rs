use wasm_bindgen::JsValue;
use web_sys::{AudioBuffer, AudioContext, BiquadFilterType, OscillatorType};

#[derive(Clone, Copy, PartialEq)]
pub enum Snd {
    Sniper,
    Rifle,
    Pistol,
    Shotgun,
    EnemyShot,
    EnemyHeavy,
    Hit,
    Headshot,
    Kill,
    Hurt,
    Reload,
    Bolt,
    Empty,
    Pickup,
    Heal,
    Step,
    Land,
    Explosion,
    Alarm,
    Door,
    Zone,
    Switch,
    Alert,
    Clear,
    Latch,
}

pub struct Audio {
    ctx: Option<AudioContext>,
    noise: Option<AudioBuffer>,
}

impl Audio {
    pub fn new() -> Audio {
        Audio { ctx: None, noise: None }
    }

    /// Must be called from a user gesture.
    pub fn init(&mut self) {
        if self.ctx.is_some() {
            if let Some(c) = &self.ctx {
                let _ = c.resume();
            }
            return;
        }
        let ctx = match AudioContext::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let sr = ctx.sample_rate();
        if let Ok(buf) = ctx.create_buffer(1, sr as u32 * 2, sr) {
            let mut data = vec![0.0f32; (sr as usize) * 2];
            let mut s: u32 = 0x1234_5678;
            for d in data.iter_mut() {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                *d = (s as f32 / u32::MAX as f32) * 2.0 - 1.0;
            }
            let _ = buf.copy_to_channel(&mut data, 0);
            self.noise = Some(buf);
        }
        self.ctx = Some(ctx);
        let _ = self.wind();
    }

    fn wind(&self) -> Result<(), JsValue> {
        let (Some(ctx), Some(nb)) = (&self.ctx, &self.noise) else { return Ok(()) };
        let src = ctx.create_buffer_source()?;
        src.set_buffer(Some(nb));
        src.set_loop(true);
        let f = ctx.create_biquad_filter()?;
        f.set_type(BiquadFilterType::Lowpass);
        f.frequency().set_value(380.0);
        let g = ctx.create_gain()?;
        g.gain().set_value(0.05);
        // slow gusts
        let lfo = ctx.create_oscillator()?;
        lfo.frequency().set_value(0.11);
        let lg = ctx.create_gain()?;
        lg.gain().set_value(140.0);
        lfo.connect_with_audio_node(&lg)?;
        lg.connect_with_audio_param(&f.frequency())?;
        lfo.start()?;
        src.connect_with_audio_node(&f)?;
        f.connect_with_audio_node(&g)?;
        g.connect_with_audio_node(&ctx.destination())?;
        src.start()?;
        Ok(())
    }

    #[allow(deprecated)]
    fn noise(&self, when: f64, dur: f64, ft: BiquadFilterType, f0: f32, f1: f32, gain: f32, pan: f32) -> Result<(), JsValue> {
        let (Some(ctx), Some(nb)) = (&self.ctx, &self.noise) else { return Ok(()) };
        let t = ctx.current_time() + when;
        let src = ctx.create_buffer_source()?;
        src.set_buffer(Some(nb));
        let f = ctx.create_biquad_filter()?;
        f.set_type(ft);
        f.frequency().set_value_at_time(f0, t)?;
        f.frequency().exponential_ramp_to_value_at_time(f1.max(20.0), t + dur)?;
        let g = ctx.create_gain()?;
        g.gain().set_value_at_time(gain.max(0.0002), t)?;
        g.gain().exponential_ramp_to_value_at_time(0.0005, t + dur)?;
        let p = ctx.create_stereo_panner()?;
        p.pan().set_value(pan.clamp(-1.0, 1.0));
        src.connect_with_audio_node(&f)?;
        f.connect_with_audio_node(&g)?;
        g.connect_with_audio_node(&p)?;
        p.connect_with_audio_node(&ctx.destination())?;
        src.start_with_when(t)?;
        src.stop_with_when(t + dur + 0.05)?;
        Ok(())
    }

    #[allow(deprecated)]
    fn tone(&self, when: f64, dur: f64, w: OscillatorType, f0: f32, f1: f32, gain: f32, pan: f32) -> Result<(), JsValue> {
        let Some(ctx) = &self.ctx else { return Ok(()) };
        let t = ctx.current_time() + when;
        let o = ctx.create_oscillator()?;
        o.set_type(w);
        o.frequency().set_value_at_time(f0, t)?;
        o.frequency().exponential_ramp_to_value_at_time(f1.max(10.0), t + dur)?;
        let g = ctx.create_gain()?;
        g.gain().set_value_at_time(gain.max(0.0002), t)?;
        g.gain().exponential_ramp_to_value_at_time(0.0005, t + dur)?;
        let p = ctx.create_stereo_panner()?;
        p.pan().set_value(pan.clamp(-1.0, 1.0));
        o.connect_with_audio_node(&g)?;
        g.connect_with_audio_node(&p)?;
        p.connect_with_audio_node(&ctx.destination())?;
        o.start_with_when(t)?;
        o.stop_with_when(t + dur + 0.05)?;
        Ok(())
    }

    pub fn play(&self, s: Snd, vol: f32, pan: f32) {
        if self.ctx.is_none() || vol < 0.01 {
            return;
        }
        let _ = self.play_inner(s, vol, pan);
    }

    fn play_inner(&self, s: Snd, v: f32, pan: f32) -> Result<(), JsValue> {
        use BiquadFilterType::*;
        use OscillatorType as W;
        match s {
            Snd::Sniper => {
                self.noise(0.0, 0.9, Lowpass, 4000.0, 160.0, 1.0 * v, pan)?;
                self.tone(0.0, 0.5, W::Sine, 130.0, 35.0, 0.9 * v, pan)?;
                self.noise(0.28, 1.2, Lowpass, 900.0, 80.0, 0.22 * v, pan * 0.5)?;
            }
            Snd::Rifle => {
                self.noise(0.0, 0.2, Bandpass, 2600.0, 380.0, 0.75 * v, pan)?;
                self.tone(0.0, 0.12, W::Sawtooth, 170.0, 55.0, 0.35 * v, pan)?;
            }
            Snd::Pistol => {
                self.noise(0.0, 0.09, Highpass, 1800.0, 900.0, 0.22 * v, pan)?;
                self.tone(0.0, 0.06, W::Triangle, 700.0, 220.0, 0.12 * v, pan)?;
            }
            Snd::Shotgun => {
                self.noise(0.0, 0.55, Lowpass, 3000.0, 140.0, 1.0 * v, pan)?;
                self.tone(0.0, 0.35, W::Sine, 95.0, 32.0, 0.9 * v, pan)?;
            }
            Snd::EnemyShot => {
                self.noise(0.0, 0.22, Bandpass, 2200.0, 350.0, 0.6 * v, pan)?;
                self.tone(0.0, 0.1, W::Sawtooth, 150.0, 50.0, 0.25 * v, pan)?;
            }
            Snd::EnemyHeavy => {
                self.noise(0.0, 0.5, Lowpass, 2600.0, 130.0, 0.9 * v, pan)?;
                self.tone(0.0, 0.3, W::Sine, 85.0, 30.0, 0.7 * v, pan)?;
            }
            Snd::Hit => {
                self.tone(0.0, 0.06, W::Square, 1300.0, 800.0, 0.10 * v, 0.0)?;
            }
            Snd::Headshot => {
                self.tone(0.0, 0.12, W::Square, 2000.0, 1300.0, 0.14 * v, 0.0)?;
                self.tone(0.04, 0.12, W::Triangle, 1000.0, 600.0, 0.14 * v, 0.0)?;
            }
            Snd::Kill => {
                self.tone(0.0, 0.18, W::Triangle, 520.0, 240.0, 0.20 * v, 0.0)?;
            }
            Snd::Hurt => {
                self.noise(0.0, 0.2, Lowpass, 500.0, 120.0, 0.5 * v, pan)?;
                self.tone(0.0, 0.16, W::Sine, 110.0, 50.0, 0.4 * v, pan)?;
            }
            Snd::Reload => {
                self.noise(0.0, 0.05, Bandpass, 1800.0, 1200.0, 0.3 * v, 0.0)?;
                self.noise(0.5, 0.05, Bandpass, 900.0, 600.0, 0.35 * v, 0.0)?;
                self.noise(1.0, 0.07, Bandpass, 2200.0, 1500.0, 0.4 * v, 0.0)?;
            }
            Snd::Bolt => {
                self.noise(0.25, 0.05, Bandpass, 1500.0, 900.0, 0.35 * v, 0.0)?;
                self.noise(0.5, 0.06, Bandpass, 2000.0, 1100.0, 0.4 * v, 0.0)?;
            }
            Snd::Empty => {
                self.noise(0.0, 0.03, Bandpass, 2500.0, 2000.0, 0.3 * v, 0.0)?;
            }
            Snd::Pickup => {
                self.tone(0.0, 0.14, W::Sine, 600.0, 1100.0, 0.2 * v, 0.0)?;
            }
            Snd::Heal => {
                self.tone(0.0, 0.2, W::Sine, 523.0, 523.0, 0.18 * v, 0.0)?;
                self.tone(0.1, 0.2, W::Sine, 659.0, 659.0, 0.18 * v, 0.0)?;
                self.tone(0.2, 0.35, W::Sine, 784.0, 784.0, 0.18 * v, 0.0)?;
            }
            Snd::Step => {
                self.noise(0.0, 0.08, Lowpass, 600.0, 200.0, 0.14 * v, pan)?;
            }
            Snd::Land => {
                self.noise(0.0, 0.14, Lowpass, 500.0, 120.0, 0.35 * v, 0.0)?;
            }
            Snd::Explosion => {
                self.noise(0.0, 1.8, Lowpass, 1600.0, 50.0, 1.2 * v, pan)?;
                self.tone(0.0, 1.3, W::Sine, 75.0, 22.0, 1.0 * v, pan)?;
            }
            Snd::Alarm => {
                for i in 0..8 {
                    let f = if i % 2 == 0 { 780.0 } else { 560.0 };
                    self.tone(i as f64 * 0.32, 0.3, W::Square, f, f, 0.07 * v, 0.0)?;
                }
            }
            Snd::Door => {
                self.noise(0.0, 0.9, Lowpass, 320.0, 160.0, 0.5 * v, pan)?;
                self.tone(0.0, 0.9, W::Sawtooth, 55.0, 80.0, 0.18 * v, pan)?;
            }
            Snd::Zone => {
                self.tone(0.0, 0.2, W::Triangle, 440.0, 440.0, 0.2 * v, 0.0)?;
                self.tone(0.12, 0.2, W::Triangle, 554.0, 554.0, 0.2 * v, 0.0)?;
                self.tone(0.24, 0.4, W::Triangle, 659.0, 659.0, 0.2 * v, 0.0)?;
            }
            Snd::Alert => {
                self.tone(0.0, 0.14, W::Square, 880.0, 880.0, 0.11 * v, 0.0)?;
                self.tone(0.12, 0.22, W::Square, 1320.0, 1100.0, 0.11 * v, 0.0)?;
            }
            Snd::Clear => {
                self.tone(0.0, 0.35, W::Sine, 520.0, 330.0, 0.12 * v, 0.0)?;
            }
            Snd::Latch => {
                // a sharp metallic clang, then the chain rattling loose
                self.tone(0.0, 0.5, W::Triangle, 1850.0, 1650.0, 0.22 * v, pan)?;
                self.tone(0.0, 0.7, W::Sine, 2630.0, 2500.0, 0.12 * v, pan)?;
                self.noise(0.0, 0.08, Highpass, 4000.0, 2500.0, 0.4 * v, pan)?;
                for i in 0..5 {
                    self.noise(0.12 + i as f64 * 0.07, 0.05, Bandpass, 3200.0, 2400.0, 0.12 * v, pan)?;
                }
            }
            Snd::Switch => {
                self.noise(0.0, 0.06, Bandpass, 1200.0, 800.0, 0.2 * v, 0.0)?;
            }
        }
        Ok(())
    }
}
