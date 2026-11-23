//! A pass down the quarter mile: the car's engine, clutch, turbo and transmission, a thirtieth of a
//! second at a time. Hold the throttle to bring the revs up against the limiter, shift into first
//! on the green, and shift up before the redline.
//!
//! Everything about the car comes from the pack (`Drive`); nothing here knows what it is.

use crate::pack::{Boost, Drive, Driven};

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
    /// The wheel, to keep it in its lane.
    pub left: bool,
    pub right: bool,
    /// The clutch pedal, in a car with a transmission worked by hand: held in, first can be put in
    /// without going anywhere, and letting it out is the launch.
    pub clutch: bool,
}

/// Either side of a car's best launch speed, how far the revs can be and still make a good start.
pub const BAND: f64 = 600.0;

/// How a launch went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Launch {
    /// Not yet.
    None,
    /// Too few revs: it fell on its face.
    Bogged,
    Good,
    /// Too many: the tires went up in smoke.
    Spun,
}

/// How far either way of the middle of its lane a car can wander before it is over a line: the
/// lane is six meters, the car under two.
pub const LANE: f64 = 2.1;

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
    /// Meters down the strip, and meters a second.
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
    /// Which way, and how hard, this car pulls when it launches: 1 is to the right as hard as its
    /// driven wheels make it, -1 to the left. Nothing on the rollers, where it cannot wander.
    pub lean: f64,
    /// Meters right of the middle of its lane (the center line is to the right, the wall to the
    /// left), how fast it is going that way, and where the wheel is, from -1 to 1.
    pub drift: f64,
    sideways: f64,
    pub wheel: f64,
    /// Over a line: 1 across the center line, -1 into the wall. Either is the race lost.
    pub out: i8,
    /// The tires are spinning this tick.
    pub spin: bool,
    /// How the launch went, how much wheelspin is still to come from it (in seconds), and how
    /// many revs over the band it was let out at: what it took out of the clutch.
    pub start: Launch,
    smoke: f64,
    pub abuse: f64,
    /// In first with the clutch in, on the line, waiting to let it out.
    pub held: bool,
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
            lean: 0.0, drift: 0.0, sideways: 0.0, wheel: 0.0, out: 0, spin: false, start: Launch::None, smoke: 0.0, abuse: 0.0, held: false,
        }
    }

    /// The engine speed it launches best from.
    pub fn best_launch(&self) -> f64 {
        self.drive.launch
    }

    /// A transmission worked by hand, with a clutch pedal.
    pub fn manual(&self) -> bool {
        !self.drive.auto
    }

    pub fn redline(&self) -> f64 {
        self.drive.redline
    }

    /// How much of its boost the turbo is making, 0 to 1.
    pub fn boost_share(&self) -> f64 {
        self.boost_kind.as_ref().map_or(0.0, |b| (self.boost / b.psi.max(1.0)).clamp(0.0, 1.0))
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

    /// Away: the launch, at whatever the revs are now. Within the band of its best launch speed it
    /// is a good one; well under, it bogs; well over, the tires go up in smoke for a while and the
    /// clutch pays for it.
    fn go(&mut self, green: bool, since_green: f64) {
        self.launch = self.rpm;
        self.red = !green;
        self.rt = if green { since_green as i32 } else { 0 };
        self.phase = Phase::Run;
        if self.red {
            self.clock = 0;
        }
        let best = self.drive.launch;
        let over = self.rpm - (best + BAND);
        // The clutch bites, and the revs fall as the car takes up the drive.
        self.rpm *= 0.8;
        self.start = if self.rpm < best - BAND * 2.0 { Launch::Bogged } else if over > 0.0 { Launch::Spun } else { Launch::Good };
        if over > 0.0 {
            self.smoke = (over / 1400.0).min(2.0);
            self.abuse = over;
        }
    }

    /// Torque steer: on the power the driven wheels drag the car to one side, hardest in first
    /// with the tires at their limit; the wheel brings it back, and more quickly the faster it goes.
    fn wander(&mut self, c: Controls, asked: f64) {
        if self.lean == 0.0 {
            return;
        }
        let spin = asked >= 1.0;
        let pull = match self.drive.driven {
            // A car driven from the back only pulls when its tires are spinning.
            Driven::Rear if !spin => 0.0,
            d => d.torque_steer() * asked.min(1.0) / (self.gear.max(1) as f64).powf(1.5) * if spin { 1.4 } else { 1.0 },
        };
        let want = if c.left { -1.0 } else if c.right { 1.0 } else { 0.0 };
        self.wheel += (want - self.wheel) / 5.0;
        let bite = (self.v / 8.0).min(1.0);
        self.sideways = (self.sideways + (pull * self.lean + self.wheel * 4.0 * bite) / HZ) * 0.92;
        if self.v <= 0.0 {
            self.sideways = 0.0;
        }
        self.drift += self.sideways / HZ;
        if self.drift.abs() > LANE {
            self.out = if self.drift > 0.0 { 1 } else { -1 };
        }
    }

    pub fn tick(&mut self, c: Controls) {
        let mut c = c;
        if self.drive.auto {
            // An automatic goes when your foot is down a moment after the green, and changes up by itself.
            c.down = false;
            c.up = if self.gear == 0 { c.throttle && self.phase == Phase::Tree && self.tree + 1 >= TREE_STEP * 3 + 8 } else { self.phase == Phase::Run && self.shift == 0 && self.rpm > self.drive.redline - 300.0 };
        }
        self.spin = false;
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
        let manual = !self.drive.auto;
        if c.up && self.gear == 0 && self.phase != Phase::Stage && manual && c.clutch {
            // Clutch in, into first: it sits on the line in gear, waiting for the clutch to come out.
            self.gear = 1;
            self.held = true;
        } else if self.held && !c.clutch {
            // The clutch let out: that is the launch, and before the green it is a red light.
            self.held = false;
            self.go(green, since_green);
        } else if c.up && !self.held {
            if self.gear == 0 && self.phase != Phase::Stage {
                // Into first without the clutch: dropped in from wherever the revs are.
                self.gear = 1;
                self.go(green, since_green);
            } else if self.gear > 0 && self.gear < self.drive.gears.len() {
                self.gear += 1;
                self.shift = 5;
            }
        }
        if c.down && self.gear > 1 {
            self.gear -= 1;
            self.shift = 5;
        }
        if (self.gear == 0 || self.held) && manual {
            // Out of gear, or the clutch in: the revs go where your foot puts them, up to the
            // limiter. Holding them in the band is feathering the throttle.
            if c.throttle {
                self.rpm += 5200.0 / HZ;
                if self.rpm >= self.drive.redline {
                    self.rpm = self.drive.redline - 250.0;
                }
            } else {
                self.rpm = (self.rpm - 3600.0 / HZ).max(IDLE);
            }
        } else if self.gear == 0 {
            // An automatic sits on the brake against the converter: its revs go to its launch speed.
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
        if self.gear == 0 || self.held {
            return;
        }
        let mut push;
        // How hard the tires are being asked to drive it, as a share of all they have.
        let mut asked = 0.0;
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
            let want = force / (self.drive.weight_lb * KG) * self.gain * part * more;
            asked = (want / self.drive.grip.max(1.0)).min(1.3);
            push = want.min(self.drive.grip);
            // Let out too high, the tires are still spinning: smoke, not speed, and it squirms.
            if self.smoke > 0.0 {
                self.smoke -= 1.0 / HZ;
                push = push.min(self.drive.grip * 0.62);
                asked = asked.max(1.25);
            }
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
        self.spin = asked >= 1.0;
        self.wander(c, asked);
        if self.out != 0 {
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
