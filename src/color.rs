use std::sync::{Arc, OnceLock};

use crate::rng::Rng;

fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

/// Round half to even, matching the reference renderer.
#[inline]
pub fn round_half_even(value: f64) -> f64 {
    value.round_ties_even()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

impl Color {
    pub fn new(r: i32, g: i32, b: i32, a: f64) -> Self {
        Self {
            r: clamp(r as f64, 0.0, 255.0) as u8,
            g: clamp(g as f64, 0.0, 255.0) as u8,
            b: clamp(b as f64, 0.0, 255.0) as u8,
            a: clamp(a, 0.0, 1.0),
        }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn hex(value: &str, a: f64) -> Self {
        let v = value.trim_start_matches('#');
        let parse = |i: usize| u8::from_str_radix(&v[i..i + 2], 16).unwrap_or(0) as i32;
        Self::new(parse(0), parse(2), parse(4), a)
    }

    pub fn with_alpha(self, alpha: f64) -> Self {
        Self::new(self.r as i32, self.g as i32, self.b as i32, alpha)
    }

    pub fn mix(self, other: Self, t: f64) -> Self {
        let t = clamp(t, 0.0, 1.0);
        Self::new(
            round_half_even(self.r as f64 + (other.r as f64 - self.r as f64) * t) as i32,
            round_half_even(self.g as f64 + (other.g as f64 - self.g as f64) * t) as i32,
            round_half_even(self.b as f64 + (other.b as f64 - self.b as f64) * t) as i32,
            self.a + (other.a - self.a) * t,
        )
    }

    pub fn scale(self, factor: f64) -> Self {
        Self::new(
            round_half_even(self.r as f64 * factor) as i32,
            round_half_even(self.g as f64 * factor) as i32,
            round_half_even(self.b as f64 * factor) as i32,
            self.a,
        )
    }

    pub fn lighten(self, t: f64) -> Self {
        self.mix(Color::new(255, 255, 255, self.a), t)
    }

    pub fn darken(self, t: f64) -> Self {
        self.mix(Color::new(0, 0, 0, self.a), t)
    }

    pub fn css(self) -> String {
        format!("rgba({},{},{},{:.3})", self.r, self.g, self.b, self.a)
    }

    pub fn rgb_tuple(self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    pub fn rgba(self) -> (u8, u8, u8, u8) {
        (
            self.r,
            self.g,
            self.b,
            round_half_even(self.a * 255.0) as u8,
        )
    }

    /// Blend toward another color in linear-ish RGB space (used by the UI).
    pub fn blend(self, other: Self, t: f64) -> Self {
        self.mix(other, t)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub name: String,
    pub paper: Color,
    pub ink: Color,
    pub haze: Color,
    pub rock: Color,
    pub fill: Color,
    pub wash: Color,
    pub foliage: Vec<Color>,
    pub trunk: Color,
    pub roof: Color,
    pub wall: Color,
    pub water: Color,
    pub sky_top: Color,
    pub sky_bottom: Color,
    pub mist: Color,
    pub accent: Color,
    pub sun: Color,
    pub distant: Color,
    pub snow: Color,
}

impl Palette {
    fn depth(depth: f64) -> f64 {
        clamp(depth, 0.0, 1.0)
    }

    pub fn shade(&self, base: Color, depth: f64, amount: f64) -> Color {
        base.mix(self.haze, (1.0 - Self::depth(depth)) * amount)
    }

    pub fn ink_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.ink, depth, 0.7).with_alpha(alpha)
    }

    pub fn rock_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.rock, depth, 0.62).with_alpha(alpha)
    }

    pub fn wash_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.wash, depth, 0.5).with_alpha(alpha)
    }

    pub fn foliage_at(&self, rng: &mut Rng, depth: f64, alpha: f64) -> Color {
        let base = self.foliage[(self.foliage.len() as f64 * rng.random()) as usize];
        let jitter = 0.9 + 0.2 * rng.random();
        self.shade(base.scale(jitter), depth, 0.62)
            .with_alpha(alpha)
    }

    pub fn trunk_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.trunk, depth, 0.62).with_alpha(alpha)
    }

    pub fn roof_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.roof, depth, 0.62).with_alpha(alpha)
    }

    pub fn wall_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.wall, depth, 0.35).with_alpha(alpha)
    }

    pub fn water_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.water, depth, 0.45).with_alpha(alpha)
    }

    pub fn mist_at(&self, alpha: f64) -> Color {
        self.mist.with_alpha(alpha)
    }

    pub fn accent_at(&self, alpha: f64, depth: f64) -> Color {
        self.shade(self.accent, depth, 0.25).with_alpha(alpha)
    }

    pub fn sun_at(&self, alpha: f64) -> Color {
        self.sun.with_alpha(alpha)
    }

    pub fn distant_at(&self, t: f64, _depth: f64) -> Color {
        self.distant
            .mix(self.fill, clamp(t, 0.0, 1.0) * 0.55 + 0.15)
            .with_alpha(1.0)
    }

    /// Snow stays close to the paper white in every theme, with only a touch
    /// of the atmospheric haze so high peaks sit behind the foreground.
    pub fn snow_at(&self, alpha: f64, depth: f64) -> Color {
        self.snow
            .mix(self.haze, (1.0 - Self::depth(depth)) * 0.22)
            .with_alpha(alpha)
    }

    pub fn tone(&self, t: f64, depth: f64, alpha: f64) -> Color {
        let t = clamp(t, 0.0, 1.0);
        let base = self.foliage[0].mix(*self.foliage.last().unwrap(), t);
        self.shade(base, depth, 0.62).with_alpha(alpha)
    }
}

#[allow(clippy::too_many_arguments)]
fn p(
    name: &str,
    paper: (i32, i32, i32),
    ink: (i32, i32, i32),
    haze: (i32, i32, i32),
    rock: (i32, i32, i32),
    fill: (i32, i32, i32),
    wash: (i32, i32, i32),
    foliage: &[(i32, i32, i32)],
    trunk: (i32, i32, i32),
    roof: (i32, i32, i32),
    wall: (i32, i32, i32),
    water: (i32, i32, i32),
    sky_top: (i32, i32, i32),
    sky_bottom: (i32, i32, i32),
    mist: (i32, i32, i32),
    accent: (i32, i32, i32),
    sun: (i32, i32, i32),
    distant: (i32, i32, i32),
    snow: (i32, i32, i32),
) -> Palette {
    Palette {
        name: name.into(),
        paper: Color::new(paper.0, paper.1, paper.2, 1.0),
        ink: Color::new(ink.0, ink.1, ink.2, 1.0),
        haze: Color::new(haze.0, haze.1, haze.2, 1.0),
        rock: Color::new(rock.0, rock.1, rock.2, 1.0),
        fill: Color::new(fill.0, fill.1, fill.2, 1.0),
        wash: Color::new(wash.0, wash.1, wash.2, 1.0),
        foliage: foliage
            .iter()
            .map(|c| Color::new(c.0, c.1, c.2, 1.0))
            .collect(),
        trunk: Color::new(trunk.0, trunk.1, trunk.2, 1.0),
        roof: Color::new(roof.0, roof.1, roof.2, 1.0),
        wall: Color::new(wall.0, wall.1, wall.2, 1.0),
        water: Color::new(water.0, water.1, water.2, 1.0),
        sky_top: Color::new(sky_top.0, sky_top.1, sky_top.2, 1.0),
        sky_bottom: Color::new(sky_bottom.0, sky_bottom.1, sky_bottom.2, 1.0),
        mist: Color::new(mist.0, mist.1, mist.2, 1.0),
        accent: Color::new(accent.0, accent.1, accent.2, 1.0),
        sun: Color::new(sun.0, sun.1, sun.2, 1.0),
        distant: Color::new(distant.0, distant.1, distant.2, 1.0),
        snow: Color::new(snow.0, snow.1, snow.2, 1.0),
    }
}

fn themes() -> &'static Vec<Palette> {
    static THEMES: OnceLock<Vec<Palette>> = OnceLock::new();
    THEMES.get_or_init(|| {
        vec![
            // The house palette: warm kangra paper, indigo ink, ochre rock and
            // the vermilion-and-saffron accents of a Pahari miniature.
            p(
                "pahari",
                (243, 231, 206),
                (46, 42, 58),
                (217, 211, 196),
                (154, 107, 69),
                (251, 243, 223),
                (185, 138, 84),
                &[(53, 92, 74), (92, 126, 90), (126, 155, 99), (44, 74, 69)],
                (107, 74, 50),
                (58, 74, 102),
                (241, 223, 192),
                (127, 163, 168),
                (168, 196, 212),
                (240, 217, 184),
                (245, 234, 213),
                (182, 64, 47),
                (232, 184, 92),
                (143, 169, 188),
                (251, 248, 240),
            ),
            // Kangra: cool celadon, rose and a soft apricot horizon.
            p(
                "kangra",
                (246, 238, 226),
                (43, 58, 58),
                (211, 221, 214),
                (111, 140, 116),
                (249, 244, 232),
                (103, 144, 131),
                &[
                    (49, 94, 78),
                    (78, 125, 95),
                    (120, 155, 108),
                    (168, 190, 138),
                ],
                (108, 79, 59),
                (92, 71, 89),
                (244, 230, 206),
                (141, 183, 176),
                (187, 215, 214),
                (246, 220, 196),
                (238, 244, 236),
                (194, 90, 99),
                (240, 200, 120),
                (157, 186, 180),
                (253, 251, 244),
            ),
            // Basohli: the boldest of the hill styles, with vermilion grounds,
            // lapis sky and saturated emerald foliage.
            p(
                "basohli",
                (242, 223, 192),
                (42, 30, 28),
                (216, 191, 162),
                (140, 79, 50),
                (251, 239, 215),
                (178, 87, 53),
                &[(47, 91, 69), (85, 122, 66), (126, 148, 76), (33, 70, 58)],
                (94, 58, 38),
                (128, 52, 44),
                (246, 225, 184),
                (110, 158, 154),
                (127, 169, 192),
                (242, 201, 140),
                (245, 228, 200),
                (197, 43, 43),
                (242, 179, 61),
                (124, 152, 168),
                (255, 247, 232),
            ),
            // Indigo: a monochrome study in lapis and ivory.
            p(
                "indigo",
                (238, 233, 220),
                (31, 44, 74),
                (199, 207, 216),
                (85, 100, 126),
                (244, 241, 231),
                (74, 94, 134),
                &[(44, 66, 86), (63, 90, 107), (82, 111, 120), (35, 56, 72)],
                (63, 58, 58),
                (36, 58, 94),
                (233, 227, 210),
                (123, 155, 176),
                (169, 191, 210),
                (232, 223, 204),
                (233, 237, 240),
                (166, 58, 50),
                (221, 195, 130),
                (130, 151, 174),
                (248, 249, 245),
            ),
            // Saffron: marigold and turmeric over a warm cream ground.
            p(
                "saffron",
                (245, 230, 200),
                (58, 42, 34),
                (225, 199, 159),
                (164, 117, 58),
                (252, 242, 217),
                (192, 138, 60),
                &[(76, 98, 56), (111, 132, 65), (151, 163, 84), (59, 82, 54)],
                (108, 74, 44),
                (138, 78, 36),
                (247, 227, 182),
                (138, 169, 162),
                (185, 207, 210),
                (245, 206, 134),
                (248, 232, 200),
                (194, 74, 42),
                (242, 178, 60),
                (162, 142, 108),
                (255, 250, 236),
            ),
            // Monsoon: wet slate, grey-green hills and a bleached horizon.
            p(
                "monsoon",
                (233, 230, 218),
                (36, 48, 51),
                (195, 205, 200),
                (95, 114, 104),
                (240, 239, 229),
                (93, 122, 114),
                &[(45, 74, 61), (73, 102, 86), (108, 133, 109), (34, 59, 52)],
                (78, 66, 56),
                (58, 74, 82),
                (232, 226, 210),
                (138, 167, 168),
                (158, 177, 182),
                (221, 224, 211),
                (226, 232, 227),
                (168, 74, 60),
                (217, 201, 143),
                (132, 154, 152),
                (246, 247, 242),
            ),
            // Snow: the high Himalaya under a pale winter sky.
            p(
                "snow",
                (240, 242, 242),
                (39, 49, 62),
                (203, 214, 222),
                (124, 138, 152),
                (250, 252, 252),
                (114, 135, 154),
                &[(60, 81, 75), (86, 107, 93), (115, 131, 114), (44, 64, 61)],
                (90, 75, 64),
                (52, 70, 92),
                (237, 241, 239),
                (169, 194, 207),
                (182, 206, 221),
                (239, 242, 237),
                (242, 246, 247),
                (168, 56, 50),
                (233, 223, 188),
                (148, 168, 184),
                (255, 255, 255),
            ),
            // Dusk over the foothills: rose sky, violet shadow, ember light.
            p(
                "dusk",
                (245, 227, 208),
                (56, 47, 60),
                (218, 191, 193),
                (140, 95, 90),
                (249, 234, 219),
                (142, 90, 102),
                &[(79, 74, 80), (111, 92, 97), (139, 111, 98), (60, 58, 68)],
                (95, 68, 56),
                (71, 57, 79),
                (241, 220, 196),
                (178, 145, 164),
                (199, 166, 184),
                (243, 183, 132),
                (242, 217, 200),
                (168, 60, 56),
                (233, 138, 76),
                (176, 140, 148),
                (251, 241, 232),
            ),
            // Pichwai: the gold-leaf shrine palette of Nathdwara.
            p(
                "pichwai",
                (242, 228, 192),
                (35, 51, 58),
                (215, 199, 159),
                (143, 122, 62),
                (250, 240, 212),
                (169, 138, 60),
                &[(43, 90, 82), (62, 115, 97), (94, 138, 106), (31, 74, 68)],
                (104, 73, 42),
                (140, 106, 46),
                (245, 228, 182),
                (111, 158, 161),
                (169, 200, 204),
                (242, 215, 154),
                (247, 233, 197),
                (181, 65, 47),
                (231, 179, 62),
                (143, 169, 159),
                (255, 248, 228),
            ),
            // Nocturne: indigo night, silver ink and a single warm lamp.
            p(
                "nocturne",
                (24, 35, 56),
                (216, 224, 232),
                (51, 68, 94),
                (90, 106, 128),
                (32, 45, 68),
                (70, 90, 120),
                &[(44, 74, 80), (61, 94, 94), (86, 112, 106), (31, 56, 62)],
                (74, 61, 58),
                (42, 56, 80),
                (85, 98, 122),
                (76, 107, 136),
                (20, 32, 58),
                (64, 81, 108),
                (53, 72, 98),
                (194, 84, 76),
                (234, 215, 163),
                (103, 122, 146),
                (232, 238, 240),
            ),
            // Pomegranate: anar red, rose paper and green shadow.
            p(
                "pomegranate",
                (246, 232, 226),
                (60, 38, 48),
                (225, 199, 193),
                (160, 94, 88),
                (251, 239, 230),
                (168, 80, 85),
                &[(62, 93, 72), (95, 124, 85), (138, 154, 98), (46, 74, 61)],
                (106, 68, 54),
                (110, 47, 58),
                (247, 228, 213),
                (154, 180, 178),
                (201, 214, 210),
                (243, 195, 175),
                (248, 231, 222),
                (176, 47, 62),
                (237, 179, 106),
                (169, 138, 139),
                (255, 248, 242),
            ),
            // Tea: the terraced gardens of Darjeeling in spring.
            p(
                "tea",
                (240, 233, 212),
                (46, 52, 40),
                (210, 211, 188),
                (124, 112, 72),
                (247, 242, 222),
                (123, 138, 80),
                &[(60, 90, 52), (88, 117, 62), (123, 144, 75), (44, 71, 47)],
                (95, 74, 48),
                (74, 62, 46),
                (239, 227, 198),
                (143, 169, 160),
                (180, 200, 192),
                (234, 217, 175),
                (242, 238, 220),
                (166, 68, 47),
                (229, 194, 114),
                (149, 163, 138),
                (250, 247, 232),
            ),
        ]
    })
}

pub fn theme_names() -> Vec<&'static str> {
    themes().iter().map(|p| p.name.as_str()).collect()
}

pub fn get_palette(name: &str) -> Result<Palette, String> {
    themes()
        .iter()
        .find(|p| p.name == name)
        .cloned()
        .ok_or_else(|| {
            format!(
                "unknown theme '{}' (available: {})",
                name,
                theme_names().join(", ")
            )
        })
}

/// Colour swatches for UI theme cards: sky, paper, foliage, accent, ink.
pub fn theme_swatch(name: &str) -> Option<[Color; 4]> {
    themes()
        .iter()
        .find(|p| p.name == name)
        .map(|p| [p.sky_top, p.paper, p.foliage[0], p.accent])
}

pub fn palette_arc(name: &str) -> Result<Arc<Palette>, String> {
    get_palette(name).map(Arc::new)
}
