use std::collections::HashMap;

const VERSION: qrcode::Version = qrcode::Version::Normal(12);
const EC: qrcode::EcLevel = qrcode::EcLevel::L;

mod qr;
use qr::QR;

use qrcode::{cast::As, optimize::Parser, Color};

fn fake_code(real_data: &[u8], fake_data: &[u8]) -> qrcode::QrCode {
    let mut bits = qrcode::bits::Bits::new(VERSION);

    // let actual_segments = Parser::new(real_data).optimize(bits.version);
    // bits.push_segments(real_data, actual_segments).unwrap();
    bits.push_byte_data(real_data).unwrap();

    bits.push_number(4, 0); // terminator
    for &x in fake_data {
        bits.push_number(8, x as u16);
    }

    bits.push_terminator(EC).unwrap();

    let code = {
        let version = bits.version();
        let data = bits.into_bytes();
        let (encoded_data, ec_data) = qrcode::ec::construct_codewords(&*data, version, EC).unwrap();

        // ec_data.truncate(0);

        let mut canvas = qrcode::canvas::Canvas::new(version, EC);
        canvas.draw_all_functional_patterns();
        canvas.draw_data(&*encoded_data, &*ec_data);

        canvas.apply_mask(qrcode::canvas::MaskPattern::Checkerboard);

        qrcode::QrCode { content: canvas.into_colors(), version, ec_level: EC, width: version.width().as_usize() }
    };

    code
}

fn map_positions(real_data: &[u8], fake_len_bytes: usize) -> HashMap<usize, (usize, usize, Color)> {
    let fake = &std::iter::repeat(b'\x00').take(fake_len_bytes).collect::<Vec<_>>();

    let original_code = fake_code(real_data, fake);

    let mut map = HashMap::new();

    for byte in 0..fake_len_bytes {
        for bit in 0..8 {
            let mut diff = fake.clone();
            diff[byte] ^= 1 << bit;
            let new_code = fake_code(real_data, &diff);

            let mut last = None;
            for x in 0..new_code.width() {
                for y in 0..new_code.width() {
                    let i = x + y * new_code.width();
                    let ocol = original_code.to_colors()[i];
                    let ncol = new_code.to_colors()[i];
                    if ocol != ncol {
                        last = Some((x, y, ocol));
                    }
                }
            }
            if let Some(res) = last {
                map.insert(byte * 8 + bit, res);
            }
        }
    }

    map
}

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

fn get_mog_at(x: usize, y: usize) -> bool {
    return MOGUS[(x % MOG_WIDTH) + (y % MOG_HEIGHT) * MOG_WIDTH] == 1;
}

fn main() {
    let real_data = b"https://youtu.be/T59N3DPrvac";

    let bits = qrcode::bits::Bits::new(VERSION);
    let fake_len_bits = bits.max_len(EC).unwrap() - real_data.len() * 8;

    println!("{:?}", fake_len_bits);

    let fake_len_bytes = fake_len_bits / 8 - 5;

    let positions = map_positions(real_data, fake_len_bytes);

    let fake = &std::iter::repeat(b'\x00').take(fake_len_bytes).collect::<Vec<_>>();

    let code = fake_code(real_data, fake);
    let qr = QR::from_code(code);

    qr.dump_hl(|x, y| {
        for &(px, py, _c) in positions.values() {
            if x == px && y == py {
                return true;
            }
        }
        false
    });

    let mut nfake = std::iter::repeat(0u8).take(fake_len_bytes).collect::<Vec<_>>();
    for (bitidx, &(x, y, c)) in positions.iter() {
        let byte = bitidx / 8;
        let bit = bitidx % 8;
        if (c == Color::Light) ^ get_mog_at(x, y) {
            nfake[byte] ^= 1 << bit;
        }
    }

    let code = fake_code(real_data, &nfake);
    let qrm = QR::from_code(code);
    // qrm.dump_hl(|x, y| {
    //     for &(px, py, _c) in positions.values() {
    //         if x == px && y == py {
    //             return true;
    //         }
    //     }
    //     false
    // });
    qrm.dump();
}

// fn main() {
//     let qa = QR::make(b"\x55\x55\x55\x55", VERSION);
//     let qb = QR::make(b"\x54\x54\x54\x54", VERSION);

//     println!("A = 00");
//     qa.dump();

//     println!("B = ff");
//     qb.dump();

//     println!("diff");

//     hl_diff(&qa, &qb);
// }

fn hl_diff(qa: &QR, qb: &QR) {
    qa.dump_hl(|x, y| {
        qa.get_at(x, y) != qb.get_at(x, y)
    });
}
