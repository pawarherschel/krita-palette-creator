use image::{ColorType, DynamicImage, GenericImage, GenericImageView, Rgba};
use krita_palette_creator::{Colors, Gpl};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Sub;
use std::{env, fs};

fn sat(c: i16) -> u8 {
    if c > i16::from(u8::MAX) {
        u8::MAX
    } else if c < i16::from(u8::MIN) {
        u8::MIN
    } else {
        u8::try_from(c).expect("already checked")
    }
}

struct MinMax<T> {
    min: T,
    max: T,
}
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Idx {
    x: u32,
    y: u32,
}
#[derive(Debug, Copy, Clone)]
struct Item {
    color: Color,
    idx: Idx,
}

#[allow(clippy::too_many_lines)]
fn main() {
    let args = env::args().collect::<Vec<_>>();
    let kv_args = args
        .iter()
        .skip(1)
        .filter_map(|s| {
            let s = s.trim_start_matches("--");

            if !s.contains('=') {
                return None;
            }

            let (k, v) = s.split_once('=').unwrap();
            if v.is_empty() | k.is_empty() {
                return None;
            }

            Some((k, v))
        })
        .collect::<HashMap<_, _>>();

    let depth = if cfg!(debug_assertions) {
        4
    } else {
        kv_args
            .get("depth")
            .map(|q| q.parse::<u32>().expect("error parsing depth, expected u32"))
            .expect("depth not found")
    };
    let image_path = if cfg!(debug_assertions) {
        "Quantum_frog.png"
    } else {
        kv_args.get("src").expect("no image provided")
    };
    let tolerance = if cfg!(debug_assertions) {
        0
    } else {
        kv_args
            .get("tolerance")
            .map(|q| {
                q.parse::<u16>()
                    .expect("error parsing tolerance, expected u16")
            })
            .unwrap_or_default()
    };

    let image = image::open(image_path).expect("image not found, make sure the src path is valid");

    let bucket = image
        .pixels()
        .filter_map(|(x, y, Rgba([r, g, b, a]))| {
            if u8::MAX == a {
                Some(Item {
                    color: Color { r, g, b },
                    idx: Idx { x, y },
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut colors_used = Vec::new();

    let buf = median_cut(bucket, depth, BTreeMap::new(), &mut colors_used);

    let mut result_image = DynamicImage::new(image.width(), image.height(), ColorType::Rgba8);

    for (Idx { x, y }, Color { r, g, b }) in buf {
        result_image.put_pixel(x, y, Rgba([r, g, b, 255]));
    }

    let save_path = if cfg!(debug_assertions) {
        String::from("Quantum frog.png.rust.4.png")
    } else {
        format!("{image_path}.{depth}.{tolerance}.png")
    };

    result_image.save(&save_path).expect("image saving failed");

    let mut colors_used = dedupe(colors_used, tolerance);
    colors_used.sort();
    let colors_used = colors_used;

    for c in &colors_used {
        println!("{c:?}");
    }

    println!("total: {}", colors_used.len());

    let colors = colors_used
        .iter()
        .map(|&Color { r, g, b }| krita_palette_creator::Color {
            r,
            g,
            b,
            name: format!("#{r:02X}{g:02X}{b:02X}"),
        })
        .collect::<Vec<_>>();
    let colors = Colors(colors);

    const COLUMNS: u32 = 8;
    const SQUARE_SIZE: u32 = 16;

    let gpl: Gpl<COLUMNS> = Gpl {
        name: save_path.clone(),
        colors,
    };

    let palette_save_path = format!("{save_path}.gpl");
    fs::write(&palette_save_path, gpl.to_string()).expect("failed to write gpl");

    let w = COLUMNS * SQUARE_SIZE;
    let h = u32::try_from(gpl.colors.0.len())
        .expect("how")
        .div_ceil(COLUMNS)
        * SQUARE_SIZE;

    let mut palette_preview = DynamicImage::new(w, h, ColorType::Rgb8);

    for (idx, &Color { r, g, b }) in colors_used.iter().enumerate() {
        for (i, j) in (0..SQUARE_SIZE).flat_map(move |i| (0..SQUARE_SIZE).map(move |j| (i, j))) {
            let idx = u32::try_from(idx).expect("how");
            let row = idx / COLUMNS;
            let col = idx % COLUMNS;

            let x = col * SQUARE_SIZE + j;
            let y = row * SQUARE_SIZE + i;

            palette_preview.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    palette_preview
        .save(format!("{palette_save_path}.png"))
        .expect("failed to save preview image");
}

fn dedupe(vec: Vec<Color>, tolerance: u16) -> Vec<Color> {
    let mut set = BTreeSet::new();
    let tolerance = tolerance as i16;

    for c in vec {
        let Color { r, g, b } = c;
        let r = i16::from(r);
        let g = i16::from(g);
        let b = i16::from(b);

        let contained_in_set = (-tolerance..=tolerance)
            .flat_map(move |x| {
                (-tolerance..=tolerance)
                    .flat_map(move |y| (-tolerance..=tolerance).map(move |z| (x, y, z)))
            })
            .map(|(x, y, z)| {
                let r = r + x;
                let g = g + y;
                let b = b + z;

                let r = sat(r);
                let g = sat(g);
                let b = sat(b);

                Color { r, g, b }
            })
            .any(|c| set.contains(&c));

        if !contained_in_set {
            set.insert(c);
        }
    }

    set.into_iter().collect()
}

fn median_cut(
    mut bucket: Vec<Item>,
    depth: u32,
    buf: BTreeMap<Idx, Color>,
    colors_used: &mut Vec<Color>,
) -> BTreeMap<Idx, Color> {
    if depth == 0 {
        return quantize(bucket, buf, colors_used);
    }

    let MinMax { min, max } = bucket.iter().fold(
        MinMax {
            min: Color::MAX,
            max: Color::MIN,
        },
        |min_max, item| {
            let MinMax { min, max } = min_max;

            let &Item { color, .. } = item;

            let min = min.min_channels(color);
            let max = max.max_channels(color);

            MinMax { min, max }
        },
    );

    let value = max - min;

    let highest_channel = value.r.max(value.g).max(value.b);

    if value.r == highest_channel {
        bucket.sort_by(|one, two| one.color.r.cmp(&two.color.r));
    } else if value.g == highest_channel {
        bucket.sort_by(|one, two| one.color.g.cmp(&two.color.g));
    } else {
        bucket.sort_by(|one, two| one.color.b.cmp(&two.color.b));
    };

    let median_index = bucket.len() / 2;

    let second_half = bucket.split_off(median_index);
    let first_half = bucket;

    let buf = median_cut(first_half, depth - 1, buf, colors_used);
    let buf = median_cut(second_half, depth - 1, buf, colors_used);

    buf
}

fn quantize(
    bucket: Vec<Item>,
    mut buf: BTreeMap<Idx, Color>,
    colors_used: &mut Vec<Color>,
) -> BTreeMap<Idx, Color> {
    let [sr, sg, sb] = bucket.iter().fold(
        [0, 0, 0],
        |[sr, sg, sb],
         &Item {
             color: Color { r, g, b },
             ..
         }| { [sr + u32::from(r), sg + u32::from(g), sb + u32::from(b)] },
    );

    let average_color = Color {
        r: u8::try_from((sr as usize) / bucket.len())
            .expect("average color should be within range"),
        g: u8::try_from((sg as usize) / bucket.len())
            .expect("average color should be within range"),
        b: u8::try_from((sb as usize) / bucket.len())
            .expect("average color should be within range"),
    };

    colors_used.push(average_color);

    for item in bucket {
        buf.insert(item.idx, average_color);
    }

    buf
}

impl Color {
    const MAX: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    const MIN: Self = Self { r: 0, g: 0, b: 0 };
}

impl Sub for Color {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let r = self.r - rhs.r;
        let g = self.g - rhs.g;
        let b = self.b - rhs.b;

        Self::Output { r, g, b }
    }
}

impl Color {
    #[inline]
    pub(crate) fn min_channels(self, rhs: Self) -> Self {
        let r = self.r.min(rhs.r);
        let g = self.g.min(rhs.g);
        let b = self.b.min(rhs.b);

        Self { r, g, b }
    }

    #[inline]
    fn max_channels(self, rhs: Self) -> Self {
        let r = self.r.max(rhs.r);
        let g = self.g.max(rhs.g);
        let b = self.b.max(rhs.b);

        Self { r, g, b }
    }
}
