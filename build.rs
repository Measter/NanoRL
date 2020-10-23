// This build script converts the tiles.png file into a byte array suitable
// for flinging onto the display.

// The pixel data on the display is sent to the display in a packet starting with
// 0x40, then 8 bytes representing the 8 columns.
// The MSB in each byte is the bottom row, LSB is the top.

use image::{GenericImageView, SubImage, RgbaImage};

use std::{
    fs::File,
    io::{Write, BufWriter},
    path::Path,
};

const SSD1306_DATA: u8 = 0x40;
const TILE_SIZE: u32 = 8;
const NUM_TILES: u32 = 15;

const TILES: [&str; 15] = ["FLOOR", "WALL", "STAIRS", "ENEMY", "PLAYER",
    "N0", "N1", "N2", "N3", "N4", "N5", "N6", "N7", "N8", "N9",
];

fn main() {
    println!("cargo::rerun-if-changes=tiles.png");
    println!("cargo::rerun-if-changes=title_screen.png");
    println!("cargo::rerun-if-changes=game_over.png");

    let tiles = image::open("assets/tiles.png").unwrap()
        .to_rgba();

    assert_eq!(tiles.width(), NUM_TILES * TILE_SIZE);
    assert_eq!(tiles.height(), TILE_SIZE);

    let tiles: Vec<_> = (0..NUM_TILES)
        .map(|i| (i*TILE_SIZE, 0, TILE_SIZE, TILE_SIZE))
        .map(|(x, y, width, height)| tiles.view(x, y, width, height))
        .collect();

    let tiles = process_tiles(tiles);

    let path = Path::new("src").join("game").join("tiles.rs");
    let file = File::create(path).unwrap();
    let mut file = BufWriter::new(file);

    writeln!(&mut file, "// Generated by the build.rs file during compilation").unwrap();

    for (name, tile) in TILES.iter().zip(tiles) {
        writeln!(&mut file, "#[link_section = \".text\"]").unwrap();
        write!(&mut file, "pub static {}: [u8; {}] = [", name, tile.len()).unwrap();

        // Fortunately, rust doesn't care about trailing commas!
        for b in tile {
            write!(&mut file, "{},", b).unwrap();
        }

        writeln!(&mut file, "];").unwrap();
    }

    write_splash(&mut file, "TITLE_SCREEN", "title_screen.png");
    write_splash(&mut file, "GAME_OVER", "game_over.png");
}

fn write_splash(file: &mut BufWriter<File>, splash_name: &str, filename: &str) {
    let title_screen = process_splash(filename);

    writeln!(file, "#[link_section = \".text\"]").unwrap();
    writeln!(file, "pub static {}: [u8; {}] = [", splash_name, title_screen.len()).unwrap();

    // We'll be nice, and chunk the output...
    for chunk in title_screen.chunks(32) {
        write!(file, "    ").unwrap();
        for b in chunk {
            write!(file, "{},", b).unwrap();
        }
        writeln!(file).unwrap();
    }

    writeln!(file, "];").unwrap();
}

fn process_splash(filename: &str) -> Vec<u8> {
    let path = Path::new("assets").join(filename);

    let img = image::open(path).unwrap().to_rgba();

    // This won't be process as a set of tiles. Instead, we'll output this so the entire thing can
    // be thrown onto the screen in one go.
    // The screen will take a single 8-pixel column, starting from the upper left, and when it reaches
    // the end of the column, it will wrap around to the next 8-pixel column.

    // So we need to chunk the image into 8-pixel wide rows.
    // The processing is otherwise basically the same as for the tiles.
    // One thing we need to concern ourselves with is that my TWI buffer length is 32 bytes.
    // This means that we need to insert the SSD1306_DATA byte every 31st byte.

    let mut bytes = Vec::new();
    let mut count = 0;
    let mut push = |b| {
        if count == 0 {
            bytes.push(SSD1306_DATA);
        }
        count += 1;
        count %= 31;
        bytes.push(b);
    };
    for row in 0..8 {
        let row = img.view(0, row*TILE_SIZE, 128, TILE_SIZE);
        for x in 0..128 {
            let mut column = 0;
            for y in (0..TILE_SIZE).rev() {
                column <<= 1;
                if row.get_pixel(x, y)[0] == 255 {
                    column |= 1;
                }
            }
            push(column);
        }
    }

    bytes
}

fn process_tiles(tiles: Vec<SubImage<&RgbaImage>>) -> Vec<Vec<u8>> {
    tiles.into_iter().map(|tile| {
        let mut bytes = vec![SSD1306_DATA];

        for x in 0..TILE_SIZE {
            let mut column = 0;
            for y in (0..TILE_SIZE).rev() {
                column <<= 1;
                if tile.get_pixel(x, y)[0] == 255 {
                    column |= 1;
                }
            }
            bytes.push(column);
        }

        bytes
    }).collect()
}