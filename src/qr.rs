use std::collections::HashSet;
use std::rc::Rc;

use bardecoder::decode::{Decode, QRDecoder};
use bardecoder::util::qr::{QRData, QRError};

#[derive(Clone)]
pub struct QR {
    pub data: Vec<u8>,
    pub side: u32,
    pub version: u32,

    functional: Rc<HashSet<usize>>,
}

// Safety: self.functional will never be mutated
unsafe impl Send for QR {}
unsafe impl Sync for QR {}

impl QR {
    pub fn from_code(code: qrcode::QrCode) -> QR {
        let version = if let qrcode::Version::Normal(x) = code.version() {
            x as u32
        } else {
            panic!("aoeu micro moment");
        };
        let mut functional = HashSet::new();
        for x in 0..code.width() {
            for y in 0..code.width() {
                let i = x + y * code.width();
                if code.is_functional(x, y) {
                    functional.insert(i);
                }
            }
        }
        // version markers
        for s in 0..4 {
            for l in 0..7 {
                let x = code.width() - 11 + s;
                let y = l;

                let i = x + y * code.width();
                let i_flip = y + x * code.width();

                functional.insert(i);
                functional.insert(i_flip);
            }
        }

        let cdata = code.to_colors();
        let idata: Vec<_> = cdata
            .into_iter()
            .map(|x| if x == qrcode::Color::Light { 1 } else { 0 })
            .collect();

        println!("Using version {:?}", code.version());

        assert_eq!(code.version(), qrcode::Version::Normal(version as i16));

        let inner = QRData::new(idata, version);
        QR {
            data: inner.data,
            side: inner.side,
            version,
            functional: Rc::new(functional),
        }
    }

    pub fn make(data: &[u8], version: u32) -> QR {
        let code = qrcode::QrCode::with_version(
            data,
            qrcode::Version::Normal(version as i16),
            qrcode::EcLevel::H,
        )
        .expect("couldn't make code");

        let mut functional = HashSet::new();
        for x in 0..code.width() {
            for y in 0..code.width() {
                let i = x + y * code.width();
                if code.is_functional(x, y) {
                    functional.insert(i);
                }
            }
        }
        // version markers
        for s in 0..4 {
            for l in 0..7 {
                let x = code.width() - 11 + s;
                let y = l;

                let i = x + y * code.width();
                let i_flip = y + x * code.width();

                functional.insert(i);
                functional.insert(i_flip);
            }
        }

        let cdata = code.to_colors();
        let idata: Vec<_> = cdata
            .into_iter()
            .map(|x| if x == qrcode::Color::Light { 1 } else { 0 })
            .collect();

        println!("Using version {:?}", code.version());

        assert_eq!(code.version(), qrcode::Version::Normal(version as i16));


        let inner = QRData::new(idata, version);
        QR {
            data: inner.data,
            side: inner.side,
            version,
            functional: Rc::new(functional),
        }
    }

    pub fn is_functional(&self, x: usize, y: usize) -> bool {
        let i = y * self.side as usize + x;
        self.functional.contains(&i)
    }

    pub fn get_at(&self, x: usize, y: usize) -> u8 {
        self.data[y * self.side as usize + x]
    }

    pub fn dump(&self) {
        self.dump_hl(|_, _| false);
    }

    pub fn dump_hl<F: Fn(usize, usize) -> bool>(&self, f: F) {
        let mut buf = String::new();

        for x in 0..self.side+2 {
            buf.push_str("██");
        }
        buf.push_str("\n");
        for y in 0..self.side {
            buf.push_str("\x1b[0m██");
            for x in 0..self.side {
                let col = self.data[(y * self.side + x) as usize];

                let hl = f(x as usize, y as usize);

                if col != 0 {
                    if hl {
                        buf.push_str("\x1b[38;5;154m██"); // light green
                    } else {
                        buf.push_str("\x1b[0m██");
                    }
                } else {
                    if hl {
                        buf.push_str("\x1b[38;5;22m██"); // dark green
                    } else {
                        buf.push_str("\x1b[0m  ");
                    }
                }
            }
            buf.push_str("\x1b[0m██");
            buf.push_str("\x1b[0m\n");
        }
        for x in 0..self.side+2 {
            buf.push_str("██");
        }
        println!("{}", buf);
    }

    pub fn decode(&self) -> Result<(String, u32), String> {
        let dec = QRDecoder::new();

        let qrdata = QRData {
            data: self.data.clone(),
            version: self.version,
            side: self.side,
        };

        let res = dec.decode_with_error_count(Ok(qrdata));

        match res {
            Ok(x) => Ok(x),
            Err(QRError { msg }) => Err(msg),
        }
    }
}
