//! Himalayan and Indian miniature architecture: shikhara temples, chhatris,
//! havelis, flat-roofed hill houses, riverside ghats, elephants and boats.
//!
//! Every generator is deterministic: the order in which random numbers are
//! consumed is stable, so a seed always returns the same painting.

use std::f64::consts::PI;

use crate::color::Color;
use crate::context::Ctx;
use crate::people::{hat03, man, stick01};
use crate::primitives::stroke;
use crate::shapes::{Point, Poly, Shape};

fn filled(pts: Vec<Point>, fill: Color) -> Shape {
    Shape::Poly(Poly::new(pts, Some(fill)))
}

#[allow(clippy::too_many_arguments)]
pub fn boat01(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    _seed: f64,
    len_: f64,
    sca: f64,
    fli: bool,
    depth: f64,
) -> Vec<Shape> {
    let dir = if fli { -1.0 } else { 1.0 };
    let mut shapes: Vec<Shape> = Vec::new();
    if ctx.people > 0.0 {
        let limb_len = [0.0, 30.0, 20.0, 30.0, 10.0, 30.0, 30.0, 30.0, 30.0];
        shapes.extend(man(
            ctx,
            xoff + 20.0 * sca * dir,
            yoff,
            0.5 * sca,
            Some(hat03),
            Some(stick01),
            !fli,
            None,
            Some(&limb_len),
        ));
    }

    let fun1 = |x: f64| (x * PI).sin().max(0.0).powf(0.5) * 7.0 * sca;
    let fun2 = |x: f64| (x * PI).sin().max(0.0).powf(0.5) * 10.0 * sca;

    let mut plist1: Vec<Point> = Vec::new();
    let mut plist2: Vec<Point> = Vec::new();
    let mut i = 0.0;
    let limit = len_ * sca;
    let step = 5.0 * sca;
    if step > 0.0 {
        while i < limit {
            plist1.push((i * dir, fun1(i / len_)));
            plist2.push((i * dir, fun2(i / len_)));
            i += step;
        }
    }
    let mut plist = plist1;
    plist.extend(plist2.iter().rev().copied());

    let fill = ctx.palette.fill;
    shapes.push(filled(
        plist.iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect(),
        fill,
    ));
    let ink = ctx.palette.ink_at(0.4, depth);
    let shifted: Vec<Point> = plist.iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect();
    shapes.extend(stroke(
        ctx,
        &shifted,
        0.0,
        0.0,
        1.0,
        Some(ink),
        0.5,
        1.0,
        Some(&|x: f64| (x * PI * 2.0).sin()),
    ));
    shapes
}

// ---------------------------------------------------------------------------
// Himalayan and Indian miniature architecture
// ---------------------------------------------------------------------------

fn rect_points(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn ellipse_points(cx: f64, cy: f64, rx: f64, ry: f64, n: usize) -> Vec<Point> {
    let n = n.max(8);
    (0..n)
        .map(|i| {
            let a = i as f64 / n as f64 * PI * 2.0;
            (cx + a.cos() * rx, cy + a.sin() * ry)
        })
        .collect()
}

fn poly_stroke(pts: Vec<Point>, col: Color, width: f64) -> Shape {
    Shape::Poly(Poly {
        pts,
        fill: None,
        stroke: Some(col),
        width,
    })
}

/// A North Indian shikhara temple: stepped plinth, a curvilinear tower with
/// vertical bands, an amalaka disc and a kalasha finial.
pub fn shikhara(ctx: &mut Ctx, xoff: f64, yoff: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    // Temple stone reads as whitewashed plaster or marble, lighter than the
    // rock it stands on, with a cool shaded side for volume.
    let stone = ctx
        .palette
        .snow_at(0.98, depth)
        .mix(ctx.palette.wall_at(0.95, depth), 0.16);
    let stone_shade = stone.darken(0.26);
    let ink = ctx.palette.ink_at(0.5, depth);
    let ink_soft = ctx.palette.ink_at(0.18, depth);
    let mut shapes: Vec<Shape> = Vec::new();

    let plinth_h = hei * 0.11;
    let plinth_w = wid;
    shapes.push(filled(
        rect_points(
            xoff - plinth_w * 0.5,
            yoff - plinth_h,
            xoff + plinth_w * 0.5,
            yoff,
        ),
        stone_shade,
    ));
    let plinth = rect_points(
        xoff - plinth_w * 0.5,
        yoff - plinth_h,
        xoff + plinth_w * 0.5,
        yoff,
    );
    shapes.push(poly_stroke(
        plinth
            .iter()
            .chain(std::iter::once(&plinth[0]))
            .copied()
            .collect(),
        ink,
        0.9,
    ));

    let upper_h = plinth_h * 0.55;
    let upper_w = wid * 0.82;
    shapes.push(filled(
        rect_points(
            xoff - upper_w * 0.5,
            yoff - plinth_h - upper_h,
            xoff + upper_w * 0.5,
            yoff - plinth_h,
        ),
        stone,
    ));

    let tower_base = yoff - plinth_h - upper_h;
    let tower_h = hei * 0.66;
    let steps = 10usize;
    let mut left: Vec<Point> = Vec::with_capacity(steps + 1);
    let mut right: Vec<Point> = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let hw = wid * (0.07 + 0.38 * (1.0 - t).powf(0.72));
        let y = tower_base - tower_h * t;
        left.push((xoff - hw, y));
        right.push((xoff + hw, y));
    }
    let mut body = left.clone();
    body.extend(right.iter().rev().copied());
    shapes.push(filled(body, stone));

    // Shaded right flank of the tower.
    let mut flank: Vec<Point> = Vec::new();
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let hw = wid * (0.07 + 0.38 * (1.0 - t).powf(0.72));
        flank.push((xoff + hw * 0.46, tower_base - tower_h * t));
    }
    for i in (0..=steps).rev() {
        let t = i as f64 / steps as f64;
        let hw = wid * (0.07 + 0.38 * (1.0 - t).powf(0.72));
        flank.push((xoff + hw, tower_base - tower_h * t));
    }
    shapes.push(filled(flank, stone_shade.with_alpha(0.7)));

    shapes.push(poly_stroke(left.clone(), ink, 1.0));
    shapes.push(poly_stroke(right.clone(), ink, 1.0));

    for band in 1..=3 {
        let t = band as f64 / 4.0;
        let hw = wid * (0.07 + 0.38 * (1.0 - t).powf(0.72));
        let y = tower_base - tower_h * t;
        shapes.push(poly_stroke(
            vec![(xoff - hw, y), (xoff + hw, y)],
            ink_soft,
            0.4,
        ));
    }
    for ridge in [-0.58, 0.0, 0.58] {
        let mut pts: Vec<Point> = Vec::new();
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let hw = wid * (0.07 + 0.38 * (1.0 - t).powf(0.72));
            pts.push((xoff + hw * ridge, tower_base - tower_h * t));
        }
        shapes.push(poly_stroke(pts, ink_soft, 0.45));
    }

    // A dark, arched niche at the base of the tower.
    let niche_h = tower_h * 0.17;
    let niche_w = wid * 0.15;
    let niche_top = tower_base - niche_h;
    let mut niche = vec![
        (xoff - niche_w * 0.5, tower_base),
        (xoff - niche_w * 0.5, niche_top + niche_w * 0.25),
    ];
    for i in 0..=8 {
        let a = PI + PI * i as f64 / 8.0;
        niche.push((
            xoff + a.cos() * niche_w * 0.5,
            niche_top + niche_w * 0.25 + a.sin() * niche_w * 0.25,
        ));
    }
    niche.push((xoff + niche_w * 0.5, tower_base));
    shapes.push(filled(niche, ink));

    let top_y = tower_base - tower_h;
    let disc_y = top_y + wid * 0.015;
    let disc_rx = wid * 0.20;
    let disc_ry = wid * 0.050;
    shapes.push(filled(
        ellipse_points(xoff, disc_y, disc_rx, disc_ry, 26),
        stone_shade,
    ));
    shapes.push(poly_stroke(
        {
            let mut pts = ellipse_points(xoff, disc_y, disc_rx, disc_ry, 26);
            pts.push(pts[0]);
            pts
        },
        ink,
        0.8,
    ));
    for k in 0..8 {
        let a = k as f64 / 8.0 * PI * 2.0;
        shapes.push(poly_stroke(
            vec![
                (
                    xoff + a.cos() * disc_rx * 0.45,
                    disc_y + a.sin() * disc_ry * 0.45,
                ),
                (xoff + a.cos() * disc_rx, disc_y + a.sin() * disc_ry),
            ],
            ink_soft,
            0.4,
        ));
    }
    let small_y = disc_y - disc_ry * 1.7;
    shapes.push(filled(
        ellipse_points(xoff, small_y, disc_rx * 0.58, disc_ry * 0.72, 20),
        stone,
    ));

    let finial_y = small_y - wid * 0.045;
    shapes.push(poly_stroke(
        vec![(xoff, small_y), (xoff, finial_y - wid * 0.055)],
        ink,
        1.0,
    ));
    shapes.push(Shape::Circle(crate::shapes::Circle::filled(
        xoff,
        finial_y,
        (wid * 0.045).max(1.2),
        stone_shade,
    )));

    // A vermilion pennant, the finishing note on a North Indian shikhara.
    let flag_y = finial_y - wid * 0.055;
    let accent = ctx.palette.accent_at(0.9, depth);
    shapes.push(filled(
        vec![
            (xoff, flag_y),
            (xoff + wid * 0.19, flag_y + wid * 0.035),
            (xoff, flag_y + wid * 0.075),
        ],
        accent,
    ));

    shapes
}

/// An open domed pavilion: platform, slender columns, an onion dome and a
/// kalasha finial.
pub fn chhatri(ctx: &mut Ctx, xoff: f64, yoff: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let stone = ctx.palette.wall_at(0.92, depth);
    let roof = ctx.palette.roof_at(0.86, depth);
    let ink = ctx.palette.ink_at(0.5, depth);
    let ink_soft = ctx.palette.ink_at(0.26, depth);
    let mut shapes: Vec<Shape> = Vec::new();

    let base_h = hei * 0.11;
    shapes.push(filled(
        rect_points(xoff - wid * 0.52, yoff - base_h, xoff + wid * 0.52, yoff),
        stone,
    ));
    shapes.push(poly_stroke(
        rect_points(xoff - wid * 0.52, yoff - base_h, xoff + wid * 0.52, yoff)
            .into_iter()
            .chain(std::iter::once((xoff - wid * 0.52, yoff - base_h)))
            .collect(),
        ink,
        0.7,
    ));

    let col_h = hei * 0.48;
    let col_top = yoff - base_h - col_h;
    let columns = [-0.36, 0.0, 0.36];
    for c in columns {
        let x = xoff + wid * c;
        shapes.push(poly_stroke(
            vec![(x, yoff - base_h), (x, col_top)],
            ink,
            1.1,
        ));
        shapes.push(poly_stroke(
            vec![
                (x - wid * 0.045, yoff - base_h * 0.55),
                (x + wid * 0.045, yoff - base_h * 0.55),
            ],
            ink_soft,
            0.6,
        ));
    }
    shapes.push(filled(
        rect_points(
            xoff - wid * 0.48,
            col_top - hei * 0.045,
            xoff + wid * 0.48,
            col_top,
        ),
        stone,
    ));
    shapes.push(poly_stroke(
        vec![
            (xoff - wid * 0.48, col_top - hei * 0.045),
            (xoff + wid * 0.48, col_top - hei * 0.045),
        ],
        ink,
        0.7,
    ));

    let dome_cy = col_top - hei * 0.045;
    let rx = wid * 0.56;
    let ry = hei * 0.40;
    let mut dome: Vec<Point> = Vec::new();
    for i in 0..=24 {
        let a = PI - PI * i as f64 / 24.0;
        dome.push((
            xoff + a.cos() * rx,
            dome_cy - a.sin().max(0.0).powf(0.72) * ry,
        ));
    }
    shapes.push(filled(dome.clone(), roof));
    shapes.push(poly_stroke(
        {
            let mut edge = dome.clone();
            edge.push((xoff - rx, dome_cy));
            edge
        },
        ink,
        0.9,
    ));
    for k in 1..4 {
        let t = k as f64 / 4.0;
        let y = dome_cy - ry * t;
        let half = rx * (1.0 - t * t).sqrt();
        shapes.push(poly_stroke(
            vec![(xoff - half, y), (xoff + half, y)],
            ink_soft,
            0.4,
        ));
    }

    let spire_base = dome_cy - ry;
    let spire_top = spire_base - hei * 0.14;
    shapes.push(poly_stroke(
        vec![(xoff, spire_base), (xoff, spire_top)],
        ink,
        0.9,
    ));
    shapes.push(Shape::Circle(crate::shapes::Circle::filled(
        xoff,
        spire_top,
        (wid * 0.045).max(1.0),
        roof,
    )));
    shapes
}

/// A Rajput haveli: a walled block with a cornice, jali windows, a central
/// jharokha balcony and a chhatri on the roof.
pub fn haveli(ctx: &mut Ctx, xoff: f64, yoff: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let wall = ctx.palette.wall_at(0.94, depth);
    let wall_shade = ctx.palette.wall_at(0.70, depth);
    let roof = ctx.palette.roof_at(0.88, depth);
    let ink = ctx.palette.ink_at(0.5, depth);
    let ink_soft = ctx.palette.ink_at(0.22, depth);
    let accent = ctx.palette.accent_at(0.75, depth);
    let mut shapes: Vec<Shape> = Vec::new();

    let body_h = hei * 0.84;
    let body_top = yoff - body_h;
    let body = rect_points(xoff - wid * 0.5, body_top, xoff + wid * 0.5, yoff);
    shapes.push(filled(body.clone(), wall));
    shapes.push(poly_stroke(body, ink, 1.0));

    // Cornice and roof line.
    shapes.push(filled(
        rect_points(
            xoff - wid * 0.54,
            body_top - hei * 0.045,
            xoff + wid * 0.54,
            body_top,
        ),
        wall_shade,
    ));
    shapes.push(poly_stroke(
        vec![
            (xoff - wid * 0.54, body_top - hei * 0.045),
            (xoff + wid * 0.54, body_top - hei * 0.045),
        ],
        ink,
        0.8,
    ));
    for k in 0..7 {
        let x = xoff - wid * 0.48 + k as f64 * wid * 0.16;
        shapes.push(filled(
            rect_points(
                x,
                body_top - hei * 0.085,
                x + wid * 0.075,
                body_top - hei * 0.045,
            ),
            wall_shade,
        ));
    }

    // Jali windows: two storeys of three arched openings.
    let win_w = wid * 0.14;
    let win_h = hei * 0.13;
    for row in 0..2 {
        let cy = body_top + body_h * (0.54 + row as f64 * 0.20);
        for col in 0..3 {
            let cx = xoff + (col as f64 - 1.0) * wid * 0.27;
            let mut win = vec![
                (cx - win_w * 0.5, cy),
                (cx - win_w * 0.5, cy - win_h * 0.55),
            ];
            for i in 0..=6 {
                let a = PI + PI * i as f64 / 6.0;
                win.push((
                    cx + a.cos() * win_w * 0.5,
                    cy - win_h * 0.55 + a.sin() * win_h * 0.45,
                ));
            }
            win.push((cx + win_w * 0.5, cy));
            shapes.push(filled(win, ink_soft));
            shapes.push(poly_stroke(
                vec![
                    (cx - win_w * 0.5, cy),
                    (cx + win_w * 0.5, cy),
                    (cx, cy - win_h),
                    (cx - win_w * 0.5, cy),
                ],
                ink_soft,
                0.45,
            ));
            for k in 1..3 {
                let x = cx - win_w * 0.5 + win_w * k as f64 / 3.0;
                shapes.push(poly_stroke(
                    vec![(x, cy), (x, cy - win_h * 0.72)],
                    ink_soft,
                    0.35,
                ));
            }
        }
    }

    // Central jharokha balcony.
    let jw = wid * 0.24;
    let jh = hei * 0.17;
    let jy = body_top + body_h * 0.42;
    shapes.push(filled(
        rect_points(xoff - jw * 0.5, jy - jh, xoff + jw * 0.5, jy),
        wall_shade,
    ));
    shapes.push(poly_stroke(
        rect_points(xoff - jw * 0.5, jy - jh, xoff + jw * 0.5, jy)
            .into_iter()
            .chain(std::iter::once((xoff - jw * 0.5, jy - jh)))
            .collect(),
        ink,
        0.7,
    ));
    let mut balcony_roof: Vec<Point> = Vec::new();
    for i in 0..=10 {
        let a = PI - PI * i as f64 / 10.0;
        balcony_roof.push((xoff + a.cos() * jw * 0.62, jy - jh - a.sin() * hei * 0.05));
    }
    shapes.push(filled(balcony_roof, roof));
    shapes.push(poly_stroke(
        vec![
            (xoff - jw * 0.45, jy - jh - hei * 0.06),
            (xoff - jw * 0.45, jy - jh - hei * 0.11),
        ],
        ink_soft,
        0.5,
    ));
    shapes.push(poly_stroke(
        vec![
            (xoff + jw * 0.45, jy - jh - hei * 0.06),
            (xoff + jw * 0.45, jy - jh - hei * 0.11),
        ],
        ink_soft,
        0.5,
    ));

    // Gateway at ground level.
    let door_w = wid * 0.18;
    let door_h = hei * 0.24;
    let mut door = vec![
        (xoff - door_w * 0.5, yoff),
        (xoff - door_w * 0.5, yoff - door_h * 0.55),
    ];
    for i in 0..=8 {
        let a = PI + PI * i as f64 / 8.0;
        door.push((
            xoff + a.cos() * door_w * 0.5,
            yoff - door_h * 0.55 + a.sin() * door_h * 0.45,
        ));
    }
    door.push((xoff + door_w * 0.5, yoff));
    shapes.push(filled(door, accent));

    // A small chhatri crowns one corner of the roof.
    shapes.extend(chhatri(
        ctx,
        xoff + wid * 0.29,
        body_top - hei * 0.09,
        hei * 0.30,
        wid * 0.26,
        depth,
    ));

    shapes
}

/// A cluster of flat-roofed hill houses with whitewashed walls, small doors,
/// jali windows and a slate roof slab.
pub fn village_house(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    hei: f64,
    wid: f64,
    depth: f64,
) -> Vec<Shape> {
    let houses = 1 + (ctx.rng.random() * 2.5) as usize;
    let wall = ctx.palette.wall_at(0.94, depth);
    let wall_shade = ctx.palette.wall_at(0.72, depth);
    let roof = ctx.palette.roof_at(0.88, depth);
    let ink = ctx.palette.ink_at(0.48, depth);
    let ink_soft = ctx.palette.ink_at(0.22, depth);
    let door_col = ctx.palette.ink_at(0.62, depth);
    let snow = ctx.palette.snow_at(0.55, depth);
    let mut shapes: Vec<Shape> = Vec::new();

    for k in 0..houses {
        let t = if houses == 1 {
            0.5
        } else {
            k as f64 / (houses - 1) as f64
        };
        let hx = xoff + (t - 0.5) * wid * 0.78 + (ctx.rng.random() - 0.5) * wid * 0.05;
        let hh = hei * (0.55 + ctx.rng.random() * 0.45);
        let hw = (wid / houses as f64) * (0.72 + ctx.rng.random() * 0.22);

        let body = rect_points(hx - hw * 0.5, yoff - hh, hx + hw * 0.5, yoff);
        shapes.push(filled(body.clone(), wall));
        let mut outline = body;
        outline.push(outline[0]);
        shapes.push(poly_stroke(outline, ink, 0.75));

        // A recessed string course between storeys.
        shapes.push(filled(
            rect_points(
                hx - hw * 0.52,
                yoff - hh * 0.6,
                hx + hw * 0.52,
                yoff - hh * 0.54,
            ),
            wall_shade,
        ));

        // Flat slate roof and parapet.
        let rh = (hh * 0.12).clamp(3.0, 9.0);
        shapes.push(filled(
            rect_points(hx - hw * 0.6, yoff - hh - rh, hx + hw * 0.6, yoff - hh),
            roof,
        ));
        shapes.push(poly_stroke(
            vec![
                (hx - hw * 0.6, yoff - hh - rh),
                (hx + hw * 0.6, yoff - hh - rh),
            ],
            ink,
            0.85,
        ));
        for p in 0..4 {
            let px = hx + (p as f64 / 3.0 - 0.5) * hw * 1.1;
            shapes.push(poly_stroke(
                vec![(px, yoff - hh - rh), (px, yoff - hh - rh - 2.2)],
                ink_soft,
                0.55,
            ));
        }
        if hei > 34.0 {
            shapes.push(filled(
                rect_points(
                    hx - hw * 0.56,
                    yoff - hh - rh - 1.1,
                    hx + hw * 0.56,
                    yoff - hh - rh + 0.2,
                ),
                snow,
            ));
        }

        // Door and two jali windows.
        let dw = hw * 0.16;
        let dh = hh * 0.3;
        shapes.push(filled(
            rect_points(hx - dw * 0.5, yoff - dh, hx + dw * 0.5, yoff),
            door_col,
        ));
        for wx in [-0.28, 0.28] {
            let ww = hw * 0.13;
            let wh = hh * 0.15;
            let wy = yoff - hh * 0.66;
            let cx = hx + hw * wx;
            shapes.push(filled(
                rect_points(cx - ww * 0.5, wy - wh, cx + ww * 0.5, wy),
                ink_soft,
            ));
            shapes.push(poly_stroke(
                vec![
                    (cx - ww * 0.5, wy - wh),
                    (cx + ww * 0.5, wy - wh),
                    (cx + ww * 0.5, wy),
                    (cx - ww * 0.5, wy),
                    (cx - ww * 0.5, wy - wh),
                ],
                ink_soft,
                0.4,
            ));
            shapes.push(poly_stroke(vec![(cx, wy - wh), (cx, wy)], ink_soft, 0.3));
        }

        if k == 0 && ctx.rng.random() < 0.35 {
            shapes.extend(chhatri(
                ctx,
                hx,
                yoff - hh - rh,
                hei * 0.36,
                hw * 0.34,
                depth,
            ));
        }
    }
    shapes
}

/// A stepped riverside ghat with a small shrine at the top.
pub fn ghat(ctx: &mut Ctx, xoff: f64, yoff: f64, wid: f64, steps: usize, depth: f64) -> Vec<Shape> {
    let stone = ctx.palette.wall_at(0.88, depth);
    let stone_alt = ctx.palette.wall_at(0.70, depth);
    let ink = ctx.palette.ink_at(0.42, depth);
    let ink_soft = ctx.palette.ink_at(0.18, depth);
    let mut shapes: Vec<Shape> = Vec::new();
    let steps = steps.max(3);
    let step_h = 9.0;

    for i in 0..steps {
        let t = i as f64 / steps as f64;
        let hw = wid * 0.5 * (1.0 - 0.12 * t);
        let y = yoff + i as f64 * step_h;
        let fill = if i % 2 == 0 { stone } else { stone_alt };
        shapes.push(filled(
            rect_points(xoff - hw, y, xoff + hw, y + step_h),
            fill,
        ));
        shapes.push(poly_stroke(
            vec![
                (xoff - hw, y),
                (xoff + hw, y),
                (xoff + hw, y + step_h),
                (xoff - hw, y + step_h),
                (xoff - hw, y),
            ],
            ink_soft,
            0.55,
        ));
    }

    // Side retaining walls.
    let bottom = yoff + steps as f64 * step_h;
    shapes.push(poly_stroke(
        vec![
            (xoff - wid * 0.5, yoff),
            (xoff - wid * 0.44, bottom),
            (xoff + wid * 0.44, bottom),
            (xoff + wid * 0.5, yoff),
        ],
        ink,
        0.8,
    ));
    shapes.push(filled(
        rect_points(xoff - wid * 0.5, yoff - 4.0, xoff + wid * 0.5, yoff),
        stone,
    ));
    shapes.push(poly_stroke(
        vec![(xoff - wid * 0.5, yoff), (xoff + wid * 0.5, yoff)],
        ink,
        0.8,
    ));

    // Two lamp posts and a tiny shrine frame the landing.
    for side in [-1.0, 1.0] {
        let x = xoff + side * wid * 0.40;
        shapes.push(poly_stroke(vec![(x, yoff), (x, yoff - 17.0)], ink, 1.0));
        shapes.push(Shape::Circle(crate::shapes::Circle::filled(
            x,
            yoff - 19.0,
            2.4,
            ctx.palette.sun_at(0.55),
        )));
    }
    shapes.extend(shikhara(
        ctx,
        xoff,
        yoff - 2.0,
        34.0,
        (wid * 0.24).max(18.0),
        depth,
    ));
    shapes
}

/// An Indian elephant in profile: domed head, curved trunk, tusks, a painted
/// howdah blanket and heavy legs.
pub fn elephant(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    scale: f64,
    fli: bool,
    depth: f64,
) -> Vec<Shape> {
    let dir = if fli { -1.0 } else { 1.0 };
    let m = |x: f64, y: f64| (xoff + x * dir, yoff + y);
    let s = scale.max(0.2);
    let h = 62.0 * s;
    let w = 92.0 * s;
    let grey = ctx.palette.ink.mix(ctx.palette.rock, 0.42).with_alpha(0.92);
    let grey_dark = grey.darken(0.16).with_alpha(0.92);
    let ink = ctx.palette.ink_at(0.55, depth);
    let accent = ctx.palette.accent_at(0.82, depth);
    let gold = ctx.palette.sun_at(0.75);
    let ivory = ctx.palette.snow_at(0.95, depth);
    let mut shapes: Vec<Shape> = Vec::new();

    // Tail first, behind the body.
    let tail: Vec<Point> = (0..=6)
        .map(|i| {
            let t = i as f64 / 6.0;
            m(
                -w * 0.40,
                -h * 0.62 + t * h * 0.44 + (t * PI).sin() * h * 0.04,
            )
        })
        .collect();
    shapes.push(poly_stroke(tail, ink, 1.4));

    // Legs.
    for (lx, lw) in [(-0.30, 0.11), (-0.12, 0.11), (0.08, 0.10), (0.23, 0.10)] {
        let leg = vec![
            m(lx * w, -h * 0.30),
            m((lx + lw) * w, -h * 0.30),
            m((lx + lw * 0.92) * w, 0.0),
            m((lx + lw * 0.08) * w, 0.0),
        ];
        shapes.push(filled(leg.clone(), grey_dark));
        shapes.push(poly_stroke(
            {
                let mut l = leg;
                l.push(m(lx * w, -h * 0.30));
                l
            },
            ink,
            0.7,
        ));
        for toe in 0..2 {
            let tx = lx * w + lw * w * (0.28 + toe as f64 * 0.38);
            shapes.push(poly_stroke(vec![m(tx, -h * 0.03), m(tx, 0.0)], ink, 0.5));
        }
    }

    // Body.
    let body = (0..=28)
        .map(|i| {
            let a = i as f64 / 28.0 * PI * 2.0;
            m(a.cos() * w * 0.40, -h * 0.56 + a.sin() * h * 0.30)
        })
        .collect::<Vec<_>>();
    shapes.push(filled(body.clone(), grey));
    shapes.push(poly_stroke(
        {
            let mut b = body;
            b.push(m(w * 0.40, -h * 0.56));
            b
        },
        ink,
        0.8,
    ));

    // Painted howdah blanket.
    let blanket = vec![
        m(-w * 0.30, -h * 0.83),
        m(w * 0.18, -h * 0.83),
        m(w * 0.24, -h * 0.55),
        m(-w * 0.32, -h * 0.55),
    ];
    shapes.push(filled(blanket.clone(), accent));
    shapes.push(poly_stroke(
        {
            let mut b = blanket;
            b.push(m(-w * 0.30, -h * 0.83));
            b
        },
        ink,
        0.7,
    ));
    for k in 0..5 {
        let bx = -w * 0.28 + k as f64 * w * 0.115;
        shapes.push(filled(
            vec![
                m(bx, -h * 0.55),
                m(bx + w * 0.04, -h * 0.55),
                m(bx + w * 0.02, -h * 0.49),
            ],
            gold,
        ));
    }
    let seat = rect_points(-w * 0.10 as f64, -h * 0.98, w * 0.08, -h * 0.83);
    shapes.push(filled(seat.iter().map(|&(x, y)| m(x, y)).collect(), gold));
    shapes.push(poly_stroke(
        {
            let mut spts: Vec<Point> = seat.iter().map(|&(x, y)| m(x, y)).collect();
            spts.push(m(-w * 0.10, -h * 0.98));
            spts
        },
        ink,
        0.6,
    ));

    // Head, ear and eye.
    let head = (0..=20)
        .map(|i| {
            let a = i as f64 / 20.0 * PI * 2.0;
            m(
                w * 0.42 + a.cos() * w * 0.15,
                -h * 0.62 + a.sin() * h * 0.20,
            )
        })
        .collect::<Vec<_>>();
    shapes.push(filled(head.clone(), grey));
    shapes.push(poly_stroke(head, ink, 0.7));
    let ear = (0..=18)
        .map(|i| {
            let a = i as f64 / 18.0 * PI * 2.0;
            m(
                w * 0.33 + a.cos() * w * 0.095,
                -h * 0.60 + a.sin() * h * 0.17,
            )
        })
        .collect::<Vec<_>>();
    shapes.push(filled(ear.clone(), grey_dark));
    shapes.push(poly_stroke(ear, ink, 0.55));
    shapes.push(Shape::Circle(crate::shapes::Circle::filled(
        m(w * 0.48, -h * 0.69).0,
        m(w * 0.48, -h * 0.69).1,
        (1.3 * s).max(0.8),
        ink,
    )));

    // Trunk and tusk.
    let trunk: Vec<Point> = (0..=8)
        .map(|i| {
            let t = i as f64 / 8.0;
            m(
                w * (0.52 + 0.10 * (t * PI * 0.9).sin()),
                -h * (0.58 - 0.56 * t),
            )
        })
        .collect();
    shapes.push(poly_stroke(trunk, ink, (4.0 * s).max(1.0)));
    shapes.push(poly_stroke(
        vec![m(w * 0.49, -h * 0.48), m(w * 0.64, -h * 0.34)],
        ivory,
        (2.0 * s).max(0.8),
    ));
    shapes.push(poly_stroke(
        vec![m(w * 0.48, -h * 0.52), m(w * 0.62, -h * 0.40)],
        ivory.with_alpha(0.7),
        (1.2 * s).max(0.6),
    ));

    shapes
}
