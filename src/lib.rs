#![allow(dead_code)]

use std::fmt::{Display, Formatter};
// GIMP Palette
// Name: PixelArt 16
// Columns: 8
// #
// 0 0 0	Black
// 20 12 28	Haiti
// 68 36 52	Castro
// 48 52 109	Torea bay
// 78 74 78	Salt box
// 89 125 206	Cornflower
// 133 149 161	Pigeon post
// 109 194 202	Fountain blue
// 52 101 36	San felix
// 133 76 48	Cinnamon
// 117 113 97	Flint
// 208 70 72	Valencia
// 210 125 44	California
// 109 170 44	Christi
// 210 170 153	Quicksand
// 218 212 94	Witch haze
// 222 238 214	Peppermint
pub struct Gpl<const COLUMNS: u32 = 6> {
    pub name: String,
    pub colors: Colors,
}

impl<const COLUMNS: u32> Display for Gpl<COLUMNS> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{header}\nName: {name}\nColumns: {COLUMNS}\n{colors_header}\n{colors}",
            header = Self::HEADER,
            name = self.name,
            colors_header = Self::COLORS_HEADER,
            colors = self.colors,
        )
    }
}

impl<const COLUMNS: u32> Gpl<COLUMNS> {
    const HEADER: &'static str = "GIMP Palette";
    const COLORS_HEADER: char = '#';
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub name: String,
}

pub struct Colors(pub Vec<Color>);

impl Display for Colors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for Color { r, g, b, name } in &self.0 {
            writeln!(f, "{r} {g} {b}\t{name}")?;
        }

        Ok(())
    }
}
