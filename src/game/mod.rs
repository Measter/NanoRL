mod input;
pub mod rng;
pub mod tiles;
pub use input::Input;

use crate::{
    hal::{
        progmem::PGMSlice,
        twi::{TWIError, TWI},
    },
    peripherals::display::{self, Display},
};

// Need to be careful about the map size. The ATmega328P only has 2k of RAM.
const LEVEL_SIZE: usize = 16;
const NUM_ENEMIES: usize = 10;

const SCREEN_WIDTH: usize = display::WIDTH as usize / 8;
const SCREEN_HEIGHT: usize = display::HEIGHT as usize / 8;
const SCREEN_MAX_X: usize = LEVEL_SIZE - SCREEN_WIDTH;
const SCREEN_MAX_Y: usize = LEVEL_SIZE - SCREEN_HEIGHT;

#[derive(Copy, Clone)]
pub enum ContinueState {
    Continue,
    NewLevel,
    RestartLoop,
    GameOver,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Tile {
    Floor,
    Wall,
    Stairs,
    Player,
    Enemy,
}

impl Tile {
    pub fn graphic(self) -> PGMSlice {
        let (addr, len) = match self {
            Tile::Floor => (&tiles::FLOOR as *const u8, tiles::FLOOR.len()),
            Tile::Wall => (&tiles::WALL as *const u8, tiles::WALL.len()),
            Tile::Stairs => (&tiles::STAIRS as *const u8, tiles::STAIRS.len()),
            Tile::Player => (&tiles::PLAYER as *const u8, tiles::PLAYER.len()),
            Tile::Enemy => (&tiles::ENEMY as *const u8, tiles::ENEMY.len()),
        };

        unsafe { PGMSlice::from_raw_parts(addr, len) }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct Position {
    x: u8,
    y: u8,
}

impl Position {
    fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
}

struct Map([Tile; LEVEL_SIZE * LEVEL_SIZE]);
impl core::ops::Index<(u8, u8)> for Map {
    type Output = Tile;
    fn index(&self, (x, y): (u8, u8)) -> &Self::Output {
        let row = y as usize * LEVEL_SIZE;
        &self.0[row + x as usize]
    }
}
impl core::ops::IndexMut<(u8, u8)> for Map {
    fn index_mut(&mut self, (x, y): (u8, u8)) -> &mut Self::Output {
        let row = y as usize * LEVEL_SIZE;
        &mut self.0[row + x as usize]
    }
}

#[derive(Copy, Clone)]
struct Enemy {
    position: Position,
}

pub struct Game {
    map: Map,
    player_pos: Position,
    enemies: [Option<Enemy>; NUM_ENEMIES],
    level: u8,
}

impl Game {
    pub fn title_screen() -> PGMSlice {
        unsafe {
            PGMSlice::from_raw_parts(&tiles::TITLE_SCREEN as *const u8, tiles::TITLE_SCREEN.len())
        }
    }

    pub fn game_over_screen() -> PGMSlice {
        unsafe { PGMSlice::from_raw_parts(&tiles::GAME_OVER as *const u8, tiles::GAME_OVER.len()) }
    }

    pub fn get_digit_tile(digit: u8) -> PGMSlice {
        let (ptr, len) = match digit {
            b'9' => (&tiles::N9 as *const u8, tiles::N9.len()),
            b'8' => (&tiles::N8 as *const u8, tiles::N8.len()),
            b'7' => (&tiles::N7 as *const u8, tiles::N7.len()),
            b'6' => (&tiles::N6 as *const u8, tiles::N6.len()),
            b'5' => (&tiles::N5 as *const u8, tiles::N5.len()),
            b'4' => (&tiles::N4 as *const u8, tiles::N4.len()),
            b'3' => (&tiles::N3 as *const u8, tiles::N3.len()),
            b'2' => (&tiles::N2 as *const u8, tiles::N2.len()),
            b'1' => (&tiles::N1 as *const u8, tiles::N1.len()),
            b'0' => (&tiles::N0 as *const u8, tiles::N0.len()),
            _ => (&tiles::FLOOR as *const u8, tiles::FLOOR.len()),
        };

        unsafe { PGMSlice::from_raw_parts(ptr, len) }
    }

    pub fn new() -> Self {
        Self {
            player_pos: Position::new(0, 0),
            level: 0,
            enemies: [None; NUM_ENEMIES],
            map: Map([Tile::Floor; LEVEL_SIZE * LEVEL_SIZE]),
        }
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn reset(&mut self) {
        self.level = 0;
    }

    pub fn update(&mut self, input: &Input) -> ContinueState {
        match self.handle_player(input) {
            ContinueState::Continue => {}
            state => return state,
        }

        self.handle_enemy()
    }

    fn handle_enemy(&mut self) -> ContinueState {
        for e in self.enemies.iter_mut().filter_map(|e| e.as_mut()) {
            let dir_row = get_dir(e.position.y, self.player_pos.y);
            let dir_col = get_dir(e.position.x, self.player_pos.x);

            // Check if we're next to the player. If we are, then hit the player, killing them.
            if self.player_pos == Position::new(e.position.x + dir_col, e.position.y)
                || self.player_pos == Position::new(e.position.x, e.position.y + dir_row)
            {
                return ContinueState::GameOver;
            }

            // Otherwise try to move towards the player.
            let next_tile = self.map[(e.position.x, e.position.y + dir_row)];
            if dir_row != 0 && next_tile == Tile::Floor {
                e.position.y += dir_row;
            }

            // Check the player's location again, as if we don't the enemy can end up moving
            // diagonally into the player, without actually attacking. So we should only do
            // this second stage of the move if the player isn't there either.
            // We don't actually want to attack, as enemies can't attack on a diagonal.

            let next_tile = self.map[(e.position.x + dir_col, e.position.y)];
            let next_pos = Position::new(e.position.x + dir_col, e.position.y);
            if dir_col != 0 && next_tile == Tile::Floor && self.player_pos != next_pos {
                e.position.x += dir_col;
            }
        }

        ContinueState::Continue
    }

    fn handle_player(&mut self, input: &Input) -> ContinueState {
        let mut next_pos = self.player_pos;
        if input.left() {
            next_pos.x -= 1;
        }
        if input.right() {
            next_pos.x += 1;
        }
        if input.up() {
            next_pos.y -= 1;
        }
        if input.down() {
            next_pos.y += 1;
        }

        // Check if there's a map item we need to consider.
        let tile = self.map[(next_pos.x, next_pos.y)];
        match tile {
            Tile::Wall => return ContinueState::RestartLoop,
            Tile::Stairs => return ContinueState::NewLevel,
            _ => {}
        }

        let enemy = self
            .enemies
            .iter_mut()
            .find(|e| matches!(e, Some(e) if e.position == next_pos));

        // If there's an enemy, kill it.
        if let Some(e) = enemy {
            *e = None;
        } else {
            self.player_pos = next_pos;
        }

        ContinueState::Continue
    }

    pub fn draw(&self, display: &mut Display, twi: &mut TWI) -> Result<(), TWIError> {
        // The map is bigger than the screen, so find the top-left coordinate of the
        // rendered portion. Ensure that the value does not overflow below 0, nor that
        // the bottom or right side run off the map.
        let offset_y = (self.player_pos.y as usize)
            .saturating_sub(SCREEN_HEIGHT / 2)
            .min(SCREEN_MAX_Y);

        let offset_x = (self.player_pos.x as usize)
            .saturating_sub(SCREEN_WIDTH / 2)
            .min(SCREEN_MAX_X);

        let rows = self
            .map
            .0
            .chunks_exact(LEVEL_SIZE)
            .skip(offset_y)
            .zip(0..)
            .take(SCREEN_HEIGHT);

        for (row, y) in rows {
            for (tile, x) in row.iter().skip(offset_x).zip(0..).take(SCREEN_WIDTH) {
                display.draw_tile(twi, &tile.graphic(), x, y)?;
            }
        }

        // Rather than check every single iteration above whether we need to draw
        // the player and enemies, we'll just re-draw those tiles.
        display.draw_tile(
            twi,
            &Tile::Player.graphic(),
            self.player_pos.x - offset_x as u8,
            self.player_pos.y - offset_y as u8,
        )?;

        // The player is always on screen, so no fancy logic was needed. But for the enemies
        // we need to filter out those that aren't on screen.
        let enemies = self.enemies.iter().filter_map(Option::as_ref).filter(|e| {
            (offset_x as u8..(offset_x + SCREEN_WIDTH) as u8).contains(&e.position.x)
                && (offset_y as u8..(offset_y + SCREEN_HEIGHT) as u8).contains(&e.position.y)
        });
        for e in enemies {
            display.draw_tile(
                twi,
                &Tile::Enemy.graphic(),
                e.position.x - offset_x as u8,
                e.position.y - offset_y as u8,
            )?;
        }

        Ok(())
    }

    pub fn new_map(&mut self, rng: &mut rng::Rng) {
        // Clear the level.
        self.map.0.iter_mut().for_each(|t| *t = Tile::Floor);

        // Draw the boundry walls.
        draw_horizontal_wall(&mut self.map.0, 0);
        draw_horizontal_wall(&mut self.map.0, LEVEL_SIZE - 1);
        draw_vertical_wall(&mut self.map.0, 0);
        draw_vertical_wall(&mut self.map.0, LEVEL_SIZE - 1);

        // Find and draw the internal walls
        let row = rng.next_range(2, LEVEL_SIZE as u8 - 2);
        let col = rng.next_range(2, LEVEL_SIZE as u8 - 2);

        draw_horizontal_wall(&mut self.map.0, row as usize);
        // Need to place two doors. One on the left side of the vertical wall, one on the right.
        let door_col = rng.next_range(1, col);
        self.map[(door_col, row)] = Tile::Floor;

        let door_col = rng.next_range(col + 1, LEVEL_SIZE as u8 - 1);
        self.map[(door_col, row)] = Tile::Floor;

        draw_vertical_wall(&mut self.map.0, col as usize);
        // Likewise vertically.
        let door_row = rng.next_range(1, row);
        self.map[(col, door_row)] = Tile::Floor;

        let door_row = rng.next_range(row + 1, LEVEL_SIZE as u8 - 1);
        self.map[(col, door_row)] = Tile::Floor;

        // Placing the stairs.
        let stair_loc = place(&self.map, rng);
        self.map[(stair_loc.x, stair_loc.y)] = Tile::Stairs;

        self.level += 1;

        self.player_pos = place(&self.map, rng);

        // Placing the enemies.
        let enemy_count = NUM_ENEMIES.min(self.level as usize);
        let mut enemies = self.enemies.iter_mut();
        for e in (&mut enemies).take(enemy_count) {
            *e = Some(Enemy {
                position: place(&self.map, rng),
            })
        }

        // Empty the remaining enemies.
        enemies.for_each(|e| *e = None);
    }
}

/// This function's return value takes advantage of overflow to represent the -1 state.
fn get_dir(enemy: u8, player: u8) -> u8 {
    player.checked_sub(enemy).map(|i| i.min(1)).unwrap_or(255)
}

fn place(map: &Map, rng: &mut rng::Rng) -> Position {
    // This is kinda hideous, but it is how the original did it.
    // We'll only be placing up to 12 items on a 16*16 grid, so it should quickly find a floor tile.
    // Note that it doesn't check if there's an existing enemy.
    loop {
        let col = rng.next_range(1, LEVEL_SIZE as u8 - 2);
        let row = rng.next_range(1, LEVEL_SIZE as u8 - 2);
        if map[(col, row)] == Tile::Floor {
            break Position::new(col, row);
        }
    }
}

fn draw_horizontal_wall(map: &mut [Tile], y: usize) {
    let row_start = y * LEVEL_SIZE;
    let row_end = row_start + LEVEL_SIZE;
    map[row_start..row_end]
        .iter_mut()
        .for_each(|t| *t = Tile::Wall);
}

fn draw_vertical_wall(map: &mut [Tile], x: usize) {
    for row in map.chunks_exact_mut(LEVEL_SIZE) {
        row[x] = Tile::Wall;
    }
}
