const VERSION: u32 = 9;

use rand::Rng;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;

mod qr;
use qr::QR;

#[rustfmt::skip]
const MOGUS: [u8; 42] = [
    2, 2, 0, 0, 0, 2,
    2, 0, 1, 1, 1, 0,
    0, 1, 1, 0, 0, 2,
    0, 1, 1, 1, 1, 0,
    2, 0, 1, 1, 1, 0,
    2, 0, 1, 0, 1, 0,
    2, 2, 0, 0, 0, 2,
];
const MOG_WIDTH: usize = 6;
const MOG_HEIGHT: usize = 7;

// #[rustfmt::skip]
// const MOGUS: [u8; 16] = [
//     1, 1, 0, 0,
//     1, 1, 0, 0,
//     0, 0, 1, 1,
//     0, 0, 1, 1,
// ];
// const MOG_WIDTH: usize = 4;
// const MOG_HEIGHT: usize = 4;

// #[rustfmt::skip]
// const MOGUS: [u8; 30] = [
//     2, 0, 0, 0, 2,
//     0, 1, 1, 1, 0,
//     0, 0, 0, 1, 0,
//     0, 1, 1, 1, 0,
//     0, 1, 0, 1, 0,
//     2, 0, 0, 0, 2,
// ];
// const MOG_WIDTH: usize = 5;
// const MOG_HEIGHT: usize = 6;

const ORIENTATIONS: &[u8] = &[0,];
#[derive(Debug, Clone, Copy)]
struct Mode {
    rot: u8,
    flip: bool,
}
impl Mode {
    fn get_size(&self) -> (usize, usize) {
        if self.rot % 2 == 0 {
            (MOG_WIDTH, MOG_HEIGHT)
        } else {
            (MOG_HEIGHT, MOG_WIDTH)
        }
    }

    // x and y must be within range of get_size
    fn transform(&self, x: usize, y: usize) -> (usize, usize) {
        let (x, y) = match self.rot {
            0 => (x, y),
            1 => (MOG_WIDTH - y - 1, x),
            2 => (MOG_WIDTH - x - 1, MOG_HEIGHT - y - 1),
            3 => (y, MOG_HEIGHT - x - 1),
            _ => unreachable!("invalid rot"),
        };
        if self.flip {
            (MOG_WIDTH - x - 1, y)
        } else {
            (x, y)
        }
    }

    fn get_mog(&self, x: usize, y: usize) -> u8 {
        let (rx, ry) = self.transform(x, y);
        MOGUS[MOG_WIDTH * ry + rx]
    }
}

fn mog_match_c(qr: &QR, mode: Mode, x: usize, y: usize, invert: bool) -> i64 {
    let mut total = 0;
    for dx in 0..mode.get_size().0 {
        for dy in 0..mode.get_size().1 {
            let mog = mode.get_mog(dx, dy);
            let got = qr.get_at(x + dx, y + dy);

            let mismatch = match (mog, got) {
                (2, _) => false,
                (0, 0) => invert,
                (1, 1) => invert,
                (1, 0) => !invert,
                (0, 1) => !invert,
                _ => false,
            };

            if mismatch {
                total += 1;
            }
        }
    }
    total
}

fn mog_cost(qr: &QR, mode: Mode, x: usize, y: usize) -> f64 {
    let mog_wrongs = mog_match_c(qr, mode, x, y, false).min(mog_match_c(qr, mode, x, y, true));
    mog_wrongs as f64
}

const START: f64 = 0.2;
const END: f64 = 0.9;

// TODO: This can be made more efficient by storing the total mog for each sample and only recalculating the delta
fn tot_mog_cost(epoch: usize, qr: &QR, orig: &QR) -> f64 {
    let mut items = Vec::with_capacity((qr.side * qr.side * 8) as usize);

    for &rot in ORIENTATIONS {
        for flip in [false, true] {
            let mode = Mode { rot, flip };

            for x in 0..qr.side as usize - mode.get_size().0 {
                // if x % 5 != 0 { continue }
                for y in 0..qr.side as usize - mode.get_size().1 {
                    // if y % 6 != 0 { continue }
                    let cost = mog_cost(&qr, mode, x, y);
                    items.push(cost);
                }
            }
        }
    }

    items.sort_by(|x, y| x.partial_cmp(&y).unwrap());

    let falloff = END - (END - START) / (epoch as f64 / 100. + 1.);

    let mog_cost = items
        .into_iter()
        .enumerate()
        .map(|(i, x)| falloff.powf(i as f64) * x)
        .sum::<f64>();

    let mut n_change = 0.;
    for i in 0..(qr.side*qr.side) as usize {
        if qr.data[i] != orig.data[i] {
            n_change += 1.;
        }
    }

    mog_cost + n_change / (qr.side*qr.side) as f64
}

fn hl_mog(qr: &QR, threshold: f64) {
    let mut mogi = std::collections::HashSet::new();

    for &rot in ORIENTATIONS {
        for flip in [false, true] {
            let mode = Mode { rot, flip };

            for x in 0..qr.side as usize - mode.get_size().0 {
                for y in 0..qr.side as usize - mode.get_size().1 {
                    let mog = mog_cost(&qr, mode, x, y);

                    if mog < threshold {
                        for dx in 0..mode.get_size().0 {
                            for dy in 0..mode.get_size().1 {
                                mogi.insert((x + dx, y + dy));
                            }
                        }
                    }
                }
            }
        }
    }

    qr.dump_hl(|x, y| mogi.contains(&(x, y)));
}

const POP_SIZE: usize = 200;

const ERROR_LIMIT: u32 = 40;

fn main() {
    let text = if let Some(text) = std::env::args().nth(1) {
        text
    } else {
        eprintln!("Expected text as first argument");
        return;
    };

    let version = if let Some(version) = std::env::args().nth(2).and_then(|x| x.parse::<u32>().ok()) {
        version
    } else {
        eprintln!("Expected version as second argument");
        return;
    };

    let mut rng = rand::thread_rng();

    let orig_qr = QR::make(text.as_bytes(), version);
    // qr.dump_hl(|x, y| qr.is_functional(x, y));
    // qr.dump();
    // return;

    let mut population = vec![orig_qr.clone()];

    for epoch in 0.. {
        let pop: Vec<QR> = std::mem::take(&mut population);
        let mut scored: Vec<_> = pop
            .into_par_iter()
            .map(|x| (tot_mog_cost(epoch, &x, &orig_qr), x))
            .collect();

        scored.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let (score, best) = &scored[0];

        println!(
            "\x1b[1;1HEpoch {epoch:5}, best with score {score:6}, decodes to {:?}:",
            best.decode()
        );
        // best.dump();
        hl_mog(best, 1.5);
        // println!();
        // println!();
        // println!();
        // best.dump_hl(|x, y| {
        //     best.get_at(x, y) != orig_qr.get_at(x, y)
        // })

        population.push(best.clone());

        while population.len() < POP_SIZE {
            let i: usize = rng.gen_range(0..scored.len());
            let weight = rng.gen_range(0.0..0.1);
            let weighted_idx = (0.5 + i as f64 * weight) as usize;
            let (_, qr) = &scored[weighted_idx];
            let mut qr = qr.clone();

            let mut_count: i32 = rng.gen_range(1..4);

            for _ in 0..mut_count {
                let x = rng.gen_range(0..qr.side) as usize;
                let y = rng.gen_range(0..qr.side) as usize;

                if qr.is_functional(x, y) {
                    continue;
                }

                let j = x + y * qr.side as usize;

                qr.data[j] = 1 - qr.data[j];
            }

            match qr.decode() {
                Ok((x, count)) if x == text && count < ERROR_LIMIT => {
                    population.push(qr);
                }
                _ => continue,
            }
        }
    }
}
