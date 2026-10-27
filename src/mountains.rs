use crate::architecture as arch;
use crate::color::Color;
use crate::context::Ctx;
use crate::geometry::{mid_pt, triangulate};
use crate::primitives::{div, loop_noise, norm_rand, rand_choice, stroke, texture};
use crate::shapes::{Point, Poly, Shape};
use crate::trees;

fn count(limit: f64) -> usize {
    limit.ceil().max(0.0) as usize
}

fn mod2(value: usize) -> bool {
    value % 2 != 0
}

/// Lay a snow cap along the highest runs of a ridge. The ridge is a polyline
/// in world coordinates; the lower edge is broken up by noise so the snow
/// reads as separate fields and couloirs rather than a painted band.
fn snowcap(ctx: &mut Ctx, ridge: &[Point], hei: f64, depth: f64) -> Vec<Shape> {
    if ridge.len() < 6 || hei < 120.0 {
        return Vec::new();
    }
    let min_y = ridge.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_y = ridge.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    if max_y - min_y < hei * 0.12 {
        return Vec::new();
    }

    let threshold = min_y + hei * 0.2;
    // Far snow picks up the atmospheric blue so a cap seen over a nearer
    // ridge reads as a lit summit, not a white shelf.
    let far = depth < 0.25;
    let snow = if far {
        ctx.palette
            .snow_at(0.86, depth)
            .mix(ctx.palette.distant_at(0.16, depth), 0.26)
    } else {
        ctx.palette.snow_at(0.94, depth)
    };
    let shadow = ctx
        .palette
        .snow_at(0.5, depth)
        .mix(ctx.palette.distant_at(0.35, depth), 0.28)
        .with_alpha(if far { 0.38 } else { 0.7 });
    let mut shapes: Vec<Shape> = Vec::new();

    let mut i = 1usize;
    while i + 1 < ridge.len() {
        if ridge[i].1 > threshold {
            i += 1;
            continue;
        }
        let start = i;
        while i + 1 < ridge.len() && ridge[i + 1].1 <= threshold {
            i += 1;
        }
        let end = i;
        if end.saturating_sub(start) < 2 {
            i += 1;
            continue;
        }

        let mut cap: Vec<Point> = ridge[start..=end].to_vec();
        let mut lower: Vec<Point> = Vec::new();
        for k in (start..=end).rev() {
            let (px, py) = ridge[k];
            let below = (threshold - py).max(0.0);
            let jag = ctx.noise.noise(px * 0.08, py * 0.05, 7.0)
                + ctx.noise.noise(px * 0.29, py * 0.17, 11.0) * 0.45;
            let serr =
                (k as f64 * 0.9).sin() * 3.6 + ((k % 3) as f64 - 1.0) * if far { 1.2 } else { 2.0 };
            let th = if far {
                (2.0 + below * 0.4 + jag * 5.0 + serr).min(hei * 0.18)
            } else {
                (3.0 + below * 0.55 + jag * 7.0 + serr).min(hei * 0.22)
            };
            lower.push((px, py + th));
        }
        cap.extend(lower.iter().copied());
        shapes.push(Shape::Poly(Poly::new(cap, Some(snow))));
        shapes.push(Shape::Poly(Poly {
            pts: lower,
            fill: None,
            stroke: Some(shadow),
            width: 1.0,
        }));

        // Two thin snow gullies below the cap.
        for q in 1..=2 {
            let idx = start + (end.saturating_sub(start) * q) / 3;
            let (px, py) = ridge[idx];
            let bottom = py + (threshold - py).max(0.0) * 0.55 + 9.0;
            if bottom > py + 4.0 {
                let half = 1.4 + ctx.noise.noise(px * 0.1, q as f64, 0.0) * 1.4;
                shapes.push(Shape::Poly(Poly::new(
                    vec![
                        (px - half, py + 2.0),
                        (px + half, py + 2.0),
                        (px + half * 0.3, bottom),
                        (px - half * 0.3, bottom),
                    ],
                    Some(snow.with_alpha(0.72)),
                )));
            }
        }
    }
    shapes
}

/// Terraced fields stepping down the flat top of a foothill.
fn terraces(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    bounds: (f64, f64, f64, f64),
    depth: f64,
) -> Vec<Shape> {
    let (gxmin, gxmax, gymin, gymax) = bounds;
    let width = gxmax - gxmin;
    if width < 60.0 || ctx.rng.random() < 0.18 {
        return Vec::new();
    }

    let green = ctx.palette.foliage[0].mix(ctx.palette.foliage[2], 0.4);
    let gold = ctx.palette.wash_at(0.42, depth);
    let ink = ctx.palette.ink_at(0.22, depth);
    let bands = 3 + (ctx.rng.random() * 3.0) as usize;
    let step = ((gymax - gymin).max(12.0) * 0.45 + 6.0).clamp(5.0, 11.0);
    let top = gymin + 2.0;
    let mut shapes: Vec<Shape> = Vec::new();

    for k in 0..bands {
        let base_y = yoff + top + k as f64 * step;
        let mut upper: Vec<Point> = Vec::new();
        let mut lower: Vec<Point> = Vec::new();
        let n = 14usize;
        for i in 0..=n {
            let t = i as f64 / n as f64;
            let px = xoff + gxmin + width * (0.03 + 0.94 * t);
            let wob = ctx.noise.noise(px * 0.035, k as f64 * 0.6, 1.0) * step * 0.42;
            let sag = (t * std::f64::consts::PI).sin() * step * 0.16;
            upper.push((px, base_y + wob + sag));
            lower.push((px, base_y + step * 0.82 + wob * 0.7 + sag));
        }
        let fill = if k % 2 == 0 {
            ctx.palette.shade(green, depth, 0.5)
        } else {
            gold
        }
        .with_alpha(0.55);
        let mut poly = upper.clone();
        poly.extend(lower.iter().rev().copied());
        shapes.push(Shape::Poly(Poly::new(poly, Some(fill))));
        shapes.push(Shape::Poly(Poly {
            pts: upper,
            fill: None,
            stroke: Some(ink),
            width: 0.65,
        }));
    }
    shapes
}

pub fn foot(ctx: &mut Ctx, ptlist: &[Vec<Point>], xof: f64, yof: f64, depth: f64) -> Vec<Shape> {
    let mut ftlist: Vec<Vec<Point>> = Vec::new();
    let span = 10usize;
    let mut ni = 0usize;
    for i in 0..ptlist.len().saturating_sub(2) {
        if i == ni {
            let step = *rand_choice(ctx, &[1, 2]);
            ni = (ni + step).min(ptlist.len() - 1);
            ftlist.push(Vec::new());
            ftlist.push(Vec::new());
            for j in 0..count((ptlist[i].len() as f64 / 8.0).min(10.0)) {
                let a = ftlist.len() - 2;
                ftlist[a].push((
                    ptlist[i][j].0 + ctx.noise.noise(j as f64 * 0.1, i as f64, 0.0) * 10.0,
                    ptlist[i][j].1,
                ));
                let b = ftlist.len() - 1;
                ftlist[b].push((
                    ptlist[i][ptlist[i].len() - 1 - j].0
                        - ctx.noise.noise(j as f64 * 0.1, i as f64, 0.0) * 10.0,
                    ptlist[i][ptlist[i].len() - 1 - j].1,
                ));
            }
            let idx = ftlist.len() - 2;
            ftlist[idx].reverse();
            let idx = ftlist.len() - 1;
            ftlist[idx].reverse();
            for j in 0..span {
                let p = j as f64 / span as f64;
                let x1 = ptlist[i][0].0 * (1.0 - p) + ptlist[ni][0].0 * p;
                let mut y1 = ptlist[i][0].1 * (1.0 - p) + ptlist[ni][0].1 * p;
                let x2 = ptlist[i][ptlist[i].len() - 1].0 * (1.0 - p)
                    + ptlist[ni][ptlist[ni].len() - 1].0 * p;
                let mut y2 = ptlist[i][ptlist[i].len() - 1].1 * (1.0 - p)
                    + ptlist[ni][ptlist[ni].len() - 1].1 * p;
                let vib = -1.7 * (p - 1.0) * p.powf(1.0 / 5.0);
                let n = ctx.noise.noise(xof * 0.05, i as f64, 0.0) * 5.0;
                y1 += vib * 5.0 + n;
                y2 += vib * 5.0 + n;
                let a = ftlist.len() - 2;
                ftlist[a].push((x1, y1));
                let b = ftlist.len() - 1;
                ftlist[b].push((x2, y2));
            }
        }
    }

    let mut shapes: Vec<Shape> = Vec::new();
    for line in &ftlist {
        if !line.is_empty() {
            shapes.push(Shape::Poly(Poly::new(
                line.iter().map(|p| (p.0 + xof, p.1 + yof)).collect(),
                Some(ctx.palette.fill.mix(ctx.palette.rock, 0.18)),
            )));
        }
    }
    for line in &ftlist {
        if !line.is_empty() {
            let col = ctx.palette.ink_at(0.1 + ctx.rng.random() * 0.1, depth);
            let shifted: Vec<Point> = line.iter().map(|p| (p.0 + xof, p.1 + yof)).collect();
            shapes.extend(stroke(
                ctx,
                &shifted,
                0.0,
                0.0,
                1.0,
                Some(col),
                0.5,
                1.0,
                None,
            ));
        }
    }
    shapes
}

pub fn mountain(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    seed: f64,
    hei: Option<f64>,
    wid: Option<f64>,
    tex: usize,
    veg: bool,
    depth: f64,
) -> Vec<Shape> {
    let hei = hei.unwrap_or_else(|| 100.0 + ctx.rng.random() * 400.0);
    let wid = wid.unwrap_or_else(|| 400.0 + ctx.rng.random() * 200.0);
    let h = hei;
    let w = wid;
    let reso = (10usize, 50usize);
    let mut shapes: Vec<Shape> = Vec::new();

    let mut ptlist: Vec<Vec<Point>> = Vec::new();
    let mut hoff = 0.0;
    for j in 0..reso.0 {
        hoff += (ctx.rng.random() * yoff) / 100.0;
        let mut row: Vec<Point> = Vec::new();
        for i in 0..reso.1 {
            let x = (i as f64 / reso.1 as f64 - 0.5) * std::f64::consts::PI;
            let mut y = x.cos();
            y *= ctx.noise.noise(x + 10.0, j as f64 * 0.15, seed);
            let p = 1.0 - j as f64 / reso.0 as f64;
            row.push(((x / std::f64::consts::PI) * w * p, -y * h * p + hoff));
        }
        ptlist.push(row);
    }

    if ctx.vegetation {
        // Top fringe trees.
        let mut veglist: Vec<Point> = Vec::new();
        for i in 0..ptlist.len() {
            for j in 0..ptlist[i].len() {
                let y = ptlist[i][j].1;
                if i == 0
                    && ctx.noise.noise(j as f64 * 0.1, seed, 0.0).powi(3) < 0.1
                    && y.abs() / h > 0.2
                {
                    veglist.push(ptlist[i][j]);
                }
            }
        }
        for (vx, vy) in veglist {
            if ctx.rng.random() < 0.5 {
                let ht = 10.0 + ctx.rng.random() * 11.0;
                shapes.extend(trees::cypress(
                    ctx,
                    vx + xoff,
                    vy + yoff - 4.0,
                    ht,
                    (ht * 0.32).max(2.5),
                    depth,
                ));
            } else {
                shapes.extend(trees::tree02(
                    ctx,
                    vx + xoff,
                    vy + yoff - 5.0,
                    16.0,
                    8.0,
                    2,
                    depth,
                    None,
                ));
            }
        }
    }

    let mut outline = ptlist[0].clone();
    outline.push((0.0, reso.0 as f64 * 4.0));
    shapes.push(Shape::Poly(Poly::new(
        outline
            .into_iter()
            .map(|p| (p.0 + xoff, p.1 + yoff))
            .collect(),
        Some(ctx.palette.fill.mix(ctx.palette.rock, 0.18)),
    )));

    let outline_col = ctx.palette.ink_at(0.3, depth);
    let shifted: Vec<Point> = ptlist[0].iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect();
    shapes.extend(stroke(
        ctx,
        &shifted,
        0.0,
        0.0,
        3.0,
        Some(outline_col),
        1.0,
        1.0,
        None,
    ));

    shapes.extend(foot(ctx, &ptlist, xoff, yoff, depth));

    let sha = *rand_choice(ctx, &[0.0, 0.0, 0.0, 0.0, 5.0]);
    let shade = ctx.palette.ink_at(0.1, depth);
    shapes.extend(texture(
        ctx,
        &ptlist,
        xoff,
        yoff,
        tex,
        1.5,
        0.2,
        sha,
        None,
        Some(&mut |ctx: &mut Ctx, _t: f64| {
            ctx.palette.wash_at(0.12 + ctx.rng.random() * 0.22, depth)
        }),
        None,
        Some(shade),
    ));

    if h > 120.0 {
        let ridge: Vec<Point> = ptlist[0].iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect();
        shapes.extend(snowcap(ctx, &ridge, h, depth));
    }

    if ctx.vegetation {
        let mut veglist: Vec<Point> = Vec::new();
        for i in 0..ptlist.len() {
            for j in 0..ptlist[i].len() {
                let y = ptlist[i][j].1;
                if ctx
                    .noise
                    .noise(i as f64 * 0.1, j as f64 * 0.1, seed + 2.0)
                    .powi(3)
                    < 0.1
                    && y.abs() / h > 0.5
                {
                    veglist.push(ptlist[i][j]);
                }
            }
        }
        for (vx, vy) in veglist {
            shapes.extend(trees::tree02(
                ctx,
                vx + xoff,
                vy + yoff,
                16.0,
                8.0,
                5,
                depth,
                None,
            ));
        }
    }

    if veg && ctx.vegetation {
        // Middle trees with a cluster proof.
        let mut veglist: Vec<Point> = Vec::new();
        for i in 0..ptlist.len() {
            for j in 0..ptlist[i].len() {
                let ns = ctx.noise.noise(i as f64 * 0.2, j as f64 * 0.05, seed);
                if mod2(j) && ns.powi(4) < 0.012 && ptlist[i][j].1.abs() / h < 0.3 {
                    veglist.push(ptlist[i][j]);
                }
            }
        }
        for (i, (vx, vy)) in veglist.iter().enumerate() {
            let mut counter = 0;
            let mut proof = false;
            for (j, v) in veglist.iter().enumerate() {
                if i != j && (vx - v.0).powi(2) + (vy - v.1).powi(2) < 900.0 {
                    counter += 1;
                }
                if counter > 2 {
                    proof = true;
                    break;
                }
            }
            if proof {
                let ratio = (h + vy) / h;
                let hei = ratio * 70.0 * 0.3 + ctx.rng.random() * (ratio * 70.0) * 0.7;
                let wid = ctx.rng.random() * 3.0 + 1.0;
                shapes.extend(trees::tree01(
                    ctx,
                    vx + xoff,
                    vy + yoff,
                    hei,
                    wid,
                    depth,
                    None,
                    None,
                ));
            }
        }

        // Bottom-fringe trees.
        let mut veglist: Vec<Point> = Vec::new();
        for i in 0..ptlist.len() {
            for j in 0..ptlist[i].len() {
                let ns = ctx.noise.noise(i as f64 * 0.2, j as f64 * 0.05, seed);
                if (j == 0 || j == ptlist[i].len() - 1) && ns.powi(4) < 0.012 {
                    veglist.push(ptlist[i][j]);
                }
            }
        }
        for (vx, vy) in veglist {
            let ratio = (h + vy) / h;
            let mut ht = ratio * 120.0;
            ht = ht * 0.5 + ctx.rng.random() * ht * 0.5;
            if ctx.rng.random() < 0.32 {
                shapes.extend(trees::palm(
                    ctx,
                    vx + xoff,
                    vy + yoff,
                    ht * 0.82,
                    (ht * 0.055).max(3.0),
                    depth,
                ));
            } else {
                let bc = ctx.rng.random() * 0.1;
                let ben = move |t: f64| t * bc;
                shapes.extend(trees::tree03(
                    ctx,
                    vx + xoff,
                    vy + yoff,
                    ht,
                    5.0,
                    Some(&ben),
                    depth,
                    None,
                ));
            }
        }
    }

    // Small buildings at the foot.
    let mut veglist: Vec<Point> = Vec::new();
    for i in 0..ptlist.len() {
        for j in 0..ptlist[i].len() {
            let ns = ctx
                .noise
                .noise(i as f64 * 0.2, j as f64 * 0.05, seed + 10.0);
            if i != 0 && (j == 1 || j == ptlist[i].len() - 2) && ns.powi(4) < 0.008 {
                veglist.push(ptlist[i][j]);
            }
        }
    }
    for (vx, vy) in veglist {
        if ctx.rng.random() >= ctx.architecture {
            continue;
        }
        let tt = *rand_choice(ctx, &[0, 0, 1, 1, 1, 2]);
        if tt == 1 {
            let wid = norm_rand(ctx, 60.0, 110.0);
            let hei = norm_rand(ctx, 34.0, 58.0);
            shapes.extend(arch::village_house(
                ctx,
                vx + xoff,
                vy + yoff,
                hei,
                wid,
                depth,
            ));
        } else if tt == 2 {
            let hei = norm_rand(ctx, 55.0, 85.0);
            let wid = norm_rand(ctx, 80.0, 120.0);
            shapes.extend(arch::haveli(ctx, vx + xoff, vy + yoff, hei, wid, depth));
        }
    }

    // A shikhara temple or a domed chhatri on the first interior ridge.
    let mut veglist: Vec<Point> = Vec::new();
    for i in 0..ptlist.len() {
        for j in 0..ptlist[i].len() {
            if i == 1
                && (j as f64 - ptlist[i].len() as f64 / 2.0).abs() < 1.0
                && ptlist[i][j].1 + yoff > 200.0
                && ctx.rng.random() < 0.1
            {
                veglist.push(ptlist[i][j]);
            }
        }
    }
    for (vx, vy) in veglist {
        if ctx.rng.random() < ctx.architecture {
            if ctx.rng.random() < 0.64 {
                let wid = 38.0 + ctx.rng.random() * 22.0;
                let hei = wid * (1.9 + ctx.rng.random() * 0.5);
                shapes.extend(arch::shikhara(ctx, vx + xoff, vy + yoff, hei, wid, depth));
            } else {
                let hei = 66.0 + ctx.rng.random() * 44.0;
                let wid = 42.0 + ctx.rng.random() * 30.0;
                shapes.extend(arch::chhatri(ctx, vx + xoff, vy + yoff, hei, wid, depth));
            }
        }
    }

    // Hill shrines and havelis along the outer rows.
    let mut veglist: Vec<Point> = Vec::new();
    for i in 0..ptlist.len() {
        for j in 0..ptlist[i].len() {
            if i % 2 == 0
                && i > 0
                && ptlist[i][j].1 + yoff > 130.0
                && (j == 1 || j == ptlist[i].len() - 2)
                && ctx
                    .noise
                    .noise(
                        i as f64 * 0.2,
                        j as f64 * 0.05,
                        seed + 20.0 * std::f64::consts::PI,
                    )
                    .powi(4)
                    < 0.005
            {
                veglist.push(ptlist[i][j]);
            }
        }
    }
    for (vx, vy) in veglist {
        if ctx.rng.random() < ctx.architecture {
            if ctx.rng.random() < 0.55 {
                let hei = 42.0 + ctx.rng.random() * 30.0;
                let wid = 28.0 + ctx.rng.random() * 20.0;
                shapes.extend(arch::chhatri(ctx, vx + xoff, vy + yoff, hei, wid, depth));
            } else {
                let hei = 52.0 + ctx.rng.random() * 28.0;
                let wid = 72.0 + ctx.rng.random() * 42.0;
                shapes.extend(arch::haveli(ctx, vx + xoff, vy + yoff, hei, wid, depth));
            }
        }
    }

    // Rocks along the base.
    let mut veglist: Vec<Point> = Vec::new();
    for i in 0..ptlist.len() {
        for j in 0..ptlist[i].len() {
            if (j == 0 || j == ptlist[i].len() - 1) && ctx.rng.random() < 0.1 {
                veglist.push(ptlist[i][j]);
            }
        }
    }
    for (vx, vy) in veglist {
        let wid = 20.0 + ctx.rng.random() * 20.0;
        let hei = 20.0 + ctx.rng.random() * 20.0;
        shapes.extend(rock(
            ctx,
            vx + xoff,
            vy + yoff,
            seed,
            hei,
            wid,
            40,
            2.0,
            depth,
        ));
    }

    shapes
}

pub fn flat_mount(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    seed: f64,
    hei: Option<f64>,
    wid: Option<f64>,
    tex: usize,
    cho: f64,
    depth: f64,
) -> Vec<Shape> {
    let hei = hei.unwrap_or_else(|| 40.0 + ctx.rng.random() * 400.0);
    let wid = wid.unwrap_or_else(|| 400.0 + ctx.rng.random() * 200.0);
    let mut shapes: Vec<Shape> = Vec::new();
    let mut ptlist: Vec<Vec<Point>> = Vec::new();
    let reso = (5usize, 50usize);
    let mut hoff = 0.0;
    let mut flat: Vec<Vec<Point>> = Vec::new();

    for j in 0..reso.0 {
        hoff += (ctx.rng.random() * yoff) / 100.0;
        ptlist.push(Vec::new());
        flat.push(Vec::new());
        for i in 0..reso.1 {
            let x = (i as f64 / reso.1 as f64 - 0.5) * std::f64::consts::PI;
            let mut y = (x * 2.0).cos() + 1.0;
            y *= ctx.noise.noise(x + 10.0, j as f64 * 0.1, seed);
            let p = 1.0 - (j as f64 / reso.0 as f64) * 0.6;
            let nx = (x / std::f64::consts::PI) * wid * p;
            let mut ny = -y * hei * p + hoff;
            let h = 100.0;
            if ny < -h * cho + hoff {
                ny = -h * cho + hoff;
                if flat[flat.len() - 1].len() % 2 == 0 {
                    let last = flat.len() - 1;
                    flat[last].push((nx, ny));
                }
            } else if flat[flat.len() - 1].len() % 2 == 1 {
                let last = flat.len() - 1;
                let p = ptlist[last][ptlist[last].len() - 1];
                flat[last].push(p);
            }
            let last = ptlist.len() - 1;
            ptlist[last].push((nx, ny));
        }
    }

    let mut outline = ptlist[0].clone();
    outline.push((0.0, reso.0 as f64 * 4.0));
    shapes.push(Shape::Poly(Poly::new(
        outline
            .into_iter()
            .map(|p| (p.0 + xoff, p.1 + yoff))
            .collect(),
        Some(ctx.palette.fill.mix(ctx.palette.rock, 0.18)),
    )));

    let col = ctx.palette.ink_at(0.3, depth);
    let shifted: Vec<Point> = ptlist[0].iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect();
    shapes.extend(stroke(
        ctx,
        &shifted,
        0.0,
        0.0,
        3.0,
        Some(col),
        1.0,
        1.0,
        None,
    ));

    let shade = ctx.palette.ink_at(0.1, depth);
    shapes.extend(texture(
        ctx,
        &ptlist,
        xoff,
        yoff,
        tex,
        2.0,
        0.2,
        0.0,
        None,
        Some(&mut |ctx: &mut Ctx, _t: f64| {
            ctx.palette.wash_at(0.12 + ctx.rng.random() * 0.25, depth)
        }),
        Some(&mut |ctx: &mut Ctx| {
            if ctx.rng.random() > 0.5 {
                0.1 + 0.4 * ctx.rng.random()
            } else {
                0.9 - 0.4 * ctx.rng.random()
            }
        }),
        Some(shade),
    ));

    let mut grlist1: Vec<Point> = Vec::new();
    let mut grlist2: Vec<Point> = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        if flat[i].len() >= 2 {
            grlist1.push(flat[i][0]);
            grlist2.push(flat[i][flat[i].len() - 1]);
        }
        i += 2;
    }

    if grlist1.is_empty() {
        return shapes;
    }

    let wb = (grlist1[0].0, grlist2[0].0);
    for i in 0..3 {
        let p = 0.8 - i as f64 * 0.2;
        let y1 = grlist1[0].1 - 5.0;
        let y2 = grlist2[0].1 - 5.0;
        grlist1.insert(0, (wb.0 * p, y1));
        grlist2.insert(0, (wb.1 * p, y2));
    }
    let wb = (grlist1[grlist1.len() - 1].0, grlist2[grlist2.len() - 1].0);
    for i in 0..3 {
        let p = 0.6 - (i * i) as f64 * 0.1;
        let y1 = grlist1[grlist1.len() - 1].1 + 1.0;
        let y2 = grlist2[grlist2.len() - 1].1 + 1.0;
        grlist1.push((wb.0 * p, y1));
        grlist2.push((wb.1 * p, y2));
    }

    let d = 5;
    grlist1 = div(&grlist1, d as f64);
    grlist2 = div(&grlist2, d as f64);
    let rev1: Vec<Point> = grlist1.iter().rev().copied().collect();
    let mut grlist: Vec<Point> = rev1.clone();
    grlist.extend(grlist2);
    grlist.push(rev1[0]);

    for i in 0..grlist.len() {
        let v = (1.0 - ((i % d as usize) as f64 - d as f64 / 2.0).abs() / (d as f64 / 2.0)) * 0.12;
        grlist[i] = (
            grlist[i].0 * (1.0 - v + ctx.noise.noise(grlist[i].1 * 0.5, 0.0, 0.0) * v),
            grlist[i].1,
        );
    }

    shapes.push(Shape::Poly(Poly::new(
        grlist.iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect(),
        Some(ctx.palette.fill.mix(ctx.palette.rock, 0.18)),
    )));
    let col = ctx.palette.ink_at(0.2, depth);
    let shifted: Vec<Point> = grlist.iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect();
    shapes.extend(stroke(
        ctx,
        &shifted,
        0.0,
        0.0,
        3.0,
        Some(col),
        0.5,
        1.0,
        None,
    ));

    let xmin = grlist.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let xmax = grlist.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let ymin = grlist.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let ymax = grlist.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    shapes.extend(flat_dec(ctx, xoff, yoff, (xmin, xmax, ymin, ymax), depth));
    shapes
}

pub fn flat_dec(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    bounds: (f64, f64, f64, f64),
    depth: f64,
) -> Vec<Shape> {
    let (gxmin, gxmax, gymin, gymax) = bounds;
    let mut shapes: Vec<Shape> = Vec::new();
    let tt = *rand_choice(ctx, &[0, 0, 1, 2, 3, 4]);
    let mid_y = (gymin + gymax) / 2.0;

    shapes.extend(terraces(ctx, xoff, yoff, bounds, depth));

    for _ in 0..count(ctx.rng.random() * 5.0) {
        let x = xoff + norm_rand(ctx, gxmin, gxmax);
        let y = yoff + mid_y + norm_rand(ctx, -10.0, 10.0) + 10.0;
        let seed = ctx.rng.random() * 100.0;
        let wid = 10.0 + ctx.rng.random() * 20.0;
        let hei = 10.0 + ctx.rng.random() * 20.0;
        shapes.extend(rock(ctx, x, y, seed, hei, wid, 40, 2.0, depth));
    }
    for _ in 0..(*rand_choice(ctx, &[0, 0, 1, 2]) as usize) {
        let xr = xoff + norm_rand(ctx, gxmin, gxmax);
        let yr = yoff + mid_y + norm_rand(ctx, -5.0, 5.0) + 20.0;
        for _ in 0..count(2.0 + ctx.rng.random() * 3.0) {
            let nx = norm_rand(ctx, -30.0, 30.0).clamp(gxmin, gxmax);
            let hei = 60.0 + ctx.rng.random() * 40.0;
            shapes.extend(trees::tree08(ctx, xr + nx, yr, hei, 1.0, depth));
        }
    }

    match tt {
        0 => {
            for _ in 0..count(ctx.rng.random() * 3.0) {
                let x = xoff + norm_rand(ctx, gxmin, gxmax);
                let y = yoff + mid_y + norm_rand(ctx, -5.0, 5.0) + 20.0;
                let seed = ctx.rng.random() * 100.0;
                let wid = 50.0 + ctx.rng.random() * 20.0;
                let hei = 40.0 + ctx.rng.random() * 20.0;
                shapes.extend(rock(ctx, x, y, seed, hei, wid, 40, 5.0, depth));
            }
        }
        1 => {
            let pmin = ctx.rng.random() * 0.5;
            let pmax = ctx.rng.random() * 0.5 + 0.5;
            let xmin = gxmin * (1.0 - pmin) + gxmax * pmin;
            let xmax = gxmin * (1.0 - pmax) + gxmax * pmax;
            let mut i = xmin;
            while i < xmax {
                let x = xoff + i + 20.0 * norm_rand(ctx, -1.0, 1.0);
                let y = yoff + mid_y + 20.0;
                let hei = 100.0 + ctx.rng.random() * 200.0;
                if ctx.rng.random() < 0.62 {
                    shapes.extend(trees::cypress(
                        ctx,
                        x,
                        y,
                        hei * 0.46,
                        (hei * 0.06).max(4.0),
                        depth,
                    ));
                } else {
                    shapes.extend(trees::tree05(ctx, x, y, hei, 5.0, depth));
                }
                i += 30.0;
            }
            for _ in 0..count(ctx.rng.random() * 4.0) {
                let x = xoff + norm_rand(ctx, gxmin, gxmax);
                let y = yoff + mid_y + norm_rand(ctx, -5.0, 5.0) + 20.0;
                let seed = ctx.rng.random() * 100.0;
                let wid = 50.0 + ctx.rng.random() * 20.0;
                let hei = 40.0 + ctx.rng.random() * 20.0;
                shapes.extend(rock(ctx, x, y, seed, hei, wid, 40, 5.0, depth));
            }
        }
        2 => {
            for i in 0..(*rand_choice(ctx, &[1, 1, 1, 1, 2, 2, 3]) as usize) {
                let xr = norm_rand(ctx, gxmin, gxmax);
                let yr = mid_y;
                if ctx.rng.random() < 0.5 {
                    let hei = 150.0 + ctx.rng.random() * 110.0;
                    let wid = 90.0 + ctx.rng.random() * 80.0;
                    shapes.extend(trees::banyan(
                        ctx,
                        xoff + xr,
                        yoff + yr + 20.0,
                        hei,
                        wid,
                        depth,
                    ));
                } else {
                    shapes.extend(trees::tree04(
                        ctx,
                        xoff + xr,
                        yoff + yr + 20.0,
                        300.0,
                        6.0,
                        depth,
                    ));
                }
                for j in 0..count(ctx.rng.random() * 2.0) {
                    let cx = (xr + norm_rand(ctx, -50.0, 50.0)).clamp(gxmin, gxmax);
                    let ry = yoff + yr + norm_rand(ctx, -5.0, 5.0) + 20.0;
                    let seed = (j * i) as f64 * ctx.rng.random() * 100.0;
                    let wid = 50.0 + ctx.rng.random() * 20.0;
                    let hei = 40.0 + ctx.rng.random() * 20.0;
                    shapes.extend(rock(ctx, xoff + cx, ry, seed, hei, wid, 40, 5.0, depth));
                }
            }
        }
        3 => {
            for _ in 0..(*rand_choice(ctx, &[1, 1, 1, 1, 2, 2, 3]) as usize) {
                let x = xoff + norm_rand(ctx, gxmin, gxmax);
                let y = yoff + mid_y;
                let hei = 60.0 + ctx.rng.random() * 60.0;
                if ctx.rng.random() < 0.38 {
                    shapes.extend(trees::palm(ctx, x, y, hei * 1.2, 6.0, depth));
                } else {
                    shapes.extend(trees::tree06(ctx, x, y, hei, 6.0, depth));
                }
            }
        }
        4 => {
            let pmin = ctx.rng.random() * 0.5;
            let pmax = ctx.rng.random() * 0.5 + 0.5;
            let xmin = gxmin * (1.0 - pmin) + gxmax * pmin;
            let xmax = gxmin * (1.0 - pmax) + gxmax * pmax;
            let mut i = xmin;
            while i < xmax {
                let x = xoff + i + 20.0 * norm_rand(ctx, -1.0, 1.0);
                let y = yoff + mid_y + norm_rand(ctx, -1.0, 1.0);
                let hei = norm_rand(ctx, 40.0, 80.0);
                if ctx.rng.random() < 0.55 {
                    shapes.extend(trees::palm(ctx, x, y, hei, 4.5, depth));
                } else {
                    shapes.extend(trees::tree07(ctx, x, y, hei, 4.0, None, depth));
                }
                i += 20.0;
            }
        }
        _ => {}
    }

    if ctx.people > 0.0 && ctx.rng.random() < 0.45 * ctx.people {
        let herd = if ctx.rng.random() < 0.35 { 2 } else { 1 };
        for k in 0..herd {
            let ex = xoff + norm_rand(ctx, gxmin + 40.0, gxmax - 40.0) + k as f64 * 42.0;
            let ey = yoff + mid_y + 21.0;
            let scale = 0.55 + ctx.rng.random() * 0.4;
            let fli = ctx.rng.random() < 0.5;
            shapes.extend(arch::elephant(ctx, ex, ey, scale, fli, depth));
        }
    }

    for _ in 0..count(50.0 * ctx.rng.random()) {
        let x = xoff + norm_rand(ctx, gxmin, gxmax);
        let y = yoff + norm_rand(ctx, gymin, gymax);
        shapes.extend(trees::tree02(ctx, x, y, 16.0, 8.0, 5, depth, None));
    }

    let ts = *rand_choice(ctx, &[0, 0, 0, 0, 1]);
    if ts == 1 && tt != 4 {
        let x = xoff + norm_rand(ctx, gxmin, gxmax);
        let y = yoff + mid_y + 20.0;
        let wid = norm_rand(ctx, 110.0, 170.0);
        let hei = norm_rand(ctx, 42.0, 64.0);
        shapes.extend(arch::village_house(ctx, x, y, hei, wid, depth));
    }
    shapes
}

pub fn dist_mount(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    seed: f64,
    hei: Option<f64>,
    len_: f64,
    seg: i32,
    depth: f64,
) -> Vec<Shape> {
    let hei = hei.unwrap_or(300.0);
    let mut shapes: Vec<Shape> = Vec::new();
    let span = 10.0;
    let n = count(len_ / span).max(8);
    let _ = seg;

    // A handful of sharp peaks, each with its own height and width.
    let peak_count = 3 + (ctx.rng.random() * 3.0) as usize;
    let mut peaks: Vec<(f64, f64, f64)> = Vec::new();
    for k in 0..peak_count {
        let t = (k as f64 + 1.0) / (peak_count as f64 + 1.0);
        peaks.push((
            (t + (ctx.rng.random() - 0.5) * 0.08).clamp(0.05, 0.95),
            0.55 + ctx.rng.random() * 0.45,
            0.09 + ctx.rng.random() * 0.07,
        ));
    }

    let mut ridge: Vec<Point> = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let u = i as f64 / n as f64;
        let x = xoff + i as f64 * span;
        let envelope = (std::f64::consts::PI * u).sin().powf(0.45);
        let base = ctx.noise.noise(i as f64 * 0.21, seed, 0.0) * 0.45 + 0.5;
        let mut peak = 0.0f64;
        for (pc, ph, pw) in peaks.iter() {
            let d = (u - pc).abs() / pw;
            if d < 1.0 {
                peak = peak.max(ph * (1.0 - d).powf(1.35));
            }
        }
        let serr = (ctx.noise.noise(i as f64 * 1.35, seed + 5.0, 0.0) - 0.5) * 0.16;
        let h = (hei * envelope * (0.28 + base * 0.45 + peak + serr)).clamp(hei * 0.08, hei * 1.05);
        ridge.push((x, yoff - h));
    }

    // Sharpen the highest points into little horns.
    for (pc, _, _) in peaks.iter() {
        let idx = ((pc * n as f64) as usize).clamp(1, n.saturating_sub(1));
        if ridge[idx].1 > ridge[idx - 1].1 {
            ridge[idx].1 = ridge[idx - 1].1 - hei * 0.04;
        }
        if ridge[idx].1 > ridge[idx + 1].1 {
            ridge[idx].1 = ridge[idx + 1].1 - hei * 0.03;
        }
    }

    // Body: fill down to the valley floor and facet it with triangles.
    let left = ridge[0];
    let right = ridge[n];
    let mut body = ridge.clone();
    body.push((right.0, yoff + hei * 0.24));
    body.push((left.0, yoff + hei * 0.24));
    let far_ink = ctx.palette.ink_at(0.55, depth);
    let base_col = ctx.palette.distant_at(0.02, depth).mix(far_ink, 0.12);
    shapes.push(Shape::Poly(Poly::new(body, Some(base_col))));

    let mut facet: Vec<Point> = ridge.iter().step_by(4).copied().collect();
    if let Some(last) = ridge.last() {
        if facet.last() != Some(last) {
            facet.push(*last);
        }
    }
    facet.push((right.0, yoff + hei * 0.24));
    facet.push((left.0, yoff + hei * 0.24));
    let tris = triangulate(&facet, hei * hei * 0.5, true, false);
    for tri in tris {
        let m = mid_pt(&tri);
        let shade = ctx.noise.noise(m.0 * 0.03, m.1 * 0.04, yoff);
        let co = ctx.palette.distant_at(0.08 + shade * 0.5, depth);
        shapes.push(Shape::Poly(Poly::new(tri, Some(co.with_alpha(0.55)))));
    }

    // Snow above the snowline, a defined ridge, dark ribs, and a nearer
    // foothill strip.
    shapes.extend(snowcap(ctx, &ridge, hei, depth));
    shapes.push(Shape::Poly(Poly {
        pts: ridge.clone(),
        fill: None,
        stroke: Some(base_col.mix(far_ink, 0.35).with_alpha(0.5)),
        width: 0.8,
    }));
    for (pc, _, _) in peaks.iter() {
        let idx = ((pc * n as f64) as usize).clamp(1, n.saturating_sub(1));
        let top = ridge[idx];
        let mut pts = vec![top];
        for j in 1..=4 {
            let t = j as f64 / 4.0;
            pts.push((
                top.0 + (xoff + len_ * 0.5 - top.0) * 0.08 * t,
                top.1 + (yoff + hei * 0.2 - top.1) * t,
            ));
        }
        shapes.push(Shape::Poly(Poly {
            pts,
            fill: None,
            stroke: Some(ctx.palette.distant_at(0.05, depth).with_alpha(0.3)),
            width: 0.7,
        }));
    }

    let mut foot: Vec<Point> = Vec::new();
    for i in 0..=n {
        let u = i as f64 / n as f64;
        let x = xoff + i as f64 * span;
        let h = hei
            * (0.08
                + 0.12 * (1.0 - (2.0 * u - 1.0).abs())
                + ctx.noise.noise(i as f64 * 0.18, seed + 9.0, 0.0) * 0.1);
        foot.push((x, yoff + hei * 0.16 - h));
    }
    let mut foot_body = foot;
    foot_body.push((xoff + len_, yoff + hei * 0.3));
    foot_body.push((xoff, yoff + hei * 0.3));
    shapes.push(Shape::Poly(Poly::new(
        foot_body,
        Some(ctx.palette.distant_at(0.62, depth).with_alpha(0.85)),
    )));
    shapes
}

pub fn rock(
    ctx: &mut Ctx,
    xoff: f64,
    yoff: f64,
    seed: f64,
    hei: f64,
    wid: f64,
    tex: usize,
    sha: f64,
    depth: f64,
) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let reso = (10usize, 50usize);
    let mut ptlist: Vec<Vec<Point>> = Vec::new();

    for i in 0..reso.0 {
        let mut row: Vec<Point> = Vec::new();
        let mut nslist: Vec<f64> = Vec::new();
        for j in 0..reso.1 {
            nslist.push(ctx.noise.noise(i as f64, j as f64 * 0.2, seed));
        }
        loop_noise(&mut nslist);
        for j in 0..reso.1 {
            let a = (j as f64 / reso.1 as f64) * std::f64::consts::PI * 2.0
                - std::f64::consts::PI / 2.0;
            let mut l = (wid * hei) / ((hei * a.cos()).powi(2) + (wid * a.sin()).powi(2)).sqrt();
            l *= 0.7 + 0.3 * nslist[j];
            let p = 1.0 - i as f64 / reso.0 as f64;
            let nx = a.cos() * l * p;
            let mut ny = -a.sin() * l * p;
            if std::f64::consts::PI < a || a < 0.0 {
                ny *= 0.2;
            }
            ny += hei * (i as f64 / reso.0 as f64) * 0.2;
            row.push((nx, ny));
        }
        ptlist.push(row);
    }

    let mut outline = ptlist[0].clone();
    outline.push((0.0, 0.0));
    shapes.push(Shape::Poly(Poly::new(
        outline
            .into_iter()
            .map(|p| (p.0 + xoff, p.1 + yoff))
            .collect(),
        Some(ctx.palette.fill.mix(ctx.palette.rock, 0.18)),
    )));

    let outline_col = ctx.palette.ink_at(0.3, depth);
    let shifted: Vec<Point> = ptlist[0].iter().map(|p| (p.0 + xoff, p.1 + yoff)).collect();
    shapes.extend(stroke(
        ctx,
        &shifted,
        0.0,
        0.0,
        3.0,
        Some(outline_col),
        1.0,
        1.0,
        None,
    ));

    let shade = ctx.palette.ink_at(0.1, depth);
    shapes.extend(texture(
        ctx,
        &ptlist,
        xoff,
        yoff,
        tex,
        3.0,
        0.2,
        sha,
        None,
        Some(&mut |ctx: &mut Ctx, _t: f64| {
            ctx.palette.rock_at(0.3 + ctx.rng.random() * 0.3, depth)
        }),
        Some(&mut |ctx: &mut Ctx| {
            if ctx.rng.random() > 0.5 {
                0.15 + 0.15 * ctx.rng.random()
            } else {
                0.85 - 0.15 * ctx.rng.random()
            }
        }),
        Some(shade),
    ));
    shapes
}

#[allow(dead_code)]
fn _color_helper(c: Color) -> Color {
    c
}
