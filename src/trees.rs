//! Trees, shrubs, branches and bark.
//!
//! This is a close port of the original generator, including its
//! deterministic random-number call order.

use std::f64::consts::PI;

use crate::color::Color;
use crate::context::Ctx;
use crate::geometry::{mid_pt, triangulate};
use crate::primitives::{
    blob, blob_points, distance, div, loop_noise, norm_rand, rand_choice_f64, rand_choice_i32,
    rand_gaussian, stroke,
};
use crate::shapes::{Point, Poly, Shape};

fn ink(ctx: &Ctx, alpha: f64, depth: f64) -> Color {
    ctx.palette.ink_at(alpha, depth)
}

fn foliage(ctx: &mut Ctx, depth: f64, alpha: f64) -> Color {
    let palette = ctx.palette.clone();
    palette.foliage_at(&mut ctx.rng, depth, alpha)
}

fn leaf(ctx: &mut Ctx, depth: f64, base_alpha: f64) -> Color {
    let alpha = (base_alpha + ctx.rng.random() * 0.2).min(1.0);
    foliage(ctx, depth, alpha)
}

fn poly_line(pts: Vec<Point>, col: Color, width: f64) -> Shape {
    Shape::Poly(Poly {
        pts,
        fill: None,
        stroke: Some(col),
        width,
    })
}

fn poly_filled(pts: Vec<Point>, fill: Color) -> Shape {
    Shape::Poly(Poly::new(pts, Some(fill)))
}

fn poly_fill_stroke(pts: Vec<Point>, fill: Color, stroke: Color, width: f64) -> Shape {
    Shape::Poly(Poly {
        pts,
        fill: Some(fill),
        stroke: Some(stroke),
        width,
    })
}

#[allow(dead_code)]
fn leaf_alpha(ctx: &mut Ctx, col: Option<Color>) -> Option<Color> {
    col.map(|c| c.with_alpha((c.a + ctx.rng.random() * 0.2).min(1.0)))
}

pub fn tree01(
    ctx: &mut Ctx,
    x: f64,
    y: f64,
    hei: f64,
    wid: f64,
    depth: f64,
    col: Option<Color>,
    trunk: Option<Color>,
) -> Vec<Shape> {
    let reso = 10usize;
    let mut nslist: Vec<(f64, f64)> = Vec::with_capacity(reso);
    for i in 0..reso {
        nslist.push((
            ctx.noise.noise(i as f64 * 0.5, 0.0, 0.0),
            ctx.noise.noise(i as f64 * 0.5, 0.5, 0.0),
        ));
    }
    let mut shapes: Vec<Shape> = Vec::new();
    let mut line1: Vec<Point> = Vec::new();
    let mut line2: Vec<Point> = Vec::new();
    let trunk_col = trunk.unwrap_or_else(|| ink(ctx, 0.5, depth));
    for i in 0..reso {
        let nx = x;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i >= 3 {
            let count = ((reso - i) as f64 / 5.0).ceil() as usize;
            for _j in 0..count {
                let leaf = match col {
                    Some(c) => c.with_alpha((c.a + ctx.rng.random() * 0.2).min(1.0)),
                    None => leaf(ctx, depth, 0.5),
                };
                let bx = nx + (ctx.rng.random() - 0.5) * wid * 1.2 * (reso - i) as f64;
                let by = ny + (ctx.rng.random() - 0.5) * wid;
                let len_ = ctx.rng.random() * 20.0 * (reso - i) as f64 * 0.2 + 10.0;
                let bwid = ctx.rng.random() * 6.0 + 3.0;
                let ang = ((ctx.rng.random() - 0.5) * PI) / 6.0;
                shapes.extend(blob(ctx, bx, by, len_, bwid, ang, Some(leaf), 0.5, None));
            }
        }
        line1.push((nx + (nslist[i].0 - 0.5) * wid - wid / 2.0, ny));
        line2.push((nx + (nslist[i].1 - 0.5) * wid + wid / 2.0, ny));
    }
    shapes.push(poly_line(line1, trunk_col, 1.5));
    shapes.push(poly_line(line2, trunk_col, 1.5));
    shapes
}

fn leaf_fun(t: f64) -> f64 {
    if t <= 1.0 {
        ((t * PI).sin() * t).max(0.0).sqrt()
    } else {
        -(((t - 2.0) * PI * (t - 2.0)).sin()).max(0.0).sqrt()
    }
}

pub fn tree02(
    ctx: &mut Ctx,
    x: f64,
    y: f64,
    hei: f64,
    wid: f64,
    clu: i32,
    depth: f64,
    col: Option<Color>,
) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    for _i in 0..clu {
        let base = match col {
            Some(c) => c,
            None => foliage(ctx, depth, 0.8),
        };
        let bx = x + rand_gaussian(ctx) * clu as f64 * 4.0;
        let by = y + rand_gaussian(ctx) * clu as f64 * 4.0;
        let bwid = ctx.rng.random() * wid * 0.75 + wid * 0.5;
        let len_ = ctx.rng.random() * hei * 0.75 + hei * 0.5;
        shapes.extend(blob(
            ctx,
            bx,
            by,
            len_,
            bwid,
            PI / 2.0,
            Some(base),
            0.5,
            Some(&leaf_fun),
        ));
    }
    shapes
}

pub fn tree03(
    ctx: &mut Ctx,
    x: f64,
    y: f64,
    hei: f64,
    wid: f64,
    ben: Option<&dyn Fn(f64) -> f64>,
    depth: f64,
    col: Option<Color>,
) -> Vec<Shape> {
    let zero = |_x: f64| 0.0;
    let ben: &dyn Fn(f64) -> f64 = ben.unwrap_or(&zero);
    let reso = 10usize;
    let mut nslist: Vec<(f64, f64)> = Vec::with_capacity(reso);
    for i in 0..reso {
        nslist.push((
            ctx.noise.noise(i as f64 * 0.5, 0.0, 0.0),
            ctx.noise.noise(i as f64 * 0.5, 0.5, 0.0),
        ));
    }
    let mut shapes: Vec<Shape> = Vec::new();
    let mut line1: Vec<Point> = Vec::new();
    let mut line2: Vec<Point> = Vec::new();
    for i in 0..reso {
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i >= 2 {
            for _j in 0..((reso - i) * 2) {
                let shape = (50.0 * ((reso - i) as f64 / reso as f64) + 1.0).ln() / 3.95;
                let ox = ctx.rng.random() * wid * 2.0 * shape;
                let leaf = match col {
                    Some(c) => c.with_alpha((c.a + ctx.rng.random() * 0.2).min(1.0)),
                    None => leaf(ctx, depth, 0.3),
                };
                let dir = rand_choice_i32(ctx, &[-1, 1]) as f64;
                let bx = nx + ox * dir;
                let by = ny + (ctx.rng.random() - 0.5) * wid * 2.0;
                let bwid = ctx.rng.random() * 6.0 + 3.0;
                let ang = ((ctx.rng.random() - 0.5) * PI) / 6.0;
                shapes.extend(blob(
                    ctx,
                    bx,
                    by,
                    ox * 2.0,
                    bwid,
                    ang,
                    Some(leaf),
                    0.5,
                    None,
                ));
            }
        }
        line1.push((
            nx + (((nslist[i].0 - 0.5) * wid - wid / 2.0) * (reso - i) as f64) / reso as f64,
            ny,
        ));
        line2.push((
            nx + (((nslist[i].1 - 0.5) * wid + wid / 2.0) * (reso - i) as f64) / reso as f64,
            ny,
        ));
    }
    let mut lc = line1;
    lc.extend(line2.iter().rev().copied());
    shapes.insert(
        0,
        Shape::Poly(Poly {
            pts: lc,
            fill: Some(ctx.palette.fill),
            stroke: Some(ink(ctx, 0.5, depth)),
            width: 1.5,
        }),
    );
    shapes
}

pub type Branch = (Vec<Point>, Vec<Point>);

pub fn branch(ctx: &mut Ctx, hei: f64, wid: f64, ang: f64, det: f64, ben: f64) -> Branch {
    let mut tlist: Vec<Point> = vec![(0.0, 0.0)];
    let mut a0 = 0.0;
    let g = 3usize;
    for _i in 0..g {
        let r = ctx.rng.random();
        let dir = rand_choice_f64(ctx, &[-1.0, 1.0]);
        a0 += (ben / 2.0 + (r * ben) / 2.0) * dir;
        let last = *tlist.last().unwrap();
        tlist.push((
            last.0 + (a0.cos() * hei) / g as f64,
            last.1 - (a0.sin() * hei) / g as f64,
        ));
    }

    let ta = tlist.last().unwrap().1.atan2(tlist.last().unwrap().0);
    let rotated: Vec<Point> = tlist
        .iter()
        .map(|&(vx, vy)| {
            let a = vy.atan2(vx);
            let d = vx.hypot(vy);
            (d * (a - ta + ang).cos(), d * (a - ta + ang).sin())
        })
        .collect();
    tlist = rotated;

    let span = (det as usize).max(1);
    let tl = (tlist.len() - 1) * span;
    let mut trlist1: Vec<Point> = Vec::with_capacity(tl);
    let mut trlist2: Vec<Point> = Vec::with_capacity(tl);
    let mut lx = 0.0;
    let mut ly = 0.0;
    for i in 0..tl {
        let lastp = tlist[i / span];
        let next_idx = (i / span + if i % span != 0 { 1 } else { 0 }).min(tlist.len() - 1);
        let nextp = tlist[next_idx];
        let p = (i % span) as f64 / span as f64;
        let nx = lastp.0 * (1.0 - p) + nextp.0 * p;
        let ny = lastp.1 * (1.0 - p) + nextp.1 * p;
        let la = (ny - ly).atan2(nx - lx);
        let woff = ((ctx.noise.noise(i as f64 * 0.3, 0.0, 0.0) - 0.5) * wid * hei) / 80.0;
        let b = if p == 0.0 {
            ctx.rng.random() * wid
        } else {
            0.0
        };
        let nw = wid * (((tl - i) as f64 / tl as f64) * 0.5 + 0.5);
        trlist1.push((
            nx + (la + PI / 2.0).cos() * (nw + woff + b),
            ny + (la + PI / 2.0).sin() * (nw + woff + b),
        ));
        trlist2.push((
            nx + (la - PI / 2.0).cos() * (nw - woff + b),
            ny + (la - PI / 2.0).sin() * (nw - woff + b),
        ));
        lx = nx;
        ly = ny;
    }
    (trlist1, trlist2)
}

fn twig_leaf_fun(t: f64) -> f64 {
    if t <= 1.0 {
        ((t * PI).sin() * t).max(0.0).sqrt()
    } else {
        -(((t - 2.0) * PI * (t - 2.0)).sin()).max(0.0).sqrt()
    }
}

#[allow(clippy::too_many_arguments)]
fn twig_inner(
    ctx: &mut Ctx,
    tx: f64,
    ty: f64,
    dep: i32,
    dir: i32,
    sca: f64,
    wid: f64,
    ang: f64,
    lea: (bool, f64),
    depth: f64,
) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let mut twlist: Vec<Point> = Vec::new();
    let tl = 10usize;
    let hs = ctx.rng.random() * 0.5 + 0.5;
    let tfun = |x: f64| -1.0 / (x + 1.0).powi(5) + 1.0;
    let a0 = ((ctx.rng.random() * PI) / 6.0) * dir as f64 + ang;
    let last_fun = twig_leaf_fun;

    for i in 0..tl {
        let mx = dir as f64 * tfun(i as f64 / tl as f64) * 50.0 * sca * hs;
        let my = -(i as f64) * 5.0 * sca;
        let a = my.atan2(mx);
        let d = (mx * mx + my * my).sqrt();
        let nx = (a + a0).cos() * d;
        let ny = (a + a0).sin() * d;
        twlist.push((nx + tx, ny + ty));
        if (i == tl / 3 || i == (tl * 2) / 3) && dep > 0 {
            let ndir = dir * rand_choice_i32(ctx, &[-1, 1]);
            shapes.extend(twig_inner(
                ctx,
                nx + tx,
                ny + ty,
                dep - 1,
                ndir,
                sca * 0.8,
                wid,
                ang,
                lea,
                depth,
            ));
        }
        if i == tl - 1 && lea.0 {
            for j in 0..5 {
                let dj = (j as f64 - 2.5) * 5.0;
                let bx = nx + tx + ang.cos() * dj * wid;
                let by = ny + ty + (ang.sin() * dj - lea.1 / (dep as f64 + 1.0)) * wid;
                let bwid = (6.0 + 3.0 * ctx.rng.random()) * wid;
                let len_ = (15.0 + 12.0 * ctx.rng.random()) * wid;
                let bang = ang / 2.0 + PI / 2.0 + PI * 0.2 * (ctx.rng.random() - 0.5);
                let col = ink(ctx, 0.5 + dep as f64 * 0.2, depth);
                shapes.extend(blob(
                    ctx,
                    bx,
                    by,
                    len_,
                    bwid,
                    bang,
                    Some(col),
                    0.5,
                    Some(&last_fun),
                ));
            }
        }
    }
    let cos_fun = |t: f64| ((t * PI) / 2.0).cos();
    let stroke_col = ink(ctx, 0.5, depth);
    shapes.extend(stroke(
        ctx,
        &twlist,
        0.0,
        0.0,
        1.0,
        Some(stroke_col),
        0.5,
        1.0,
        Some(&cos_fun),
    ));
    shapes
}

#[allow(clippy::too_many_arguments)]
pub fn twig(
    ctx: &mut Ctx,
    tx: f64,
    ty: f64,
    dep: i32,
    dir: i32,
    sca: f64,
    wid: f64,
    ang: f64,
    lea: (bool, f64),
    depth: f64,
) -> Vec<Shape> {
    twig_inner(ctx, tx, ty, dep, dir, sca, wid, ang, lea, depth)
}

fn bark(ctx: &mut Ctx, bx: f64, by: f64, wid: f64, ang: f64, depth: f64) -> Vec<Shape> {
    let length = 10.0 + 10.0 * ctx.rng.random();
    let noi = 0.5;
    let fun = |p: f64| {
        if p <= 1.0 {
            (p * PI).sin().max(0.0).sqrt()
        } else {
            -(((p + 1.0) * PI).sin()).max(0.0).sqrt()
        }
    };
    let reso = 20usize;
    let mut lalist: Vec<(f64, f64)> = Vec::with_capacity(reso + 1);
    for i in 0..=reso {
        let p = (i as f64 / reso as f64) * 2.0;
        let xo = length / 2.0 - (p - 1.0).abs() * length;
        let yo = (fun(p) * wid) / 2.0;
        let a = yo.atan2(xo);
        lalist.push((xo.hypot(yo), a));
    }
    let mut nslist: Vec<f64> = Vec::with_capacity(reso + 1);
    for i in 0..=reso {
        nslist.push(
            ctx.noise
                .noise(i as f64 * 0.05, ctx.rng.random() * 10.0, 0.0),
        );
    }
    loop_noise(&mut nslist);
    let mut brklist: Vec<Point> = Vec::with_capacity(reso + 1);
    for (i, &(length_i, a)) in lalist.iter().enumerate() {
        let ns = nslist[i] * noi + (1.0 - noi);
        brklist.push((
            bx + (a + ang).cos() * length_i * ns,
            by + (a + ang).sin() * length_i * ns,
        ));
    }
    let fr = ctx.rng.random();
    let fun = move |t: f64| ((t + fr) * PI * 3.0).sin();
    let bark_col = ink(ctx, 0.4, depth);
    stroke(
        ctx,
        &brklist,
        0.0,
        0.0,
        0.8,
        Some(bark_col),
        0.0,
        0.0,
        Some(&fun),
    )
}

pub fn barkify(ctx: &mut Ctx, x: f64, y: f64, trlist: &Branch, depth: f64) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    for i in 2..trlist.0.len().saturating_sub(1) {
        let a0 = (trlist.0[i].1 - trlist.0[i - 1].1).atan2(trlist.0[i].0 - trlist.0[i - 1].0);
        let a1 = (trlist.1[i].1 - trlist.1[i - 1].1).atan2(trlist.1[i].0 - trlist.1[i - 1].0);
        let p = ctx.rng.random();
        let nx = trlist.0[i].0 * (1.0 - p) + trlist.1[i].0 * p;
        let ny = trlist.0[i].1 * (1.0 - p) + trlist.1[i].1 * p;
        if ctx.rng.random() < 0.2 {
            let spot_col = ink(ctx, 0.6, depth);
            shapes.extend(blob(
                ctx,
                nx + x,
                ny + y,
                15.0,
                6.0 - (p - 0.5).abs() * 10.0,
                (a0 + a1) / 2.0,
                Some(spot_col),
                1.0,
                None,
            ));
        } else {
            shapes.extend(bark(
                ctx,
                nx + x,
                ny + y,
                5.0 - (p - 0.5).abs() * 10.0,
                (a0 + a1) / 2.0,
                depth,
            ));
        }

        if ctx.rng.random() < 0.05 {
            let jl = ctx.rng.random() * 2.0 + 2.0;
            let choice = rand_choice_i32(ctx, &[0, 1]);
            let xya = if choice == 0 {
                (trlist.0[i].0, trlist.0[i].1, a0)
            } else {
                (trlist.1[i].0, trlist.1[i].1, a1)
            };
            for j in 0..(jl.ceil() as i32) {
                let bwid = 4.0 + 6.0 * ctx.rng.random();
                let spot_col = ink(ctx, 0.6, depth);
                shapes.extend(blob(
                    ctx,
                    xya.0 + x + xya.2.cos() * (j as f64 - jl / 2.0) * 4.0,
                    xya.1 + y + xya.2.sin() * (j as f64 - jl / 2.0) * 4.0,
                    bwid,
                    4.0,
                    a0 + PI / 2.0,
                    Some(spot_col),
                    0.5,
                    None,
                ));
            }
        }
    }

    let mut trflist = trlist.0.clone();
    trflist.extend(trlist.1.iter().rev().copied());
    let mut rglist: Vec<Vec<Point>> = vec![Vec::new()];
    for v in trflist {
        if ctx.rng.random() < 0.5 {
            rglist.push(Vec::new());
        } else {
            rglist.last_mut().unwrap().push(v);
        }
    }

    for (i, group) in rglist.iter().enumerate() {
        let mut resampled = div(group, 4.0);
        for j in 0..resampled.len() {
            let mut vx = resampled[j].0;
            let mut vy = resampled[j].1;
            vx += (ctx.noise.noise(i as f64, j as f64 * 0.1, 1.0) - 0.5)
                * (15.0 + 5.0 * rand_gaussian(ctx));
            vy += (ctx.noise.noise(i as f64, j as f64 * 0.1, 2.0) - 0.5)
                * (15.0 + 5.0 * rand_gaussian(ctx));
            resampled[j] = (vx, vy);
        }
        let shifted: Vec<Point> = resampled.iter().map(|p| (p.0 + x, p.1 + y)).collect();
        let line_col = ink(ctx, 0.7, depth);
        shapes.extend(stroke(
            ctx,
            &shifted,
            0.0,
            0.0,
            1.5,
            Some(line_col),
            0.5,
            0.0,
            None,
        ));
    }
    shapes
}

pub fn tree04(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let trlist = branch(ctx, hei, wid, -PI / 2.0, 10.0, PI * 0.2);
    let mut txcanv = barkify(ctx, x, y, &trlist, depth);
    let mut trunk_points = trlist.0.clone();
    trunk_points.extend(trlist.1.iter().rev().copied());
    let mut trmlist: Vec<Point> = Vec::new();
    let mut twcanv: Vec<Shape> = Vec::new();
    let n = trunk_points.len();

    for i in 0..n {
        let point = trunk_points[i];
        let growth =
            n as f64 * 0.3 <= i as f64 && i as f64 <= n as f64 * 0.7 && ctx.rng.random() < 0.1;
        if growth || (i as f64) == n as f64 / 2.0 - 1.0 {
            let ba = PI * 0.2
                - PI * 1.4
                    * if (i as f64) > n as f64 / 2.0 {
                        1.0
                    } else {
                        0.0
                    };
            let branch_hei = hei * (ctx.rng.random() + 1.0) * 0.3;
            let brlist = branch(ctx, branch_hei, wid * 0.5, ba, 10.0, PI * 0.2);
            let br1: Vec<Point> = brlist.0.iter().skip(1).copied().collect();
            let br2: Vec<Point> = brlist.1.iter().skip(1).copied().collect();
            let foff = |v: Point| (v.0 + point.0, v.1 + point.1);
            let b1: Vec<Point> = br1.iter().map(|&v| foff(v)).collect();
            let b2: Vec<Point> = br2.iter().map(|&v| foff(v)).collect();
            txcanv.extend(barkify(ctx, x, y, &(b1, b2), depth));
            for j in 0..br1.len() {
                if ctx.rng.random() < 0.2 || j == br1.len() - 1 {
                    twcanv.extend(twig(
                        ctx,
                        br1[j].0 + point.0 + x,
                        br1[j].1 + point.1 + y,
                        1,
                        if ba > -PI / 2.0 { 1 } else { -1 },
                        (0.5 * hei) / 300.0,
                        hei / 300.0,
                        if ba > -PI / 2.0 { ba } else { ba + PI },
                        (true, 12.0),
                        depth,
                    ));
                }
            }
            for &v in &br1 {
                trmlist.push(foff(v));
            }
            for &v in br2.iter().rev() {
                trmlist.push(foff(v));
            }
        } else {
            trmlist.push(point);
        }
    }

    let mut shapes: Vec<Shape> = vec![poly_filled(
        trmlist.iter().map(|v| (v.0 + x, v.1 + y)).collect(),
        ctx.palette.fill,
    )];
    let body: Vec<Point> = trmlist
        .iter()
        .skip(1)
        .take(trmlist.len().saturating_sub(2))
        .map(|v| (v.0 + x, v.1 + y))
        .collect();
    let alpha = 0.4 + ctx.rng.random() * 0.1;
    let ink_col = ink(ctx, alpha, depth);
    let flat = |_t: f64| 1.0f64.sin();
    shapes.extend(stroke(
        ctx,
        &body,
        0.0,
        0.0,
        2.5,
        Some(ink_col),
        0.9,
        0.0,
        Some(&flat),
    ));
    shapes.extend(txcanv);
    shapes.extend(twcanv);
    shapes
}

pub fn tree05(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let trlist = branch(ctx, hei, wid, -PI / 2.0, 10.0, 0.0);
    let txcanv = barkify(ctx, x, y, &trlist, depth);
    let mut trunk_points = trlist.0.clone();
    trunk_points.extend(trlist.1.iter().rev().copied());
    let mut trmlist: Vec<Point> = Vec::new();
    let mut twcanv: Vec<Shape> = Vec::new();
    let n = trunk_points.len();

    for i in 0..n {
        let point = trunk_points[i];
        let p = (i as f64 - n as f64 * 0.5).abs() / (n as f64 * 0.5);
        let growth = n as f64 * 0.2 <= i as f64
            && i as f64 <= n as f64 * 0.8
            && i % 3 == 0
            && ctx.rng.random() > p;
        if growth || (i as f64) == n as f64 / 2.0 - 1.0 {
            let bar = ctx.rng.random() * 0.2;
            let ba = -bar * PI
                - (1.0 - bar * 2.0)
                    * PI
                    * if (i as f64) > n as f64 / 2.0 {
                        1.0
                    } else {
                        0.0
                    };
            let branch_hei = hei * (0.3 * p - ctx.rng.random() * 0.05);
            let brlist = branch(ctx, branch_hei, wid * 0.5, ba, 10.0, 0.5);
            let br1: Vec<Point> = brlist.0.iter().skip(1).copied().collect();
            let br2: Vec<Point> = brlist.1.iter().skip(1).copied().collect();
            for j in 0..br1.len() {
                if j % 20 == 0 || j == br1.len() - 1 {
                    twcanv.extend(twig(
                        ctx,
                        br1[j].0 + point.0 + x,
                        br1[j].1 + point.1 + y,
                        0,
                        if ba > -PI / 2.0 { 1 } else { -1 },
                        (0.2 * hei) / 300.0,
                        hei / 300.0,
                        if ba > -PI / 2.0 { ba } else { ba + PI },
                        (true, 5.0),
                        depth,
                    ));
                }
            }
            for &v in &br1 {
                trmlist.push((v.0 + point.0, v.1 + point.1));
            }
            for &v in br2.iter().rev() {
                trmlist.push((v.0 + point.0, v.1 + point.1));
            }
        } else {
            trmlist.push(point);
        }
    }

    let mut shapes: Vec<Shape> = vec![poly_filled(
        trmlist.iter().map(|v| (v.0 + x, v.1 + y)).collect(),
        ctx.palette.fill,
    )];
    let body: Vec<Point> = trmlist
        .iter()
        .skip(1)
        .take(trmlist.len().saturating_sub(2))
        .map(|v| (v.0 + x, v.1 + y))
        .collect();
    let alpha = 0.4 + ctx.rng.random() * 0.1;
    let ink_col = ink(ctx, alpha, depth);
    let flat = |_t: f64| 1.0f64.sin();
    shapes.extend(stroke(
        ctx,
        &body,
        0.0,
        0.0,
        2.5,
        Some(ink_col),
        0.9,
        0.0,
        Some(&flat),
    ));
    shapes.extend(txcanv);
    shapes.extend(twcanv);
    shapes
}

#[allow(clippy::too_many_arguments)]
fn frac_tree06(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    dep: i32,
    fhei: f64,
    fwid: f64,
    fang: f64,
    fben: f64,
    depth: f64,
    txcanv: &mut Vec<Shape>,
    twcanv: &mut Vec<Shape>,
) -> Vec<Point> {
    let trlist = branch(ctx, fhei, fwid, fang, fhei / 20.0, fben);
    txcanv.extend(barkify(ctx, xoff, yoff, &trlist, depth));
    let mut trunk = trlist.0.clone();
    trunk.extend(trlist.1.iter().rev().copied());
    let mut trmlist: Vec<Point> = Vec::new();
    let n = trunk.len();
    for i in 0..n {
        let point = trunk[i];
        let rnd = ctx.rng.random();
        let condition = ((rnd < 0.025 && n as f64 * 0.2 <= i as f64 && i as f64 <= n as f64 * 0.8)
            || i == n / 2 - 1
            || i == n / 2 + 1)
            && dep > 0;
        if condition {
            let bar = 0.02 + ctx.rng.random() * 0.08;
            let ba = bar * PI
                - bar
                    * 2.0
                    * PI
                    * if (i as f64) > n as f64 / 2.0 {
                        1.0
                    } else {
                        0.0
                    };
            let branch_hei = fhei * (0.7 + ctx.rng.random() * 0.2);
            let brlist = frac_tree06(
                ctx,
                point.0 + xoff,
                point.1 + yoff,
                dep - 1,
                branch_hei,
                fwid * 0.6,
                fang + ba,
                0.55,
                depth,
                txcanv,
                twcanv,
            );
            for br in &brlist {
                if ctx.rng.random() < 0.03 {
                    let tw_ang = ba * (ctx.rng.random() * 0.5 + 0.75);
                    twcanv.extend(twig(
                        ctx,
                        br.0 + point.0 + xoff,
                        br.1 + point.1 + yoff,
                        2,
                        if ba > 0.0 { 1 } else { -1 },
                        0.3,
                        1.0,
                        tw_ang,
                        (false, 0.0),
                        depth,
                    ));
                }
            }
            for &v in &brlist {
                trmlist.push((v.0 + point.0, v.1 + point.1));
            }
        } else {
            trmlist.push(point);
        }
    }
    trmlist
}

pub fn tree06(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let mut txcanv: Vec<Shape> = Vec::new();
    let mut twcanv: Vec<Shape> = Vec::new();
    let trmlist = frac_tree06(
        ctx,
        x,
        y,
        3,
        hei,
        wid,
        -PI / 2.0,
        0.0,
        depth,
        &mut txcanv,
        &mut twcanv,
    );
    let mut shapes: Vec<Shape> = vec![poly_filled(
        trmlist.iter().map(|v| (v.0 + x, v.1 + y)).collect(),
        ctx.palette.fill,
    )];
    let body: Vec<Point> = trmlist
        .iter()
        .skip(1)
        .take(trmlist.len().saturating_sub(2))
        .map(|v| (v.0 + x, v.1 + y))
        .collect();
    let alpha = 0.4 + ctx.rng.random() * 0.1;
    let ink_col = ink(ctx, alpha, depth);
    let flat = |_t: f64| 1.0f64.sin();
    shapes.extend(stroke(
        ctx,
        &body,
        0.0,
        0.0,
        2.5,
        Some(ink_col),
        0.9,
        0.0,
        Some(&flat),
    ));
    shapes.extend(txcanv);
    shapes.extend(twcanv);
    shapes
}

pub fn tree07(
    ctx: &mut Ctx,
    x: f64,
    y: f64,
    hei: f64,
    wid: f64,
    ben: Option<&dyn Fn(f64) -> f64>,
    depth: f64,
) -> Vec<Shape> {
    let default_ben = |t: f64| t.sqrt() * 0.2;
    let ben: &dyn Fn(f64) -> f64 = ben.unwrap_or(&default_ben);
    let reso = 10usize;
    let mut nslist: Vec<(f64, f64)> = Vec::with_capacity(reso);
    for i in 0..reso {
        nslist.push((
            ctx.noise.noise(i as f64 * 0.5, 0.0, 0.0),
            ctx.noise.noise(i as f64 * 0.5, 0.5, 0.0),
        ));
    }
    let mut leaves: Vec<Vec<Point>> = Vec::new();
    let mut line1: Vec<Point> = Vec::new();
    let mut line2: Vec<Point> = Vec::new();
    let canopy_fun = |t: f64| {
        if t <= 1.0 {
            2.75 * t * (1.0 - t).powf(1.0 / 1.8)
        } else {
            2.75 * (t - 2.0) * (t - 1.0).powf(1.0 / 1.8)
        }
    };

    for i in 0..reso {
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i >= 3 {
            let bx = nx + (ctx.rng.random() - 0.5) * wid * 1.2 * (reso - i) as f64 * 0.5;
            let by = ny + (ctx.rng.random() - 0.5) * wid * 0.5;
            let len_ = ctx.rng.random() * 50.0 + 20.0;
            let bwid = ctx.rng.random() * 12.0 + 12.0;
            let bang = (-ctx.rng.random() * PI) / 6.0;
            let bpl = blob_points(ctx, bx, by, len_, bwid, bang, 0.5, Some(&canopy_fun));
            leaves.extend(triangulate(&bpl, 50.0, true, false));
        }
        line1.push((nx + (nslist[i].0 - 0.5) * wid - wid / 2.0, ny));
        line2.push((nx + (nslist[i].1 - 0.5) * wid + wid / 2.0, ny));
    }

    let mut trunk_line = line1;
    trunk_line.extend(line2.iter().rev().copied());
    let mut tris = triangulate(&trunk_line, 50.0, true, true);
    tris.extend(leaves);
    let mut shapes: Vec<Shape> = Vec::new();
    for tri in tris {
        let m = mid_pt(&tri);
        let t = ctx.noise.noise(m.0 * 0.02, m.1 * 0.02, 0.0);
        let co = ctx.palette.tone(t, depth, 0.8);
        shapes.push(poly_fill_stroke(tri, co, co, 0.0));
    }
    shapes
}

#[allow(clippy::too_many_arguments)]
fn frac_tree08(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    dep: i32,
    fang: f64,
    flen: f64,
    fben: f64,
    depth: f64,
) -> Vec<Shape> {
    let fun_leaf = |t: f64| (0.5 * PI * t).cos();
    let fun_one = |_t: f64| 1.0;
    let fun: &dyn Fn(f64) -> f64 = if dep == 0 { &fun_leaf } else { &fun_one };
    let spt = (xoff, yoff);
    let ept = (xoff + fang.cos() * flen, yoff + fang.sin() * flen);
    let mut trmlist = div(&[(xoff, yoff), (xoff + flen, yoff)], 10.0);
    let sign = rand_choice_f64(ctx, &[-1.0, 1.0]);
    for i in 0..trmlist.len() {
        trmlist[i].1 += sign * (i as f64 / trmlist.len() as f64 * PI).sin() * 2.0;
    }
    for v in trmlist.iter_mut() {
        let d = distance(*v, spt);
        let a = (v.1 - spt.1).atan2(v.0 - spt.0);
        *v = (spt.0 + d * (a + fang).cos(), spt.1 + d * (a + fang).sin());
    }

    let twig_col = ink(ctx, 0.5, depth);
    let mut tcanv = stroke(
        ctx,
        &trmlist,
        0.0,
        0.0,
        0.8,
        Some(twig_col),
        0.5,
        1.0,
        Some(fun),
    );
    if dep != 0 {
        let nben = fben + rand_choice_f64(ctx, &[-1.0, 1.0]) * PI * 0.001 * (dep * dep) as f64;
        if ctx.rng.random() < 0.5 {
            let (a, b) = (norm_rand(ctx, -1.0, 0.5), norm_rand(ctx, 0.5, 1.0));
            let pick = rand_choice_f64(ctx, &[a, b]);
            let angle1 = fang + fben + PI * pick * 0.2;
            let len1 = flen * norm_rand(ctx, 0.8, 0.9);
            tcanv.extend(frac_tree08(
                ctx,
                ept.0,
                ept.1,
                dep - 1,
                angle1,
                len1,
                nben,
                depth,
            ));
            let (c, d) = (norm_rand(ctx, -1.0, -0.5), norm_rand(ctx, 0.5, 1.0));
            let pick2 = rand_choice_f64(ctx, &[c, d]);
            let angle2 = fang + fben + PI * pick2 * 0.2;
            let len2 = flen * norm_rand(ctx, 0.8, 0.9);
            tcanv.extend(frac_tree08(
                ctx,
                ept.0,
                ept.1,
                dep - 1,
                angle2,
                len2,
                nben,
                depth,
            ));
        } else {
            let len1 = flen * norm_rand(ctx, 0.8, 0.9);
            tcanv.extend(frac_tree08(
                ctx,
                ept.0,
                ept.1,
                dep - 1,
                fang + fben,
                len1,
                nben,
                depth,
            ));
        }
    }
    tcanv
}

pub fn tree08(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let ang = norm_rand(ctx, -1.0, 1.0) * PI * 0.2;
    let trlist = branch(ctx, hei, wid, -PI / 2.0 + ang, hei / 20.0, PI * 0.2);
    let mut trunk = trlist.0.clone();
    trunk.extend(trlist.1.iter().rev().copied());
    let mut twcanv: Vec<Shape> = Vec::new();

    for (i, point) in trunk.iter().enumerate() {
        if ctx.rng.random() < 0.2 {
            let dep = (4.0 * ctx.rng.random()).floor() as i32;
            let a = -PI / 2.0 - ang * ctx.rng.random();
            twcanv.extend(frac_tree08(
                ctx,
                x + point.0,
                y + point.1,
                dep,
                a,
                15.0,
                0.0,
                depth,
            ));
        } else if i == trunk.len() / 2 {
            twcanv.extend(frac_tree08(
                ctx,
                x + point.0,
                y + point.1,
                3,
                -PI / 2.0 + ang,
                15.0,
                0.0,
                depth,
            ));
        }
    }

    let mut shapes: Vec<Shape> = vec![poly_filled(
        trunk.iter().map(|v| (v.0 + x, v.1 + y)).collect(),
        ctx.palette.fill,
    )];
    let alpha = 0.6 + ctx.rng.random() * 0.1;
    let ink_col = ink(ctx, alpha, depth);
    let flat = |_t: f64| 1.0f64.sin();
    shapes.extend(stroke(
        ctx,
        &trunk.iter().map(|v| (v.0 + x, v.1 + y)).collect::<Vec<_>>(),
        0.0,
        0.0,
        2.5,
        Some(ink_col),
        0.9,
        0.0,
        Some(&flat),
    ));
    shapes.extend(twcanv);
    shapes
}

// ---------------------------------------------------------------------------
// Himalayan flora
// ---------------------------------------------------------------------------

/// A tall Himalayan cypress: a narrow spire of layered, needle-like foliage.
pub fn cypress(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let lean = (ctx.rng.random() - 0.5) * wid * 0.5;
    let trunk_col = ink(ctx, 0.52, depth);

    let mut trunk: Vec<Point> = Vec::new();
    for i in 0..=8 {
        let t = i as f64 / 8.0;
        trunk.push((x + lean * t * t, y - hei * t));
    }
    shapes.push(poly_line(trunk, trunk_col, 1.5));

    let tiers = 9usize;
    for k in 0..tiers {
        let t = k as f64 / (tiers - 1) as f64;
        let cy = y - hei * (0.08 + 0.90 * t);
        let hw = (wid * (1.0 - t).powf(0.9) * 0.55).max(1.2);
        let hh = hei * 0.062 + 1.0;
        let wobble = ctx.noise.noise(k as f64 * 0.7, hei * 0.01, 0.0) * 0.3;
        let base = foliage(ctx, depth, 0.8);
        let dark = base.darken(0.2).with_alpha(0.92);
        let lit = base.lighten(0.16).with_alpha(0.9);
        let mut pts = vec![
            (x + lean * t * t, cy - hh * 1.6),
            (x + hw * (0.55 + wobble), cy - hh * 0.5),
            (x + hw * 1.05, cy + hh * 0.25),
            (x + hw * 0.6, cy + hh * 0.58),
            (x + hw * 0.15, cy + hh * 0.3),
            (x, cy + hh * 0.78),
            (x - hw * 0.15, cy + hh * 0.3),
            (x - hw * 0.6, cy + hh * 0.58),
            (x - hw * 1.05, cy + hh * 0.25),
            (x - hw * (0.55 + wobble), cy - hh * 0.5),
        ];
        for p in pts.iter_mut() {
            p.0 += ctx.noise.noise(p.1 * 0.1, p.0 * 0.1, 2.0) * hw * 0.12;
        }
        shapes.push(poly_fill_stroke(pts, base.with_alpha(0.9), dark, 0.55));
        let edge = if k % 2 == 0 {
            vec![
                (x + hw * 0.45, cy - hh * 0.5),
                (x + hw * 0.92, cy + hh * 0.1),
                (x + hw * 0.5, cy + hh * 0.42),
            ]
        } else {
            vec![
                (x - hw * 0.45, cy - hh * 0.5),
                (x - hw * 0.92, cy + hh * 0.1),
                (x - hw * 0.5, cy + hh * 0.42),
            ]
        };
        shapes.push(poly_line(edge, lit, 0.65));
    }

    shapes.push(poly_line(
        vec![(x + lean, y - hei + 3.0), (x + lean, y - hei - 2.0)],
        trunk_col,
        0.9,
    ));
    shapes
}

/// A banyan: a broad, heavy canopy carried on a cluster of trunks, with
/// aerial roots hanging back down to the ground.
pub fn banyan(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let trunk_col = ink(ctx, 0.55, depth);
    let root_col = ink(ctx, 0.38, depth);
    let canopy_cy = y - hei * 0.68;

    // Aerial roots behind the canopy.
    for _ in 0..5 {
        let rx = x + (ctx.rng.random() - 0.5) * wid * 0.9;
        let top = canopy_cy + hei * 0.12;
        let bottom = y - ctx.rng.random() * 5.0;
        let sway = (ctx.rng.random() - 0.5) * 10.0;
        let mut pts: Vec<Point> = Vec::new();
        for j in 0..=6 {
            let t = j as f64 / 6.0;
            pts.push((
                rx + sway * t + (t * PI).sin() * 3.0,
                top + (bottom - top) * t,
            ));
        }
        shapes.push(poly_line(pts, root_col, 0.8 + ctx.rng.random() * 0.7));
    }

    // Three leaning trunks and their buttress roots.
    for k in 0..3 {
        let off = (k as f64 - 1.0) * wid * 0.14;
        let mut pts: Vec<Point> = vec![(x + off * 1.5, y)];
        for j in 1..=6 {
            let t = j as f64 / 6.0;
            let px =
                x + off * (1.5 - 1.1 * t) + ctx.noise.noise(t * 2.0, k as f64, 0.0) * wid * 0.04;
            pts.push((px, y - (y - canopy_cy) * 0.92 * t));
        }
        shapes.push(poly_line(pts, trunk_col, 2.6 + k as f64 * 0.35));
    }

    // Branches reaching out under the canopy.
    for k in 0..5 {
        let t = (k as f64 + 0.5) / 5.0;
        let dir = if k % 2 == 0 { -1.0 } else { 1.0 };
        let mut pts: Vec<Point> = vec![(x, canopy_cy + hei * 0.15)];
        for j in 1..=3 {
            let p = j as f64 / 3.0;
            pts.push((
                x + dir * wid * 0.46 * p * (0.5 + t * 0.5),
                canopy_cy + hei * (0.15 - 0.2 * p),
            ));
        }
        shapes.push(poly_line(pts, trunk_col, 1.2));
    }

    // Canopy: overlapping horizontal blobs with a dark underside.
    let clusters = 8usize;
    for k in 0..clusters {
        let t = k as f64 / (clusters - 1) as f64;
        let bx = x + (t - 0.5) * wid * 1.1 + (ctx.rng.random() - 0.5) * wid * 0.08;
        let by = canopy_cy - hei * (0.08 + 0.06 * (t * PI).sin())
            + (ctx.rng.random() - 0.5) * hei * 0.04;
        let len = wid * (0.3 + ctx.rng.random() * 0.14);
        let bwid = hei * (0.16 + ctx.rng.random() * 0.06);
        let jitter = ctx.rng.random();
        let col = foliage(ctx, depth, 0.72 + jitter * 0.12);
        shapes.extend(blob(ctx, bx, by, len, bwid, 0.0, Some(col), 0.78, None));
    }

    // A few roots in front of the trunk for depth.
    for _ in 0..3 {
        let rx = x + (ctx.rng.random() - 0.5) * wid * 0.35;
        let top = canopy_cy + hei * 0.2;
        let bottom = y - 2.0;
        let mut pts: Vec<Point> = Vec::new();
        for j in 0..=5 {
            let t = j as f64 / 5.0;
            pts.push((rx + (t * PI).sin() * 4.0, top + (bottom - top) * t));
        }
        shapes.push(poly_line(pts, root_col, 0.9));
    }
    shapes
}

/// A palm: a slender, leaning trunk with a crown of drooping fronds.
pub fn palm(ctx: &mut Ctx, x: f64, y: f64, hei: f64, wid: f64, depth: f64) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let lean = (ctx.rng.random() - 0.5) * hei * 0.26;
    let top = (x + lean, y - hei);
    let trunk_col = ink(ctx, 0.55, depth);
    let mut trunk: Vec<Point> = Vec::new();
    for i in 0..=12 {
        let t = i as f64 / 12.0;
        let px = x + lean * t.powf(1.6) + ctx.noise.noise(t * 3.0, 0.0, 0.0) * wid * 0.1 * t;
        trunk.push((px, y - hei * t));
    }
    shapes.push(poly_line(trunk.clone(), trunk_col, (wid * 0.3).max(1.4)));

    // Trunk rings.
    for i in 1..10 {
        let t = i as f64 / 10.0;
        let idx = ((t * 12.0) as usize).min(11);
        let p = trunk[idx];
        let q = trunk[(idx + 1).min(12)];
        let a = (q.1 - p.1).atan2(q.0 - p.0) + PI / 2.0;
        let hw = wid * 0.18 * (1.0 - 0.45 * t);
        shapes.push(poly_line(
            vec![
                (p.0 - a.cos() * hw, p.1 - a.sin() * hw),
                (p.0 + a.cos() * hw, p.1 + a.sin() * hw),
            ],
            ink(ctx, 0.28, depth),
            0.55,
        ));
    }

    let fronds = 8 + (ctx.rng.random() * 3.0) as usize;
    for k in 0..fronds {
        let spread = -PI + (k as f64 + 0.5) / fronds as f64 * PI;
        let droop = 0.4 + ctx.rng.random() * 0.35;
        let len = hei * (0.34 + ctx.rng.random() * 0.16);
        let mut rib: Vec<Point> = Vec::new();
        for j in 0..=7 {
            let t = j as f64 / 7.0;
            rib.push((
                top.0 + spread.cos() * len * t,
                top.1 + spread.sin() * len * t + t * t * len * droop,
            ));
        }
        shapes.push(poly_line(rib.clone(), ink(ctx, 0.45, depth), 1.0));

        let leaflet_col = leaf(ctx, depth, 0.58);
        for (j, point) in rib.iter().enumerate().skip(1) {
            let t = j as f64 / 7.0;
            let ll = len * (0.2 * (1.0 - t) + 0.035);
            let a = spread + PI / 2.0 - t * 0.5;
            for dir in [-1.0, 1.0] {
                shapes.push(poly_line(
                    vec![
                        *point,
                        (
                            point.0 + a.cos() * ll * dir,
                            point.1 + a.sin() * ll * dir + ll * 0.25,
                        ),
                    ],
                    leaflet_col,
                    0.75,
                ));
            }
        }
    }

    // A small cluster of dates under the crown.
    for k in 0..4 {
        let a = PI * (0.35 + k as f64 * 0.1);
        let px = top.0 + a.cos() * wid * 0.12 - wid * 0.12;
        let py = top.1 + wid * 0.18 + (k as f64) * 1.3;
        shapes.extend(blob(
            ctx,
            px,
            py,
            wid * 0.12,
            wid * 0.1,
            0.4,
            Some(ctx.palette.accent_at(0.8, depth)),
            0.4,
            None,
        ));
    }
    shapes
}
