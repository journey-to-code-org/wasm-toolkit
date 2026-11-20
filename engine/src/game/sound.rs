//! What the car sounds like this tick, for a page that can make a sound of its own: how fast the
//! engine is turning and how hard it is working, the turbo, the tyres, the wind and the law. The
//! page turns these into noise; the engine only says what is happening.

use super::*;

/// One tick's sound. An engine speed of nothing is no engine to be heard.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sound {
    pub rpm: f32,
    /// How hard it is working: 0 coasting, 1 flat out.
    pub load: f32,
    /// How much of its boost the turbo is making, 0 to 1; nothing without a turbo.
    pub boost: f32,
    pub cylinders: f32,
    /// Tyres at their limit, 0 to 1.
    pub squeal: f32,
    /// Goes up by one each time the throttle shuts on boost: the page hears the valve let go.
    pub blowoffs: f32,
    /// A siren behind you, 0 or 1.
    pub siren: f32,
    /// How fast it is going, as wind: 0 to 1.
    pub wind: f32,
    /// What its parts make of it, as fractions more (or less): exhaust loudness, depth and rasp,
    /// intake roar, and the turbo's whistle.
    pub loud: f32,
    pub deep: f32,
    pub rasp: f32,
    pub intake: f32,
    pub whistle: f32,
    /// Nitrous spraying: 0 or 1.
    pub spray: f32,
}

/// How many numbers a tick's sound is, for the page.
pub const SOUND_LEN: usize = 14;

impl Sound {
    pub fn as_array(&self) -> [f32; SOUND_LEN] {
        [self.rpm, self.load, self.boost, self.cylinders, self.squeal, self.blowoffs, self.siren, self.wind, self.loud, self.deep, self.rasp, self.intake, self.whistle, self.spray]
    }
}

impl Game {
    /// What is to be heard after this tick, and the turbo's valve if the throttle just shut on boost.
    pub fn listen(&mut self) -> Sound {
        let keys = self.last_keys;
        let gas = keys & KEY_UP != 0;
        let car = self.car();
        let cylinders = car.map_or(4, |c| c.drive.cylinders.max(1)) as f32;
        let turbo = self.owned().zip(car).is_some_and(|(o, c)| self.tune(o).1.is_some() || c.drive.boost.is_some());
        // For a car with no gearbox to ask, the gear it has picked for itself.
        let geared = |_: f64| self.auto.1;
        let mut s = match self.scene {
            Scene::Race => self.race.as_ref().map_or(Sound::default(), |r| Sound {
                rpm: r.rpm as f32,
                load: if r.phase == Phase::Slip { 0.0 } else if gas { 1.0 } else { 0.1 },
                boost: r.boost_share() as f32,
                squeal: if r.spin { 1.0 } else { 0.0 },
                wind: (r.v / 60.0).min(1.0) as f32,
                ..Sound::default()
            }),
            Scene::Road => self.trip.as_ref().map_or(Sound::default(), |t| {
                let rpm = t.gearbox.as_ref().map_or_else(|| geared(t.speed), |b| b.rpm.max(800.0));
                // A bend taken too fast, or the brakes hard on at speed.
                let bend = t.seg(t.z).curve.abs() * (t.speed / 56.0).powi(2) * t.pace.slide;
                let braking = keys & KEY_DOWN != 0 && t.speed > 12.0;
                Sound {
                    rpm: rpm as f32,
                    load: if gas { 1.0 } else { 0.15 },
                    boost: if turbo && gas { ((rpm - 2500.0) / 2500.0).clamp(0.0, 1.0) as f32 } else { 0.0 },
                    squeal: if braking { 0.7 } else { ((bend - 0.6) * 2.0).clamp(0.0, 1.0) as f32 },
                    siren: if t.chase.is_some() { 1.0 } else { 0.0 },
                    wind: (t.speed / 60.0).min(1.0) as f32,
                    ..Sound::default()
                }
            }),
            Scene::Drive => self.town.as_ref().map_or(Sound::default(), |t| {
                let rpm = geared(t.speed);
                Sound {
                    rpm: rpm as f32,
                    load: if gas { 1.0 } else { 0.15 },
                    boost: if turbo && gas { ((rpm - 2500.0) / 2500.0).clamp(0.0, 1.0) as f32 } else { 0.0 },
                    squeal: if keys & KEY_DOWN != 0 && t.speed > 12.0 { 0.6 } else if t.steer.abs() > 0.8 && t.speed > 18.0 { 0.5 } else { 0.0 },
                    siren: if t.chase.is_some() { 1.0 } else { 0.0 },
                    wind: (t.speed / 60.0).min(1.0) as f32,
                    ..Sound::default()
                }
            }),
            Scene::Dyno if self.rollers.phase == 1 => Sound {
                rpm: self.rollers.rpm as f32,
                load: if gas { 1.0 } else { 0.1 },
                boost: if turbo && gas { ((self.rollers.rpm - 2500.0) / 2500.0).clamp(0.0, 1.0) as f32 } else { 0.0 },
                ..Sound::default()
            },
            _ => Sound::default(),
        };
        if !turbo {
            s.boost = 0.0;
        }
        // The throttle shut with the turbo on boost: the valve lets go.
        if self.spooled > 0.5 && s.load < 0.5 && s.rpm > 0.0 {
            self.blowoffs += 1;
        }
        self.spooled = if s.load >= 0.5 { s.boost } else { 0.0 };
        s.cylinders = cylinders;
        s.blowoffs = self.blowoffs as f32;
        if s.rpm > 0.0 {
            self.voice_of_parts(&mut s);
            s.spray = match self.scene {
                Scene::Race => self.race.as_ref().is_some_and(|r| r.spraying),
                Scene::Road => self.trip.as_ref().is_some_and(|t| t.spray > 0.0),
                _ => false,
            } as i32 as f32;
        }
        s
    }

    /// What the parts on the car in the bay do to its sound, added up; and a car with no exhaust
    /// on it at all is very loud and very rough.
    fn voice_of_parts(&self, s: &mut Sound) {
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let mut v = crate::pack::PartSound::default();
        for p in o.fitted.values().filter(|id| cond_of(id) > 0).filter_map(|id| self.part(id)) {
            if let Some(ps) = p.sound {
                (v.loud, v.deep, v.rasp, v.intake, v.whistle) = (v.loud + ps.loud, v.deep + ps.deep, v.rasp + ps.rasp, v.intake + ps.intake, v.whistle + ps.whistle);
            }
        }
        // A slot on the build sheet that the exhaust goes in, with nothing in it: straight out of the manifold.
        let open = o.sheet && car.stock.iter().any(|(slot, id)| self.part(id).is_some_and(|p| p.sound.is_some_and(|ps| ps.loud > 0) || slot.to_lowercase().contains("exhaust system")) && !o.fitted.contains_key(slot));
        if open {
            v.loud += 150;
            v.rasp += 90;
        }
        let pct = |x: i32| (x.clamp(-90, 400) as f32) / 100.0;
        (s.loud, s.deep, s.rasp, s.intake, s.whistle) = (pct(v.loud), pct(v.deep), pct(v.rasp), pct(v.intake), pct(v.whistle));
    }

    /// The engine note as a beep, for a page with no sound of its own; a page that makes its own
    /// says so, and then this is quiet.
    pub(super) fn hum(&self, hz: i32) {
        if !self.voiced {
            tone(hz, 60);
        }
    }
}
