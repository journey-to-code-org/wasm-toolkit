//! A pass down the quarter mile: the car's engine, clutch, turbo and gearbox, a thirtieth of a
//! second at a time. Hold the throttle to bring the revs up against the limiter, shift into first
//! on the green, and shift up before the redline.
//!
//! Everything about the car comes from the pack (`Drive`); nothing here knows what it is.

use crate::pack::{Boost, Drive};

pub const HZ: f64 = 30.0;
pub const QUARTER: f64 = 402.0;
/// Ticks between the lights of the tree.
pub const TREE_STEP: i32 = 15;
const IDLE: f64 = 900.0;
/// Ticks against the redline under load before the engine lets go.
pub const BLOWS_AT: f64 = 45.0;
const WHEEL_RADIUS: f64 = 0.3;
const TO_NM: f64 = 1.3558;
const KG: f64 = 0.4536;
/// How much of the torque gets through the driveline.
const DRIVELINE: f64 = 0.96;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    /// Rolled up to the line; Enter starts the tree.
    Stage,
    Tree,
    Run,
    /// Over the line, or beaten: the timeslip.
    Slip,
}

/// What your hands and feet are doing this tick.
#[derive(Clone, Copy, Default)]
pub struct Controls {
    pub throttle: bool,
    pub stage: bool,
    pub up: bool,
    pub down: bool,
    /// The button on the wheel: nitrous, if there is a bottle and something in it.
    pub nitrous: bool,
}

pub struct Race {
    /// Nitrous: how much more it makes while it sprays (0.35 is 35 per cent), the seconds left in
    /// the bottle, and whether it is spraying now.
    pub shot: f64,
    pub bottle: f64,
    pub spraying: bool,
    drive: Drive,
    /// Torque as a fraction of what the car makes stock and healthy.
    gain: f64,
    boost_kind: Option<Boost>,
    /// The boost the torque curve was measured at.
    stock_psi: f64,
    pub phase: Phase,
    pub gear: usize,
    pub rpm: f64,
    pub boost: f64,
    /// Metres down the strip, and metres a second.
    pub pos: f64,
    pub v: f64,
    launch: f64,
    shift: i32,
    clock: i32,
    pub tree: i32,
    pub red: bool,
    /// Milliseconds: reaction, sixty feet, the quarter. And miles an hour over the line.
    pub rt: i32,
    pub sixty: i32,
    pub et: i32,
    pub mph: i32,
    /// The other lane: what he runs and how late he left, in milliseconds, and where he is.
    pub rival_et: i32,
    pub rival_rt: i32,
    pub rival_pos: f64,
    pub won: bool,
    /// How long the engine has been held against its redline under load, in ticks, less what it
    /// has since been given to recover. Too long and it lets go.
    pub strain: f64,
    /// It let go. There is no more push, and the race is lost.
    pub blown: bool,
    /// The other lane is someone real, somewhere else: where they are and who won is told to the
    /// race, not worked out by it.
    pub net: bool,
}

impl Race {
    /// `boost` is the turbo fitted, if it is not the car's own. `rival_et` of 0 is a pass alone.
    pub fn new(drive: &Drive, gain: f64, boost: Option<Boost>, rival_et: i32, rival_rt: i32) -> Race {
        Race {
            shot: 0.0, bottle: 0.0, spraying: false,
            stock_psi: drive.boost.as_ref().map(|b| b.psi).unwrap_or(1.0),
            boost_kind: boost.or_else(|| drive.boost.clone()),
            drive: drive.clone(), gain,
            phase: Phase::Stage, gear: 0, rpm: IDLE, boost: 0.0, pos: 0.0, v: 0.0, launch: 0.0, shift: 0, clock: 0, tree: 0,
            red: false, rt: 0, sixty: 0, et: 0, mph: 0, rival_et, rival_rt, rival_pos: 0.0, won: false, strain: 0.0, blown: false, net: false,
        }
    }

    pub fn redline(&self) -> f64 {
        self.drive.redline
    }

    pub fn turbo(&self) -> bool {
        self.boost_kind.is_some()
    }

    pub fn alone(&self) -> bool {
        self.rival_et == 0 && !self.net
    }

    /// Which light of the tree is lit: 0 to 2 the ambers, 3 the green, -1 none yet.
    pub fn light(&self) -> i32 {
        match self.phase {
            Phase::Stage => -1,
            Phase::Tree => (self.tree / TREE_STEP).min(3),
            _ => 3,
        }
    }

    fn torque(&self, rpm: f64) -> f64 {
        let t = &self.drive.torque;
        if rpm <= t[0][0] {
            return t[0][1];
        }
        for w in t.windows(2) {
            if rpm <= w[1][0] {
                return w[0][1] + (w[1][1] - w[0][1]) * (rpm - w[0][0]) / (w[1][0] - w[0][0]);
            }
        }
        t[t.len() - 1][1]
    }

    fn ratio(&self) -> f64 {
        self.drive.gears[self.gear - 1] * self.drive.final_drive
    }

    pub fn tick(&mut self, c: Controls) {
        let mut c = c;
        if self.drive.auto {
            // An automatic goes when your foot is down a moment after the green, and changes up by itself.
            c.down = false;
            c.up = if self.gear == 0 { c.throttle && self.phase == Phase::Tree && self.tree + 1 >= TREE_STEP * 3 + 8 } else { self.phase == Phase::Run && self.shift == 0 && self.rpm > self.drive.redline - 300.0 };
        }
        if self.phase == Phase::Slip {
            // Past the line: off the throttle and on the brakes.
            self.v = (self.v - 8.0 / HZ).max(0.0);
            self.pos += self.v / HZ;
            self.rpm += (IDLE - self.rpm) / 8.0;
            return;
        }
        if self.phase == Phase::Stage {
            if c.stage {
                self.phase = Phase::Tree;
                self.tree = 0;
            }
        } else {
            self.tree += 1;
        }
        let since_green = (self.tree - TREE_STEP * 3) as f64 * 1000.0 / HZ;
        if !self.alone() && !self.net && self.tree > TREE_STEP * 3 {
            // He leaves on the green, a reaction late, and gathers speed all the way down.
            let t = since_green - self.rival_rt as f64;
            let f = (t / self.rival_et as f64).max(0.0);
            self.rival_pos = if f <= 1.0 { QUARTER * (f / 4.0 + f * f * 3.0 / 4.0) } else { QUARTER + QUARTER * 1.75 * (f - 1.0) };
            // Long gone, with you still on the line or broken down: that is the race.
            if t > self.rival_et as f64 + 12000.0 && self.pos < QUARTER {
                self.et = 0;
                self.won = false;
                self.phase = Phase::Slip;
                return;
            }
        }
        let green = self.phase == Phase::Tree && self.tree >= TREE_STEP * 3;
        if self.phase == Phase::Run || green {
            self.clock += 1;
        }
        if c.up {
            if self.gear == 0 && self.phase != Phase::Stage {
                // Into first: that is the launch, and before the green it is a red light.
                self.gear = 1;
                self.launch = self.rpm;
                self.red = !green;
                self.rt = if green { since_green as i32 } else { 0 };
                self.phase = Phase::Run;
                if self.red {
                    self.clock = 0;
                }
            } else if self.gear > 0 && self.gear < self.drive.gears.len() {
                self.gear += 1;
                self.shift = 5;
            }
        }
        if c.down && self.gear > 1 {
            self.gear -= 1;
            self.shift = 5;
        }
        if self.gear == 0 {
            // In neutral the revs go where your foot puts them, up to the launch limiter.
            let want = if c.throttle { self.drive.launch } else { IDLE };
            self.rpm += (want - self.rpm) / 4.0;
        } else {
            let road = self.v / WHEEL_RADIUS * self.ratio() * 60.0 / std::f64::consts::TAU;
            // The clutch slips the revs down to meet the road, and a hard launch keeps them up on the boost.
            let hold = (self.launch * 0.75).max(1500.0);
            let slip = if self.gear == 1 && c.throttle { (self.rpm - 60.0).max(hold) } else { IDLE };
            self.rpm = road.max(slip);
        }
        if let Some(b) = &self.boost_kind {
            let want = if c.throttle { ((self.rpm - b.from) * b.psi / 2200.0).clamp(0.0, b.psi) } else { 0.0 };
            self.boost += (want - self.boost) / b.lag.max(1.0);
        }
        if self.gear == 0 {
            return;
        }
        let mut push;
        self.spraying = false;
        // Held flat against the limiter in gear, an engine does not last: a second and a half of it,
        // and something lets go. Shifting, or lifting, gives it back.
        if c.throttle && self.rpm >= self.drive.redline && self.phase == Phase::Run {
            self.strain += 1.0;
            if self.strain > BLOWS_AT {
                self.blown = true;
            }
        } else {
            self.strain = (self.strain - 0.5).max(0.0);
        }
        if self.blown {
            push = -0.6;
        } else if self.shift > 0 {
            self.shift -= 1;
            push = 0.0;
        } else if c.throttle && self.rpm < self.drive.redline {
            // Off boost a turbo engine makes under half of what it makes on it.
            let part = if self.boost_kind.is_some() { 0.45 + 0.55 * self.boost / self.stock_psi } else { 1.0 };
            let force = self.torque(self.rpm) * TO_NM * self.ratio() / WHEEL_RADIUS * DRIVELINE;
            // The bottle only opens in gear with the engine pulling: sprayed low down it would go bang.
            self.spraying = c.nitrous && self.shot > 0.0 && self.bottle > 0.0 && self.gear > 0 && self.rpm > 2500.0;
            let more = if self.spraying { 1.0 + self.shot } else { 1.0 };
            if self.spraying {
                self.bottle = (self.bottle - 1.0 / HZ).max(0.0);
            }
            push = (force / (self.drive.weight_lb * KG) * self.gain * part * more).min(self.drive.grip);
        } else {
            push = -1.0;
        }
        push -= self.v * self.v * 0.00029 + 0.15;
        self.v = (self.v + push / HZ).max(0.0);
        if self.blown && self.v == 0.0 {
            // Rolled to a stop with a hole in the block: there is no timeslip for that.
            self.et = 0;
            self.won = false;
            self.phase = Phase::Slip;
            return;
        }
        let before = self.pos;
        self.pos += self.v / HZ;
        let now = self.clock as f64 * 1000.0 / HZ;
        if self.sixty == 0 && self.pos >= 18.3 {
            self.sixty = now as i32;
        }
        if before < QUARTER && self.pos >= QUARTER {
            self.et = (now - if self.v > 0.0 { (self.pos - QUARTER) * 1000.0 / self.v } else { 0.0 }) as i32;
            self.mph = (self.v * 2.237) as i32;
            self.phase = Phase::Slip;
            self.won = !self.alone() && !self.net && !self.red && !self.blown && self.et < self.rival_rt + self.rival_et;
        }
    }
}
