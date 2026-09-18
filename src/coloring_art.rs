//! Production line art for Classic American Iron.
//!
//! Each plate is a **closed-path coloring drawing**, not a family blob.
//! Side/¾ bodies use published inches (length / wheelbase / height), and the
//! outline **goes over the tires** (large-arc wheel lips). ¾ adds a front
//! fascia. Stroke only — kids color the panels.

use crate::coloring::Car;
use crate::coloring_svg::View;

/// Local canvas (recto group).
pub const ART_W: f64 = 520.0;
/// Local canvas height.
pub const ART_H: f64 = 186.0;
const GROUND: f64 = 172.0;
const MAX_LEN: f64 = 225.0;
const SCALE: f64 = 470.0 / MAX_LEN;

/// Draw the recto car in local [`ART_W`]×[`ART_H`] space.
pub fn draw(car: &Car, view: View) -> String {
    let l = layout(car);
    match view {
        View::Front => front(&l),
        View::Rear => rear(&l),
        View::Badge => badge(&l),
        View::Profile => side(&l, false),
        View::ThreeQuarter => side(&l, true),
    }
}

#[derive(Clone, Copy)]
enum Roof {
    Convertible,
    Coupe,
    Fastback,
    Split,
    Kamm,
    Highboy,
    Chopped,
}

#[derive(Clone, Copy)]
enum Jewel {
    Fins59,
    Split63,
    Pipes,
    Buttress,
    ShakerHockey,
    BossScoop,
    Deuce,
    BelAir57,
    Porthole,
    Stacked64,
    Mustang65,
    Camaro67,
    Cowl70,
    RumbleStripe,
    TransAm,
    Sled,
    ThreeWin,
    Skylark,
    Batwing,
    Gt40,
    RamAir,
    Javelin,
    Gto72,
    Riviera,
}

#[derive(Clone, Copy)]
enum Wheel {
    Mag5,
    Mag8,
    Wire,
    Moon,
}

struct Dim {
    id: &'static str,
    length: f64,
    wb: f64,
    height: f64,
    width: f64,
    front_oh: f64,
    tire: f64,
    tire_r: f64,
    ride: f64,
    cowl_frac: f64,
    roof: Roof,
    jewel: Jewel,
}

struct Lyt {
    id: &'static str,
    x0: f64,
    x1: f64,
    fx: f64,
    rx: f64,
    rf: f64,
    rr: f64,
    cyf: f64,
    cyr: f64,
    rocker: f64,
    hood_y: f64,
    roof_y: f64,
    deck_y: f64,
    cowl: f64,
    roof0: f64,
    roof1: f64,
    width: f64,
    jewel: Jewel,
    wheel: Wheel,
}

fn dim_of(car: &Car) -> Dim {
    let m = car.model.to_ascii_lowercase();
    if m.contains("eldorado") {
        d(
            "eldorado",
            225.0,
            130.0,
            54.0,
            80.0,
            38.0,
            14.0,
            14.0,
            9.0,
            0.40,
            Roof::Convertible,
            Jewel::Fins59,
        )
    } else if m.contains("sting") || m.contains("corvette") {
        d(
            "stingray",
            175.0,
            98.0,
            50.0,
            70.0,
            40.0,
            13.0,
            13.0,
            7.5,
            0.48,
            Roof::Split,
            Jewel::Split63,
        )
    } else if m.contains("cobra") {
        d(
            "cobra",
            156.0,
            90.0,
            48.0,
            68.0,
            32.0,
            12.5,
            15.5,
            8.0,
            0.46,
            Roof::Convertible,
            Jewel::Pipes,
        )
    } else if m.contains("charger") {
        d(
            "charger",
            208.0,
            117.0,
            53.0,
            77.0,
            38.0,
            13.5,
            13.5,
            8.5,
            0.44,
            Roof::Fastback,
            Jewel::Buttress,
        )
    } else if m.contains("cuda") {
        d(
            "cuda",
            187.0,
            108.0,
            51.0,
            75.0,
            36.0,
            13.5,
            14.0,
            8.0,
            0.45,
            Roof::Fastback,
            Jewel::ShakerHockey,
        )
    } else if m.contains("boss") {
        d(
            "boss",
            187.0,
            108.0,
            51.0,
            72.0,
            37.0,
            13.5,
            13.5,
            8.0,
            0.50,
            Roof::Fastback,
            Jewel::BossScoop,
        )
    } else if m.contains("deuce") {
        d(
            "deuce",
            148.0,
            106.0,
            60.0,
            68.0,
            22.0,
            13.0,
            14.0,
            16.0,
            0.50,
            Roof::Highboy,
            Jewel::Deuce,
        )
    } else if m.contains("bel air") {
        d(
            "belair",
            200.0,
            115.0,
            59.0,
            77.0,
            36.0,
            13.5,
            13.5,
            9.0,
            0.42,
            Roof::Coupe,
            Jewel::BelAir57,
        )
    } else if m.contains("thunderbird") {
        d(
            "tbird",
            175.0,
            102.0,
            52.0,
            70.0,
            34.0,
            13.0,
            13.0,
            8.0,
            0.46,
            Roof::Coupe,
            Jewel::Porthole,
        )
    } else if m.contains("mustang gt") || (m.contains("mustang") && car.year == 1965) {
        d(
            "mustang65",
            182.0,
            108.0,
            51.0,
            68.0,
            35.0,
            13.0,
            13.0,
            8.0,
            0.50,
            Roof::Fastback,
            Jewel::Mustang65,
        )
    } else if m.contains("camaro") {
        d(
            "camaro",
            185.0,
            108.0,
            51.0,
            73.0,
            36.0,
            13.5,
            13.5,
            8.0,
            0.48,
            Roof::Coupe,
            Jewel::Camaro67,
        )
    } else if m.contains("chevelle") {
        d(
            "chevelle",
            197.0,
            112.0,
            53.0,
            76.0,
            37.0,
            13.5,
            13.5,
            8.5,
            0.44,
            Roof::Coupe,
            Jewel::Cowl70,
        )
    } else if m.contains("challenger") {
        d(
            "challenger",
            191.0,
            110.0,
            51.0,
            76.0,
            36.0,
            13.5,
            14.0,
            8.0,
            0.46,
            Roof::Coupe,
            Jewel::RumbleStripe,
        )
    } else if m.contains("trans am") {
        d(
            "transam",
            192.0,
            108.0,
            50.0,
            73.0,
            37.0,
            13.5,
            13.5,
            7.5,
            0.48,
            Roof::Fastback,
            Jewel::TransAm,
        )
    } else if m.contains("skylark") {
        d(
            "skylark",
            196.0,
            122.0,
            58.0,
            76.0,
            34.0,
            13.0,
            13.0,
            9.5,
            0.44,
            Roof::Convertible,
            Jewel::Skylark,
        )
    } else if m.contains("impala") {
        d(
            "impala",
            209.0,
            118.0,
            57.0,
            78.0,
            38.0,
            14.0,
            14.0,
            9.0,
            0.40,
            Roof::Coupe,
            Jewel::Batwing,
        )
    } else if m.contains("gt40") {
        d(
            "gt40",
            160.0,
            95.0,
            40.0,
            70.0,
            36.0,
            12.0,
            13.0,
            5.5,
            0.52,
            Roof::Kamm,
            Jewel::Gt40,
        )
    } else if m.contains("442") {
        d(
            "442",
            203.0,
            112.0,
            52.0,
            76.0,
            37.0,
            13.5,
            13.5,
            8.5,
            0.44,
            Roof::Coupe,
            Jewel::RamAir,
        )
    } else if m.contains("javelin") {
        d(
            "javelin",
            191.0,
            110.0,
            52.0,
            75.0,
            36.0,
            13.5,
            13.5,
            8.0,
            0.50,
            Roof::Fastback,
            Jewel::Javelin,
        )
    } else if m.contains("gto") && car.year >= 1970 {
        d(
            "gto72",
            203.0,
            112.0,
            52.0,
            77.0,
            37.0,
            13.5,
            13.5,
            8.5,
            0.44,
            Roof::Coupe,
            Jewel::Gto72,
        )
    } else if m.contains("gto") {
        d(
            "gto64",
            203.0,
            115.0,
            54.0,
            73.0,
            38.0,
            13.5,
            13.5,
            8.5,
            0.42,
            Roof::Coupe,
            Jewel::Stacked64,
        )
    } else if m.contains("riviera") {
        d(
            "riviera",
            208.0,
            117.0,
            53.0,
            77.0,
            38.0,
            13.0,
            13.0,
            8.0,
            0.46,
            Roof::Fastback,
            Jewel::Riviera,
        )
    } else if m.contains("3-window") || m.contains("3 window") {
        d(
            "threewin",
            160.0,
            112.0,
            62.0,
            68.0,
            24.0,
            13.0,
            13.5,
            14.0,
            0.48,
            Roof::Highboy,
            Jewel::ThreeWin,
        )
    } else {
        d(
            "mercury",
            214.0,
            118.0,
            52.0,
            76.0,
            40.0,
            13.5,
            13.5,
            8.0,
            0.40,
            Roof::Chopped,
            Jewel::Sled,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn d(
    id: &'static str,
    length: f64,
    wb: f64,
    height: f64,
    width: f64,
    front_oh: f64,
    tire: f64,
    tire_r: f64,
    ride: f64,
    cowl_frac: f64,
    roof: Roof,
    jewel: Jewel,
) -> Dim {
    Dim {
        id,
        length,
        wb,
        height,
        width,
        front_oh,
        tire,
        tire_r,
        ride,
        cowl_frac,
        roof,
        jewel,
    }
}

fn wheel_of(j: Jewel) -> Wheel {
    match j {
        Jewel::Pipes | Jewel::Deuce | Jewel::ThreeWin => Wheel::Wire,
        Jewel::Fins59 | Jewel::BelAir57 | Jewel::Sled | Jewel::Batwing | Jewel::Skylark => {
            Wheel::Moon
        }
        Jewel::Gt40 => Wheel::Mag8,
        _ => Wheel::Mag5,
    }
}

fn layout(car: &Car) -> Lyt {
    let g = dim_of(car);
    let len = g.length * SCALE;
    let x0 = (ART_W - len) * 0.5;
    let x1 = x0 + len;
    let fx = x0 + g.front_oh * SCALE;
    let rx = fx + g.wb * SCALE;
    let rf = g.tire * SCALE;
    let rr = g.tire_r * SCALE;
    let roof_y = GROUND - g.height * SCALE;
    let hood_y = match g.roof {
        Roof::Kamm => GROUND - g.height * SCALE * 0.70,
        Roof::Convertible => GROUND - g.height * SCALE * 0.55,
        Roof::Highboy => GROUND - g.height * SCALE * 0.58,
        Roof::Chopped => GROUND - g.height * SCALE * 0.76,
        Roof::Split => GROUND - g.height * SCALE * 0.62,
        _ => GROUND - g.height * SCALE * 0.60,
    };
    let cowl = x0 + g.length * SCALE * g.cowl_frac;
    let roof0 = cowl + 8.0;
    let roof1 = match g.roof {
        Roof::Fastback | Roof::Split | Roof::Kamm => x1 - 36.0,
        Roof::Convertible => cowl + 38.0,
        Roof::Highboy => cowl + (x1 - x0) * 0.38,
        _ => x0 + len * 0.70,
    };
    Lyt {
        id: g.id,
        x0,
        x1,
        fx,
        rx,
        rf,
        rr,
        cyf: GROUND - rf,
        cyr: GROUND - rr,
        rocker: GROUND - g.ride * SCALE,
        hood_y,
        roof_y,
        deck_y: hood_y + (GROUND - g.ride * SCALE - hood_y) * 0.18,
        cowl,
        roof0,
        roof1,
        width: g.width * SCALE,
        jewel: g.jewel,
        wheel: wheel_of(g.jewel),
    }
}

struct Pen(String);

impl Pen {
    fn new() -> Self {
        Self(String::with_capacity(12_288))
    }
    fn d(&mut self, path: &str) {
        self.0.push_str("<path d=\"");
        self.0.push_str(path);
        self.0.push_str("\"/>");
    }
    fn c(&mut self, cx: f64, cy: f64, r: f64) {
        self.0.push_str(&format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}"/>"#
        ));
    }
    fn e(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        self.0.push_str(&format!(
            r#"<ellipse cx="{cx:.1}" cy="{cy:.1}" rx="{rx:.1}" ry="{ry:.1}"/>"#
        ));
    }
    fn wheel(&mut self, cx: f64, cy: f64, r: f64, squash: f64, thick: f64, kind: Wheel) {
        let rx = r * squash;
        self.e(cx, cy, rx, r);
        if thick > 0.4 {
            self.e(cx + thick, cy - thick * 0.28, rx * 0.93, r * 0.93);
        }
        self.e(cx, cy, rx * 0.78, r * 0.78);
        self.e(cx, cy, rx * 0.70, r * 0.70);
        match kind {
            Wheel::Wire => {
                let n = 14u32;
                for i in 0..n {
                    let a = (i as f64) * std::f64::consts::TAU / (n as f64);
                    self.d(&format!(
                        "M {x1:.1} {y1:.1} L {x2:.1} {y2:.1}",
                        x1 = cx + rx * 0.10 * a.cos(),
                        y1 = cy + r * 0.10 * a.sin(),
                        x2 = cx + rx * 0.68 * a.cos(),
                        y2 = cy + r * 0.68 * a.sin()
                    ));
                }
                self.e(cx, cy, rx * 0.12, r * 0.12);
            }
            Wheel::Moon => {
                self.e(cx, cy, rx * 0.48, r * 0.48);
                self.e(cx, cy, rx * 0.18, r * 0.18);
                for i in 0..8 {
                    let a = (i as f64) * std::f64::consts::TAU / 8.0 - 0.12;
                    self.d(&format!(
                        "M {x1:.1} {y1:.1} L {x2:.1} {y2:.1}",
                        x1 = cx + rx * 0.20 * a.cos(),
                        y1 = cy + r * 0.20 * a.sin(),
                        x2 = cx + rx * 0.44 * a.cos(),
                        y2 = cy + r * 0.44 * a.sin()
                    ));
                }
            }
            Wheel::Mag8 | Wheel::Mag5 => {
                let n = if matches!(kind, Wheel::Mag8) { 8 } else { 5 };
                for i in 0..n {
                    let a0 = (i as f64) * std::f64::consts::TAU / (n as f64) - 0.18;
                    let a1 = a0 + 0.42;
                    self.d(&format!(
                        "M {hx:.1} {hy:.1} L {x1:.1} {y1:.1} L {x2:.1} {y2:.1} Z",
                        hx = cx,
                        hy = cy,
                        x1 = cx + rx * 0.66 * a0.cos(),
                        y1 = cy + r * 0.66 * a0.sin(),
                        x2 = cx + rx * 0.66 * a1.cos(),
                        y2 = cy + r * 0.66 * a1.sin()
                    ));
                }
                self.e(cx, cy, rx * 0.16, r * 0.16);
            }
        }
    }
    fn take(self) -> String {
        self.0
    }
}

fn arch_xs(cx: f64, cy: f64, r: f64, rocker: f64) -> (f64, f64) {
    let dy = rocker - cy;
    let inner = (r * r - dy * dy).max(9.0).sqrt();
    (cx - inner, cx + inner)
}

/// Wheel-lip commands: large arc **over** the tire (y-down, right → left).
fn wells(l: &Lyt, extra: f64) -> String {
    let af = l.rf + extra;
    let ar = l.rr + extra;
    let (fl, fr) = arch_xs(l.fx, l.cyf, af, l.rocker);
    let (rl, rr) = arch_xs(l.rx, l.cyr, ar, l.rocker);
    format!(
        "L {rr:.1} {rk:.1} A {ar:.1} {ar:.1} 0 1 0 {rl:.1} {rk:.1} \
         L {fr:.1} {rk:.1} A {af:.1} {af:.1} 0 1 0 {fl:.1} {rk:.1}",
        rk = l.rocker
    )
}

fn well_lip(p: &mut Pen, l: &Lyt, extra: f64) {
    let af = l.rf + extra;
    let ar = l.rr + extra;
    let (fl, fr) = arch_xs(l.fx, l.cyf, af, l.rocker);
    let (rl, rr) = arch_xs(l.rx, l.cyr, ar, l.rocker);
    p.d(&format!(
        "M {rr:.1} {rk:.1} A {ar:.1} {ar:.1} 0 1 0 {rl:.1} {rk:.1}",
        rk = l.rocker
    ));
    p.d(&format!(
        "M {fr:.1} {rk:.1} A {af:.1} {af:.1} 0 1 0 {fl:.1} {rk:.1}",
        rk = l.rocker
    ));
}

fn side(l: &Lyt, tq: bool) -> String {
    let mut p = Pen::new();
    p.d("M 18 172 H 502");
    p.e(260.0, 176.0, 200.0, 5.0);
    let squash = if tq { 0.72 } else { 1.0 };
    let thick = if tq { 6.5 } else { 0.0 };
    if tq {
        let dz = l.width * 0.20;
        p.wheel(l.fx + dz, l.cyf - 11.0, l.rf * 0.82, 0.46, 0.0, l.wheel);
        p.wheel(l.rx + dz, l.cyr - 11.0, l.rr * 0.82, 0.46, 0.0, l.wheel);
        far_cabin(&mut p, l, dz);
    }
    paint(&mut p, l);
    p.wheel(l.fx, l.cyf, l.rf, squash, thick, l.wheel);
    p.wheel(l.rx, l.cyr, l.rr, squash, thick, l.wheel);
    if tq {
        fascia_3q(&mut p, l);
    }
    format!(r#"<g id="plate-{}">{}</g>"#, l.id, p.take())
}

fn far_cabin(p: &mut Pen, l: &Lyt, dz: f64) {
    p.d(&format!(
        "M {x0:.1} {y0:.1} L {x1:.1} {y1:.1} L {x2:.1} {y2:.1}",
        x0 = l.cowl + dz * 0.35,
        y0 = l.hood_y - 6.0,
        x1 = l.roof0 + dz * 0.55,
        y1 = l.roof_y - 8.0,
        x2 = l.roof1 + dz * 0.45,
        y2 = l.roof_y - 5.0
    ));
}

fn paint(p: &mut Pen, l: &Lyt) {
    match l.id {
        "eldorado" => eldorado(p, l),
        "stingray" => stingray(p, l),
        "cobra" => cobra(p, l),
        "charger" => charger(p, l),
        "cuda" => cuda(p, l),
        "boss" => boss(p, l),
        "deuce" => deuce(p, l),
        "belair" => belair(p, l),
        "tbird" => tbird(p, l),
        "mustang65" => mustang65(p, l),
        "camaro" => camaro(p, l),
        "chevelle" => chevelle(p, l),
        "challenger" => challenger(p, l),
        "transam" => transam(p, l),
        "skylark" => skylark(p, l),
        "impala" => impala(p, l),
        "gt40" => gt40(p, l),
        "442" => olds442(p, l),
        "javelin" => javelin(p, l),
        "gto72" => gto72(p, l),
        "gto64" => gto64(p, l),
        "riviera" => riviera(p, l),
        "threewin" => threewin(p, l),
        _ => mercury(p, l),
    }
}

fn cabin_bits(p: &mut Pen, l: &Lyt, door_x: f64) {
    p.d(&format!(
        "M {x:.1} {y0:.1} L {x:.1} {y1:.1}",
        x = door_x,
        y0 = l.hood_y + 3.0,
        y1 = l.rocker + 1.0
    ));
    p.d(&format!(
        "M {x:.1} {y0:.1} L {x:.1} {y1:.1}",
        x = l.cowl + 6.0,
        y0 = l.hood_y + 2.0,
        y1 = l.rocker + 1.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1}",
        a = door_x + 8.0,
        y = (l.hood_y + l.rocker) * 0.52,
        b = door_x + 18.0
    ));
    p.d(&format!(
        "M {x:.1} {y:.1} L {x2:.1} {y2:.1} L {x3:.1} {y3:.1}",
        x = l.cowl + 2.0,
        y = l.hood_y + 6.0,
        x2 = l.cowl - 10.0,
        y2 = l.hood_y + 10.0,
        x3 = l.cowl - 8.0,
        y3 = l.hood_y + 16.0
    ));
}

fn bumper_bar(p: &mut Pen, x0: f64, x1: f64, y: f64, h: f64) {
    p.d(&format!(
        "M {x0:.1} {y:.1} Q {xm:.1} {y2:.1} {x1:.1} {y:.1} L {x1:.1} {yb:.1} Q {xm:.1} {yb2:.1} {x0:.1} {yb:.1} Z",
        xm = (x0 + x1) * 0.5,
        y2 = y - 4.0,
        yb = y + h,
        yb2 = y + h + 3.0
    ));
}

fn eldorado(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, r0, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.roof0, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {b1:.1} {n2:.1} {hy2:.1} {hl:.1} {hy3:.1} \
         C {hp:.1} {fpy:.1} {fx:.1} {fpy:.1} {c:.1} {hy:.1} \
         L {r0:.1} {ry:.1} L {r1:.1} {rh:.1} L {rb:.1} {hy:.1} \
         C {mid:.1} {hy:.1} {rx:.1} {dy:.1} {fin:.1} {fy:.1} \
         L {tip:.1} {ty:.1} L {t:.1} {dy2:.1} \
         C {t:.1} {rk:.1} {t2:.1} {rk:.1} {t2:.1} {rk:.1} \
         {w} Z",
        b1 = rk - 20.0,
        n2 = n + 16.0,
        hy2 = hy + 28.0,
        hl = n + 28.0,
        hy3 = hy + 16.0,
        hp = l.fx - 8.0,
        fpy = hy - 14.0,
        fx = l.fx + 10.0,
        r1 = r0 + 34.0,
        rh = ry + 10.0,
        rb = r0 + 42.0,
        mid = (c + t) * 0.58,
        rx = l.rx,
        dy = l.deck_y - 6.0,
        fin = t - 28.0,
        fy = ry + 10.0,
        tip = t + 14.0,
        ty = ry - 22.0,
        dy2 = l.deck_y + 8.0,
        t2 = t - 16.0,
        w = wells(l, 7.0)
    ));
    well_lip(p, l, 3.0);
    bumper_bar(p, n - 2.0, n + 44.0, rk - 6.0, 10.0);
    p.e(n + 22.0, hy + 14.0, 7.0, 5.0);
    p.e(n + 22.0, hy + 26.0, 6.5, 4.5);
    p.e(n + 12.0, rk - 10.0, 5.0, 4.0);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {c:.1} {y1:.1} L {d:.1} {y0:.1} Z",
        a = c + 2.0,
        y0 = hy + 1.0,
        b = r0 + 4.0,
        y1 = ry + 4.0,
        c = r0 + 30.0,
        d = c + 28.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} Q {m:.1} {y2:.1} {b:.1} {y:.1}",
        a = n + 50.0,
        y = hy + 18.0,
        m = (n + t) * 0.5,
        y2 = hy + 8.0,
        b = l.rx - 8.0
    ));
    cabin_bits(p, l, (c + l.roof1) * 0.55);
    p.c(t - 10.0, ry + 6.0, 5.5);
}

fn stingray(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {pt:.1} {n:.1} {hy:.1} {ns:.1} {hy:.1} \
         C {fx:.1} {fp:.1} {fx2:.1} {hy:.1} {c:.1} {hy:.1} \
         C {r0:.1} {ry:.1} {r1:.1} {ry:.1} {pk:.1} {pky:.1} \
         C {t1:.1} {dy:.1} {t:.1} {d2:.1} {t:.1} {rk:.1} \
         {w} Z",
        pt = hy + 36.0,
        ns = n + 18.0,
        fx = l.fx,
        fp = hy - 16.0,
        fx2 = l.fx + 18.0,
        r0 = l.roof0,
        r1 = l.roof1,
        pk = (l.roof1 + t) * 0.52,
        pky = ry + 8.0,
        t1 = t - 18.0,
        dy = l.deck_y,
        d2 = l.deck_y + 16.0,
        w = wells(l, 6.5)
    ));
    well_lip(p, l, 2.8);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {sp:.1} {y1:.1} L {c:.1} {y0:.1} Z",
        a = c + 4.0,
        y0 = hy + 1.0,
        b = l.roof0 + 6.0,
        y1 = ry + 5.0,
        sp = (l.roof0 + l.roof1) * 0.5 - 2.0,
        c = (c + l.roof1) * 0.48
    ));
    p.d(&format!(
        "M {a:.1} {y1:.1} L {b:.1} {y1:.1} L {c:.1} {y0:.1} L {d:.1} {y0:.1} Z",
        a = (l.roof0 + l.roof1) * 0.5 + 2.0,
        y1 = ry + 5.0,
        b = l.roof1 - 4.0,
        c = l.roof1 + 10.0,
        y0 = hy + 4.0,
        d = (c + l.roof1) * 0.52
    ));
    p.d(&format!(
        "M {x:.1} {y0:.1} L {x:.1} {y1:.1}",
        x = (l.roof0 + l.roof1) * 0.5,
        y0 = ry + 2.0,
        y1 = hy + 2.0
    ));
    for i in 0..3 {
        let y = hy + 10.0 + (i as f64) * 7.0;
        p.d(&format!(
            "M {a:.1} {y:.1} Q {m:.1} {y2:.1} {b:.1} {y:.1}",
            a = c - 6.0,
            m = c + 8.0,
            y2 = y - 3.0,
            b = c + 22.0
        ));
    }
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
        a = n + 8.0,
        y = hy + 12.0,
        y2 = hy + 18.0,
        b = n + 40.0
    ));
    cabin_bits(p, l, (c + l.roof1) * 0.52);
    bumper_bar(p, n - 4.0, n + 28.0, rk - 4.0, 8.0);
}

fn cobra(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} C {n:.1} {h1:.1} {n2:.1} {hy:.1} {c:.1} {hy:.1} \
         L {r0:.1} {ry:.1} L {r1:.1} {hy:.1} \
         C {m:.1} {hy:.1} {t1:.1} {dy:.1} {t:.1} {rk:.1} Z",
        h1 = hy + 22.0,
        n2 = n + 22.0,
        r0 = l.roof0,
        r1 = l.roof0 + 30.0,
        m = (c + t) * 0.55,
        t1 = t - 24.0,
        dy = l.deck_y
    ));
    kidney(p, l.fx, l.cyf, l.rf + 10.0, rk + 1.0);
    kidney(p, l.rx, l.cyr, l.rr + 11.0, rk + 1.0);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {c:.1} {y1:.1} L {d:.1} {y0:.1} Z",
        a = c + 1.0,
        y0 = hy + 1.0,
        b = l.roof0 + 3.0,
        y1 = ry + 3.0,
        c = l.roof0 + 26.0,
        d = c + 26.0
    ));
    for i in 0..4 {
        p.e(l.fx + 18.0 + (i as f64) * 18.0, GROUND - 7.0, 8.5, 4.2);
    }
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1}",
        a = l.fx + 12.0,
        y = GROUND - 7.0,
        b = l.rx - 14.0
    ));
    p.d(&format!(
        "M {x:.1} {y0:.1} L {x:.1} {y1:.1} M {x2:.1} {y0:.1} L {x2:.1} {y1:.1}",
        x = n + 10.0,
        x2 = n + 16.0,
        y0 = hy + 8.0,
        y1 = rk - 4.0
    ));
    bumper_bar(p, n - 2.0, n + 26.0, rk - 5.0, 8.0);
}

fn kidney(p: &mut Pen, cx: f64, cy: f64, r: f64, rocker: f64) {
    let (a, b) = arch_xs(cx, cy, r, rocker);
    p.d(&format!(
        "M {a:.1} {rocker:.1} \
         C {a:.1} {y1:.1} {cx:.1} {yt:.1} {b:.1} {y1:.1} \
         L {b:.1} {rocker:.1} \
         A {r:.1} {r:.1} 0 1 1 {a:.1} {rocker:.1} Z",
        y1 = cy - r * 0.15,
        yt = cy - r - 6.0
    ));
    let (ia, ib) = arch_xs(cx, cy, r - 4.0, rocker);
    p.d(&format!(
        "M {ib:.1} {rocker:.1} A {ir:.1} {ir:.1} 0 1 0 {ia:.1} {rocker:.1}",
        ir = r - 4.0
    ));
}

fn charger(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {h1:.1} {n2:.1} {hy:.1} {c:.1} {hy:.1} \
         C {r0:.1} {ry:.1} {r1:.1} {ry:.1} {bt:.1} {by:.1} \
         C {t1:.1} {dy:.1} {t:.1} {d2:.1} {t:.1} {rk:.1} \
         {w} Z",
        h1 = hy + 18.0,
        n2 = n + 22.0,
        r0 = l.roof0,
        r1 = l.roof1 - 10.0,
        bt = t - 40.0,
        by = ry + 22.0,
        t1 = t - 16.0,
        dy = l.deck_y,
        d2 = l.deck_y + 18.0,
        w = wells(l, 6.5)
    ));
    well_lip(p, l, 2.8);
    p.d(&format!(
        "M {a:.1} {y0:.1} Q {m:.1} {y1:.1} {b:.1} {y2:.1} L {c:.1} {y3:.1} L {a:.1} {y4:.1} Z",
        a = l.roof1 - 8.0,
        y0 = ry + 4.0,
        m = l.roof1 + 18.0,
        y1 = ry + 18.0,
        b = t - 28.0,
        y2 = l.deck_y - 2.0,
        c = l.roof1 + 4.0,
        y3 = hy + 6.0,
        y4 = hy + 4.0
    ));
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
        a = n + 6.0,
        b = c - 8.0,
        y = hy + 12.0,
        y2 = hy + 20.0
    ));
    cabin_bits(p, l, (c + l.roof1) * 0.50);
    bumper_bar(p, n - 2.0, n + 36.0, rk - 5.0, 9.0);
}

fn glass_fastback(p: &mut Pen, l: &Lyt) {
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {c:.1} {y1:.1} L {d:.1} {y0:.1} Z",
        a = l.cowl + 5.0,
        y0 = l.hood_y + 1.0,
        b = l.roof0 + 6.0,
        y1 = l.roof_y + 5.0,
        c = (l.roof0 + l.roof1) * 0.55,
        d = (l.cowl + l.roof1) * 0.46
    ));
    p.d(&format!(
        "M {a:.1} {y1:.1} L {b:.1} {y1:.1} L {c:.1} {y0:.1} L {d:.1} {y0:.1} Z",
        a = (l.roof0 + l.roof1) * 0.57,
        y1 = l.roof_y + 5.0,
        b = l.roof1 - 4.0,
        c = l.roof1 + 12.0,
        y0 = l.hood_y + 5.0,
        d = (l.cowl + l.roof1) * 0.50
    ));
}

fn glass_coupe(p: &mut Pen, l: &Lyt) {
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {c:.1} {y1:.1} L {d:.1} {y0:.1} Z",
        a = l.cowl + 5.0,
        y0 = l.hood_y + 1.0,
        b = l.roof0 + 6.0,
        y1 = l.roof_y + 5.0,
        c = (l.roof0 + l.roof1) * 0.52,
        d = (l.cowl + l.roof1) * 0.45
    ));
    p.d(&format!(
        "M {a:.1} {y1:.1} L {b:.1} {y1:.1} L {c:.1} {y0:.1} L {d:.1} {y0:.1} Z",
        a = (l.roof0 + l.roof1) * 0.54,
        y1 = l.roof_y + 5.0,
        b = l.roof1 - 6.0,
        c = l.roof1 + 6.0,
        y0 = l.hood_y + 3.0,
        d = (l.cowl + l.roof1) * 0.48
    ));
}

fn muscle_box(p: &mut Pen, l: &Lyt, peak: f64, tail_drop: f64) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {h1:.1} {n2:.1} {hy:.1} {fx:.1} {fp:.1} \
         C {fx2:.1} {hy:.1} {c:.1} {hy:.1} {c:.1} {hy:.1} \
         C {r0:.1} {ry:.1} {r1:.1} {ry:.1} {t1:.1} {dy:.1} \
         C {t:.1} {d2:.1} {t:.1} {rk:.1} {t:.1} {rk:.1} \
         {w} Z",
        h1 = hy + 16.0,
        n2 = n + 20.0,
        fx = l.fx,
        fp = hy - peak,
        fx2 = l.fx + 16.0,
        r0 = l.roof0,
        r1 = l.roof1,
        t1 = t - 22.0,
        dy = l.deck_y,
        d2 = l.deck_y + tail_drop,
        w = wells(l, 6.5)
    ));
    well_lip(p, l, 2.8);
}

fn cuda(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 12.0, 16.0);
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y0:.1} L {b:.1} {y1:.1} L {a:.1} {y1:.1} Z",
        a = l.fx + 4.0,
        y0 = l.hood_y - 11.0,
        b = l.cowl - 8.0,
        y1 = l.hood_y
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} L {b:.1} {y2:.1} L {c:.1} {y3:.1}",
        a = l.fx - 10.0,
        y = l.hood_y + 16.0,
        b = l.cowl + 12.0,
        y2 = l.hood_y + 12.0,
        c = l.rx + 6.0,
        y3 = l.deck_y + 14.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} L {b:.1} {y2:.1} L {c:.1} {y3:.1}",
        a = l.fx - 10.0,
        y = l.hood_y + 22.0,
        b = l.cowl + 12.0,
        y2 = l.hood_y + 18.0,
        c = l.rx + 6.0,
        y3 = l.deck_y + 20.0
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.52);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 34.0, l.rocker - 5.0, 9.0);
    p.e(l.x0 + 20.0, l.hood_y + 14.0, 6.0, 4.5);
}

fn boss(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 14.0, 14.0);
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y0:.1} L {c:.1} {y1:.1} L {d:.1} {y1:.1} Z",
        a = l.fx + 2.0,
        y0 = l.hood_y - 13.0,
        b = l.cowl - 4.0,
        c = l.cowl - 10.0,
        y1 = l.hood_y,
        d = l.fx + 8.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1}",
        a = l.x0 + 4.0,
        y = l.rocker + 3.0,
        b = l.fx - 8.0
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 32.0, l.rocker - 5.0, 8.0);
    p.c(l.x0 + 18.0, l.hood_y + 14.0, 5.0);
    p.c(l.x0 + 18.0, l.hood_y + 24.0, 5.0);
}

fn deuce(p: &mut Pen, l: &Lyt) {
    p.d(&format!(
        "M {x0:.1} {y0:.1} L {x0:.1} {y1:.1} L {x1:.1} {y1:.1} L {x1:.1} {y0:.1} Z",
        x0 = l.cowl - 4.0,
        y0 = l.rocker,
        x1 = l.roof1 + 12.0,
        y1 = l.hood_y
    ));
    p.d(&format!(
        "M {x0:.1} {y0:.1} L {x0:.1} {y1:.1} L {x1:.1} {y1:.1} L {x1:.1} {y0:.1} Z",
        x0 = l.roof0,
        y0 = l.hood_y,
        x1 = l.roof1,
        y1 = l.roof_y
    ));
    p.d(&format!(
        "M {n:.1} {y0:.1} L {n:.1} {rk:.1} L {s:.1} {rk:.1} L {s:.1} {y0:.1} Z",
        n = l.x0,
        y0 = l.hood_y + 4.0,
        rk = l.rocker,
        s = l.x0 + 24.0
    ));
    kidney(p, l.fx, l.cyf, l.rf + 9.0, l.rocker);
    kidney(p, l.rx, l.cyr, l.rr + 9.0, l.rocker);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
        a = l.x0 + 3.0,
        b = l.x0 + 22.0,
        y = l.hood_y + 8.0,
        y2 = l.rocker - 6.0
    ));
    for i in 0..4 {
        let y = l.hood_y + 14.0 + (i as f64) * 8.0;
        p.d(&format!(
            "M {a:.1} {y:.1} H {b:.1}",
            a = l.x0 + 5.0,
            b = l.x0 + 20.0
        ));
    }
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.roof0 + 4.0,
        y0 = l.roof_y + 3.0,
        b = l.roof1 - 4.0,
        y1 = l.hood_y - 2.0
    ));
}

fn belair(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {h1:.1} {n2:.1} {hy:.1} {c:.1} {hy:.1} \
         C {fx:.1} {fp:.1} {c:.1} {hy:.1} {r0:.1} {ry:.1} \
         C {r1:.1} {ry:.1} {fin:.1} {fy:.1} {t:.1} {ty:.1} \
         C {t:.1} {d2:.1} {t:.1} {rk:.1} {t:.1} {rk:.1} \
         {w} Z",
        h1 = hy + 14.0,
        n2 = n + 24.0,
        fx = l.fx,
        fp = hy - 12.0,
        r0 = l.roof0,
        r1 = l.roof1,
        fin = t - 24.0,
        fy = ry + 8.0,
        ty = l.deck_y - 10.0,
        d2 = l.deck_y + 14.0,
        w = wells(l, 6.8)
    ));
    well_lip(p, l, 3.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} L {b:.1} {y2:.1} L {c:.1} {y3:.1}",
        a = l.rx + 4.0,
        y = l.deck_y,
        b = t - 6.0,
        y2 = ry + 10.0,
        c = t,
        y3 = l.deck_y + 6.0
    ));
    p.e(n + 24.0, hy + 14.0, 6.5, 5.0);
    p.c(t - 10.0, ry + 16.0, 5.0);
    cabin_bits(p, l, (c + l.roof1) * 0.52);
    bumper_bar(p, n - 2.0, n + 40.0, rk - 6.0, 10.0);
    p.e(n + 14.0, rk - 10.0, 5.5, 4.0);
}

fn tbird(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 10.0, 12.0);
    glass_coupe(p, l);
    p.c(l.roof1 + 6.0, (l.roof_y + l.hood_y) * 0.5, 8.5);
    p.c(l.roof1 + 6.0, (l.roof_y + l.hood_y) * 0.5, 3.4);
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 30.0, l.rocker - 5.0, 8.0);
    p.e(l.x0 + 20.0, l.hood_y + 14.0, 6.0, 5.0);
}

fn mustang65(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 11.0, 13.0);
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
        a = l.fx + 6.0,
        b = l.cowl - 8.0,
        y = l.hood_y + 8.0,
        y2 = l.hood_y + 14.0
    ));
    p.c(l.x0 + 22.0, l.hood_y + 16.0, 4.8);
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 28.0, l.rocker - 5.0, 8.0);
}

fn camaro(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 12.0, 14.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
        a = l.fx + 2.0,
        y = l.hood_y + 6.0,
        b = l.cowl + 28.0,
        y2 = l.hood_y + 16.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1}",
        a = l.x0 + 6.0,
        y = l.hood_y + 12.0,
        b = l.x0 + 30.0
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.52);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 32.0, l.rocker - 5.0, 8.0);
}

fn chevelle(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 11.0, 15.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.cowl - 28.0,
        y0 = l.hood_y - 9.0,
        b = l.cowl + 2.0,
        y1 = l.hood_y + 2.0
    ));
    p.c(l.x0 + 24.0, l.hood_y + 14.0, 5.5);
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 34.0, l.rocker - 5.0, 9.0);
}

fn challenger(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 12.0, 15.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {x:.1} {y0:.1} L {x:.1} {y1:.1} M {x2:.1} {y0:.1} L {x2:.1} {y1:.1}",
        x = l.fx + 14.0,
        x2 = l.fx + 20.0,
        y0 = l.hood_y,
        y1 = l.rocker
    ));
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.fx + 2.0,
        y0 = l.hood_y - 10.0,
        b = l.cowl - 8.0,
        y1 = l.hood_y
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.52);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 34.0, l.rocker - 5.0, 9.0);
}

fn transam(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 13.0, 12.0);
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} L {b:.1} {y:.1} L {c:.1} {y2:.1} L {d:.1} {y2:.1} Z",
        a = l.rx + 2.0,
        y = l.roof_y + 4.0,
        b = l.x1 - 2.0,
        c = l.x1 - 10.0,
        y2 = l.roof_y + 16.0,
        d = l.rx + 10.0
    ));
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.fx + 6.0,
        y0 = l.hood_y - 11.0,
        b = l.cowl - 6.0,
        y1 = l.hood_y
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1}",
        a = l.x0,
        y = l.rocker + 3.0,
        b = l.fx - 8.0
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 32.0, l.rocker - 5.0, 8.0);
}

fn skylark(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, r0, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.roof0, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} C {n:.1} {h1:.1} {n2:.1} {hy:.1} {c:.1} {hy:.1} \
         L {r0:.1} {ry:.1} L {r1:.1} {hy:.1} \
         C {m:.1} {hy:.1} {t1:.1} {dy:.1} {t:.1} {rk:.1} \
         {w} Z",
        h1 = hy + 18.0,
        n2 = n + 22.0,
        r1 = r0 + 32.0,
        m = (c + t) * 0.55,
        t1 = t - 28.0,
        dy = l.deck_y,
        w = wells(l, 6.5)
    ));
    well_lip(p, l, 2.8);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {c:.1} {y1:.1} L {d:.1} {y0:.1} Z",
        a = c + 2.0,
        y0 = hy + 1.0,
        b = r0 + 4.0,
        y1 = ry + 4.0,
        c = r0 + 28.0,
        d = c + 28.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} Q {m:.1} {y2:.1} {b:.1} {y:.1}",
        a = n + 18.0,
        y = hy + 18.0,
        m = c,
        y2 = hy + 6.0,
        b = l.rx - 8.0
    ));
    p.e(n + 26.0, hy + 14.0, 6.5, 5.0);
    cabin_bits(p, l, (c + l.roof1) * 0.48);
    bumper_bar(p, n - 2.0, n + 36.0, rk - 6.0, 9.0);
}

fn impala(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {h1:.1} {n2:.1} {hy:.1} {c:.1} {hy:.1} \
         C {r0:.1} {ry:.1} {r1:.1} {ry:.1} {wing:.1} {wy:.1} \
         L {t:.1} {ty:.1} C {t:.1} {d2:.1} {t:.1} {rk:.1} {t:.1} {rk:.1} \
         {w} Z",
        h1 = hy + 14.0,
        n2 = n + 24.0,
        r0 = l.roof0,
        r1 = l.roof1,
        wing = t - 22.0,
        wy = ry - 8.0,
        ty = l.deck_y + 4.0,
        d2 = l.deck_y + 16.0,
        w = wells(l, 7.0)
    ));
    well_lip(p, l, 3.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y1:.1} L {c:.1} {y2:.1} L {d:.1} {y3:.1}",
        a = l.rx,
        y0 = l.deck_y,
        b = l.rx + 20.0,
        y1 = ry - 8.0,
        c = t - 16.0,
        y2 = ry - 12.0,
        d = t,
        y3 = l.deck_y + 8.0
    ));
    p.e(n + 24.0, hy + 12.0, 6.0, 4.5);
    p.e(n + 24.0, hy + 24.0, 5.5, 4.0);
    cabin_bits(p, l, (c + l.roof1) * 0.52);
    bumper_bar(p, n - 2.0, n + 40.0, rk - 6.0, 10.0);
}

fn gt40(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} L {n:.1} {hy:.1} L {c:.1} {hy:.1} L {r0:.1} {ry:.1} \
         L {r1:.1} {ry:.1} L {t:.1} {dy:.1} L {t:.1} {rk:.1} {w} Z",
        r0 = l.roof0,
        r1 = l.roof1,
        dy = l.deck_y,
        w = wells(l, 5.5)
    ));
    well_lip(p, l, 2.4);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
        a = l.roof0 + 8.0,
        y = ry - 11.0,
        b = l.roof1 - 8.0,
        y2 = ry
    ));
    p.d(&format!(
        "M {x:.1} {y0:.1} L {x:.1} {y1:.1}",
        x = (l.roof0 + l.roof1) * 0.5,
        y0 = ry,
        y1 = rk
    ));
    glass_fastback(p, l);
    p.c(n + 16.0, hy + 10.0, 4.0);
    p.c(n + 28.0, hy + 10.0, 4.0);
    bumper_bar(p, n - 2.0, n + 22.0, rk - 3.0, 6.0);
}

fn olds442(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 11.0, 15.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.fx + 8.0,
        y0 = l.hood_y - 8.0,
        b = l.fx + 28.0,
        y1 = l.hood_y
    ));
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.fx + 36.0,
        y0 = l.hood_y - 8.0,
        b = l.fx + 56.0,
        y1 = l.hood_y
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 34.0, l.rocker - 5.0, 9.0);
}

fn javelin(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 10.0, 12.0);
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} Q {b:.1} {y1:.1} {c:.1} {y2:.1}",
        a = l.cowl,
        y0 = l.hood_y,
        b = l.roof1 + 20.0,
        y1 = l.roof_y + 16.0,
        c = l.x1 - 4.0,
        y2 = l.deck_y + 4.0
    ));
    p.c(l.x0 + 22.0, l.hood_y + 14.0, 5.5);
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.52);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 32.0, l.rocker - 5.0, 8.0);
}

fn gto72(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 11.0, 16.0);
    glass_coupe(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
        a = l.x0 + 6.0,
        y = l.hood_y + 16.0,
        b = l.cowl - 14.0,
        y2 = l.hood_y + 28.0
    ));
    p.c(l.x0 + 22.0, l.hood_y + 12.0, 5.0);
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 34.0, l.rocker - 5.0, 9.0);
}

fn gto64(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 10.0, 16.0);
    glass_coupe(p, l);
    p.c(l.x0 + 24.0, l.hood_y + 10.0, 5.2);
    p.c(l.x0 + 24.0, l.hood_y + 22.0, 5.2);
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.fx + 4.0,
        y0 = l.hood_y - 8.0,
        b = l.fx + 24.0,
        y1 = l.hood_y
    ));
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.fx + 32.0,
        y0 = l.hood_y - 8.0,
        b = l.fx + 52.0,
        y1 = l.hood_y
    ));
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.50);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 36.0, l.rocker - 5.0, 9.0);
}

fn riviera(p: &mut Pen, l: &Lyt) {
    muscle_box(p, l, 9.0, 12.0);
    glass_fastback(p, l);
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1}",
        a = l.x0 + 6.0,
        y = l.hood_y + 14.0,
        b = l.cowl - 8.0
    ));
    p.c(l.x0 + 20.0, l.hood_y + 20.0, 3.6);
    p.c(l.x0 + 30.0, l.hood_y + 20.0, 3.6);
    cabin_bits(p, l, (l.cowl + l.roof1) * 0.48);
    bumper_bar(p, l.x0 - 2.0, l.x0 + 34.0, l.rocker - 5.0, 8.0);
}

fn threewin(p: &mut Pen, l: &Lyt) {
    deuce(p, l);
    p.d(&format!(
        "M {a:.1} {y0:.1} H {b:.1} V {y1:.1} H {a:.1} Z",
        a = l.roof0 + 6.0,
        y0 = l.roof_y + 6.0,
        b = l.roof0 + 22.0,
        y1 = l.hood_y - 2.0
    ));
}

fn mercury(p: &mut Pen, l: &Lyt) {
    let (n, t, hy, ry, c, rk) = (l.x0, l.x1, l.hood_y, l.roof_y, l.cowl, l.rocker);
    p.d(&format!(
        "M {n:.1} {rk:.1} \
         C {n:.1} {h1:.1} {n2:.1} {hy:.1} {c:.1} {hy:.1} \
         C {r0:.1} {ry:.1} {r1:.1} {ry:.1} {t1:.1} {dy:.1} \
         C {t:.1} {d2:.1} {t:.1} {rk:.1} {t:.1} {rk:.1} \
         {w} Z",
        h1 = hy + 10.0,
        n2 = n + 30.0,
        r0 = l.roof0,
        r1 = l.roof1,
        t1 = t - 20.0,
        dy = l.deck_y,
        d2 = l.deck_y + 14.0,
        w = wells(l, 7.0)
    ));
    well_lip(p, l, 3.0);
    glass_coupe(p, l);
    p.e(n + 22.0, hy + 14.0, 6.0, 5.0);
    p.d(&format!(
        "M {a:.1} {y:.1} L {b:.1} {y:.1}",
        a = n + 6.0,
        y = hy + 14.0,
        b = n + 16.0
    ));
    cabin_bits(p, l, (c + l.roof1) * 0.50);
    bumper_bar(p, n - 2.0, n + 38.0, rk - 6.0, 10.0);
}

fn fascia_3q(p: &mut Pen, l: &Lyt) {
    let d = l.width * 0.38;
    let n = l.x0;
    let f = n - d * 0.42;
    p.d(&format!(
        "M {n:.1} {hy:.1} L {f:.1} {hy2:.1} L {f:.1} {rk:.1} L {n:.1} {rk:.1} Z",
        hy = l.hood_y + 2.0,
        hy2 = l.hood_y + 10.0,
        rk = l.rocker + 1.0
    ));
    p.d(&format!(
        "M {f:.1} {y:.1} H {n:.1} V {y2:.1} H {f:.1} Z",
        y = l.rocker - 12.0,
        y2 = l.rocker + 2.0
    ));
    match l.jewel {
        Jewel::Fins59 => {
            p.e(f + 10.0, l.hood_y + 18.0, 5.0, 4.0);
            p.e(f + 10.0, l.hood_y + 28.0, 4.5, 3.5);
            p.e((f + n) * 0.5, l.rocker - 8.0, 4.0, 3.2);
            let mut y = l.hood_y + 16.0;
            while y < l.rocker - 16.0 {
                p.d(&format!(
                    "M {a:.1} {y:.1} H {b:.1}",
                    a = f + 4.0,
                    b = n - 4.0
                ));
                y += 6.0;
            }
        }
        Jewel::Pipes => {
            p.c((f + n) * 0.55, (l.hood_y + l.rocker) * 0.5, 11.0);
            p.c((f + n) * 0.55, (l.hood_y + l.rocker) * 0.5, 5.0);
        }
        Jewel::Split63 | Jewel::Riviera => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = f + 4.0,
                b = n - 3.0,
                y = l.hood_y + 16.0,
                y2 = l.hood_y + 28.0
            ));
        }
        Jewel::Deuce | Jewel::ThreeWin => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = f + 3.0,
                b = n - 2.0,
                y = l.hood_y + 6.0,
                y2 = l.rocker - 8.0
            ));
        }
        Jewel::Stacked64 => {
            p.c(f + 10.0, l.hood_y + 16.0, 4.5);
            p.c(f + 10.0, l.hood_y + 26.0, 4.5);
        }
        Jewel::Buttress => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = f + 3.0,
                b = n - 2.0,
                y = l.hood_y + 14.0,
                y2 = l.hood_y + 32.0
            ));
        }
        _ => {
            p.c(f + 9.0, l.hood_y + 18.0, 4.2);
            p.c(f + 9.0, l.hood_y + 28.0, 4.0);
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
                a = f + 4.0,
                b = n - 3.0,
                y = l.hood_y + 36.0,
                y2 = l.hood_y + 42.0
            ));
        }
    }
}

fn front(l: &Lyt) -> String {
    let mut p = Pen::new();
    p.d("M 40 172 H 480");
    p.e(260.0, 176.0, 170.0, 5.0);
    let h = (GROUND - l.roof_y).clamp(78.0, 150.0);
    let w = l.width.clamp(150.0, 300.0);
    let x0 = 260.0 - w * 0.5;
    let x1 = 260.0 + w * 0.5;
    let top = GROUND - h;
    p.wheel(x0 + 30.0, GROUND - 28.0, 28.0, 0.40, 0.0, l.wheel);
    p.wheel(x1 - 30.0, GROUND - 28.0, 28.0, 0.40, 0.0, l.wheel);
    p.d(&format!(
        "M {x0:.1} {rk:.1} \
         C {x0:.1} {sh:.1} {xl:.1} {top:.1} {xr:.1} {top:.1} \
         C {x1:.1} {sh:.1} {x1:.1} {rk:.1} {x1:.1} {rk:.1} Z",
        rk = l.rocker,
        sh = l.rocker - h * 0.35,
        xl = x0 + 36.0,
        xr = x1 - 36.0
    ));
    p.d(&format!(
        "M {a:.1} {y0:.1} L {b:.1} {y0:.1} L {c:.1} {y1:.1} L {d:.1} {y1:.1} Z",
        a = x0 + 40.0,
        b = x1 - 40.0,
        y0 = top + h * 0.16,
        c = x1 - 30.0,
        y1 = top + h * 0.42,
        d = x0 + 30.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
        a = x0 + 16.0,
        b = x1 - 16.0,
        y = l.rocker - 16.0,
        y2 = l.rocker - 2.0
    ));
    fascia_front(&mut p, l, x0, x1, top, h);
    format!(r#"<g id="plate-{}-front">{}</g>"#, l.id, p.take())
}

fn fascia_front(p: &mut Pen, l: &Lyt, x0: f64, x1: f64, top: f64, h: f64) {
    let mid = top + h * 0.56;
    match l.jewel {
        Jewel::Fins59 => {
            p.e(x0 + 46.0, mid - 16.0, 9.0, 7.0);
            p.e(x0 + 46.0, mid + 2.0, 8.0, 6.0);
            p.e(x1 - 46.0, mid - 16.0, 9.0, 7.0);
            p.e(x1 - 46.0, mid + 2.0, 8.0, 6.0);
            p.e(x0 + 28.0, l.rocker - 10.0, 7.0, 5.0);
            p.e(x1 - 28.0, l.rocker - 10.0, 7.0, 5.0);
            let mut x = x0 + 72.0;
            while x < x1 - 72.0 {
                p.d(&format!(
                    "M {x:.1} {y0:.1} L {x:.1} {y1:.1}",
                    y0 = mid - 8.0,
                    y1 = mid + 16.0
                ));
                x += 11.0;
            }
        }
        Jewel::Pipes => {
            p.c(260.0, mid, 28.0);
            p.c(260.0, mid, 13.0);
            p.d(&format!(
                "M {x0:.1} {y:.1} L 260 {y2:.1} L {x1:.1} {y:.1}",
                y = top + 10.0,
                y2 = top - 8.0,
                x0 = x0 + 48.0,
                x1 = x1 - 48.0
            ));
        }
        Jewel::Buttress => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = x0 + 28.0,
                b = x1 - 28.0,
                y = mid - 14.0,
                y2 = mid + 14.0
            ));
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
                a = x0 + 40.0,
                b = x1 - 40.0,
                y = mid - 4.0,
                y2 = mid + 4.0
            ));
        }
        Jewel::Deuce | Jewel::ThreeWin => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = 208.0,
                b = 312.0,
                y = top + 6.0,
                y2 = l.rocker - 8.0
            ));
            let mut y = top + 16.0;
            while y < l.rocker - 16.0 {
                p.d(&format!("M 216 {y:.1} H 304"));
                y += 10.0;
            }
        }
        Jewel::Gt40 => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1}",
                a = x0 + 18.0,
                b = x1 - 18.0,
                y = mid + 8.0
            ));
            p.c(x0 + 50.0, mid, 7.0);
            p.c(x1 - 50.0, mid, 7.0);
            p.d("M 260 58 V 118");
        }
        Jewel::Stacked64 => {
            p.e(x0 + 46.0, mid - 12.0, 8.0, 6.5);
            p.e(x0 + 46.0, mid + 6.0, 8.0, 6.5);
            p.e(x1 - 46.0, mid - 12.0, 8.0, 6.5);
            p.e(x1 - 46.0, mid + 6.0, 8.0, 6.5);
        }
        Jewel::Split63 | Jewel::Riviera => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z M {c:.1} {y:.1} H {d:.1} V {y2:.1} H {c:.1} Z",
                a = x0 + 26.0,
                b = x0 + 72.0,
                c = x1 - 72.0,
                d = x1 - 26.0,
                y = mid - 16.0,
                y2 = mid + 10.0
            ));
        }
        Jewel::Mustang65 => {
            p.c(x0 + 48.0, mid, 7.0);
            p.c(x1 - 48.0, mid, 7.0);
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = 232.0,
                b = 288.0,
                y = mid - 10.0,
                y2 = mid + 10.0
            ));
        }
        Jewel::ShakerHockey | Jewel::RumbleStripe => {
            p.e(x0 + 44.0, mid, 8.0, 6.0);
            p.e(x1 - 44.0, mid, 8.0, 6.0);
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
                a = x0 + 70.0,
                b = x1 - 70.0,
                y = mid - 6.0,
                y2 = mid + 8.0
            ));
        }
        _ => {
            p.e(x0 + 46.0, mid, 8.0, 6.0);
            p.e(x1 - 46.0, mid, 8.0, 6.0);
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} M {a:.1} {y2:.1} H {b:.1}",
                a = x0 + 72.0,
                b = x1 - 72.0,
                y = mid - 5.0,
                y2 = mid + 8.0
            ));
        }
    }
}

fn rear(l: &Lyt) -> String {
    let mut p = Pen::new();
    p.d("M 50 172 H 470");
    p.e(260.0, 176.0, 160.0, 5.0);
    let h = (GROUND - l.roof_y).clamp(78.0, 150.0);
    let w = l.width.clamp(150.0, 290.0);
    let x0 = 260.0 - w * 0.5;
    let x1 = 260.0 + w * 0.5;
    let top = GROUND - h;
    p.wheel(x0 + 32.0, GROUND - 28.0, 28.0, 0.40, 0.0, l.wheel);
    p.wheel(x1 - 32.0, GROUND - 28.0, 28.0, 0.40, 0.0, l.wheel);
    p.d(&format!(
        "M {x0:.1} {rk:.1} \
         C {x0:.1} {m:.1} {xl:.1} {top:.1} {xr:.1} {top:.1} \
         C {x1:.1} {m:.1} {x1:.1} {rk:.1} {x1:.1} {rk:.1} Z",
        rk = l.rocker,
        m = (l.rocker + top) * 0.55,
        xl = x0 + 32.0,
        xr = x1 - 32.0
    ));
    p.d(&format!(
        "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
        a = x0 + 14.0,
        b = x1 - 14.0,
        y = l.rocker - 16.0,
        y2 = l.rocker - 2.0
    ));
    match l.jewel {
        Jewel::Fins59 => {
            p.d(&format!(
                "M {a:.1} {y:.1} L {b:.1} {y2:.1} L {c:.1} {y:.1} Z",
                a = x0 + 26.0,
                y = top + 44.0,
                b = x0 + 6.0,
                y2 = top - 10.0,
                c = x0 + 52.0
            ));
            p.d(&format!(
                "M {a:.1} {y:.1} L {b:.1} {y2:.1} L {c:.1} {y:.1} Z",
                a = x1 - 26.0,
                y = top + 44.0,
                b = x1 - 6.0,
                y2 = top - 10.0,
                c = x1 - 52.0
            ));
            p.e(x0 + 22.0, top + 18.0, 7.0, 5.5);
            p.e(x1 - 22.0, top + 18.0, 7.0, 5.5);
            p.e(x0 + 26.0, top + 34.0, 5.5, 4.5);
            p.e(x1 - 26.0, top + 34.0, 5.5, 4.5);
        }
        Jewel::Split63 => {
            p.d(&format!(
                "M {a:.1} {y:.1} L {b:.1} {y:.1} L {b:.1} {y2:.1} L {a:.1} {y2:.1} Z",
                a = x0 + 38.0,
                b = 252.0,
                y = top + 16.0,
                y2 = top + h * 0.48
            ));
            p.d(&format!(
                "M {a:.1} {y:.1} L {b:.1} {y:.1} L {b:.1} {y2:.1} L {a:.1} {y2:.1} Z",
                a = 268.0,
                b = x1 - 38.0,
                y = top + 16.0,
                y2 = top + h * 0.48
            ));
            p.d(&format!(
                "M 260 {y:.1} V {y2:.1}",
                y = top + 16.0,
                y2 = top + h * 0.48
            ));
        }
        Jewel::Buttress => {
            p.d(&format!(
                "M {a:.1} {y:.1} Q 260 {y2:.1} {b:.1} {y:.1} L {c:.1} {y3:.1} L {d:.1} {y3:.1} Z",
                a = x0 + 32.0,
                b = x1 - 32.0,
                y = top + 18.0,
                y2 = top + 4.0,
                c = x1 - 42.0,
                d = x0 + 42.0,
                y3 = top + h * 0.5
            ));
        }
        Jewel::TransAm | Jewel::BossScoop => {
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} L {c:.1} {y2:.1} H {d:.1} Z",
                a = x0 + 16.0,
                b = x1 - 16.0,
                y = top + 4.0,
                c = x1 - 26.0,
                y2 = top + 22.0,
                d = x0 + 26.0
            ));
            p.e(x0 + 50.0, top + h * 0.45, 8.0, 6.0);
            p.e(x1 - 50.0, top + h * 0.45, 8.0, 6.0);
        }
        Jewel::Batwing => {
            p.d(&format!(
                "M {a:.1} {y:.1} L {b:.1} {y2:.1} L 260 {y3:.1} L {c:.1} {y2:.1} L {d:.1} {y:.1} Z",
                a = x0 + 30.0,
                y = top + 36.0,
                b = x0 + 14.0,
                y2 = top + 4.0,
                y3 = top + 22.0,
                c = x1 - 14.0,
                d = x1 - 30.0
            ));
        }
        Jewel::Pipes => {
            p.e(x0 + 36.0, top + h * 0.42, 10.0, 7.0);
            p.e(x1 - 36.0, top + h * 0.42, 10.0, 7.0);
            for i in 0..4 {
                p.e(x0 + 70.0 + (i as f64) * 28.0, l.rocker - 8.0, 7.0, 3.5);
            }
        }
        _ => {
            p.e(x0 + 50.0, top + h * 0.45, 8.0, 6.0);
            p.e(x1 - 50.0, top + h * 0.45, 8.0, 6.0);
            p.d(&format!(
                "M {a:.1} {y:.1} H {b:.1} V {y2:.1} H {a:.1} Z",
                a = x0 + 74.0,
                b = x1 - 74.0,
                y = top + h * 0.32,
                y2 = top + h * 0.48
            ));
        }
    }
    format!(r#"<g id="plate-{}-rear">{}</g>"#, l.id, p.take())
}

fn badge(l: &Lyt) -> String {
    let mut p = Pen::new();
    match l.jewel {
        Jewel::Fins59 => {
            p.d("M 70 178 L 148 22 L 196 38 L 128 186 Z");
            p.e(176.0, 56.0, 16.0, 12.0);
            p.e(184.0, 92.0, 12.0, 9.0);
            p.d("M 56 196 H 460");
            p.d("M 214 78 L 372 46 L 392 70 L 236 108 Z");
            p.d("M 150 40 L 240 18 L 248 32 L 164 52");
        }
        Jewel::Split63 => {
            p.d("M 64 32 L 248 16 L 248 176 L 70 186 Z");
            p.d("M 272 16 L 456 32 L 448 186 L 272 176 Z");
            p.d("M 260 16 V 186");
            p.d("M 90 48 Q 160 36 240 48");
            p.d("M 280 48 Q 360 36 430 48");
        }
        Jewel::Pipes => {
            p.e(130.0, 150.0, 32.0, 16.0);
            p.e(260.0, 150.0, 32.0, 16.0);
            p.e(390.0, 150.0, 32.0, 16.0);
            p.d("M 44 96 Q 150 28 250 96");
            p.d("M 270 96 Q 370 28 476 96");
            p.d("M 80 170 H 440");
        }
        Jewel::Buttress => {
            p.d("M 72 40 Q 260 8 448 40 L 424 186 L 92 186 Z");
            p.d("M 128 58 Q 260 88 392 58");
            p.d("M 110 110 H 410");
        }
        Jewel::ShakerHockey => {
            p.d("M 96 62 L 424 50 L 438 136 L 100 148 Z");
            p.d("M 220 28 L 260 6 L 300 28");
            p.d("M 56 160 L 236 134 L 464 178");
            p.d("M 58 176 L 238 150 L 466 194");
        }
        Jewel::BossScoop => {
            p.d("M 72 68 L 448 56 L 458 148 L 82 160 Z");
            p.d("M 112 88 H 400 M 112 110 H 400 M 112 132 H 400");
            p.d("M 200 40 L 260 18 L 320 40");
        }
        _ => {
            p.d("M 86 40 L 434 32 L 448 180 L 92 186 Z");
            p.c(260.0, 104.0, 36.0);
            p.c(260.0, 104.0, 16.0);
            p.d("M 140 70 H 380");
        }
    }
    format!(r#"<g id="plate-{}-badge">{}</g>"#, l.id, p.take())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coloring::load_roster;

    fn strokes(s: &str) -> usize {
        s.matches("<path").count() + s.matches("<circle").count() + s.matches("<ellipse").count()
    }

    fn body_only(s: &str) -> &str {
        s.split_once('>').map(|(_, rest)| rest).unwrap_or(s)
    }

    #[test]
    fn every_car_is_its_own_drawing() {
        let r = load_roster().unwrap();
        let mut tqs = Vec::new();
        for c in &r.cars {
            let tq = draw(c, View::ThreeQuarter);
            assert!(
                strokes(&tq) >= 28,
                "{} {} {} still a blob ({})",
                c.year,
                c.make,
                c.model,
                strokes(&tq)
            );
            assert!(
                tq.contains("<ellipse") || tq.contains("<circle"),
                "wheels {}",
                c.model
            );
            tqs.push(tq);
        }
        for i in 0..tqs.len() {
            for j in 0..i {
                assert_ne!(
                    body_only(&tqs[i]),
                    body_only(&tqs[j]),
                    "duplicate silhouette {} vs {}",
                    r.cars[i].model,
                    r.cars[j].model
                );
            }
        }
        let sedan = draw(&r.cars[3], View::Profile);
        assert!(
            sedan.contains(" 0 1 0 "),
            "wheel lips must arc over the tire, got {sedan}"
        );
        let sting = r.cars.iter().find(|c| c.model.contains("Sting")).unwrap();
        let rear = draw(sting, View::Rear);
        assert!(rear.contains("252") && rear.contains("268"), "split panes");
        let cobra = r.cars.iter().find(|c| c.model.contains("Cobra")).unwrap();
        assert!(draw(cobra, View::ThreeQuarter).contains("<ellipse"));
        let eldo = r
            .cars
            .iter()
            .find(|c| c.model.contains("Eldorado"))
            .unwrap();
        let ep = draw(eldo, View::Profile);
        assert!(ep.contains("Z"), "closed body for coloring");
    }
}
