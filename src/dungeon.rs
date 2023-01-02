#![allow(clippy::cast_sign_loss)]

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::{Add, Mul, Sub},
};

use rand::Rng;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Tile {
    Wall,
    OpenDoor,
    ClosedDoor,
    Floor,
}

impl Tile {
    pub const fn to_char(self) -> char {
        match self {
            Self::Wall => ' ',
            Self::OpenDoor => '/',
            Self::ClosedDoor => '+',
            Self::Floor => '█',
        }
    }
}

pub struct Stage {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
}

impl Stage {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            tiles: vec![Tile::Wall; (width * height) as usize],
        }
    }

    pub fn set(&mut self, pos: Vector, tile: Tile) {
        let idx = (pos.1 * self.width + pos.0) as usize;
        let t = self.tiles.get_mut(idx);
        if let Some(t) = t {
            *t = tile;
        }
    }

    pub fn get(&self, pos: Vector) -> Option<Tile> {
        let idx = (pos.1 * self.width + pos.0) as usize;
        self.tiles.get(idx).copied()
    }

    pub const fn contains(&self, pos: Vector) -> bool {
        pos.0 >= 0 && pos.0 < self.width && pos.1 >= 0 && pos.1 < self.height
    }
}

impl Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.width + 2 {
            write!(f, "-")?;
        }
        writeln!(f)?;
        for y in 0..self.height {
            write!(f, "|")?;
            for x in 0..self.width {
                write!(f, "{}", self.get(Vector(x, y)).unwrap().to_char())?;
            }
            writeln!(f, "|")?;
        }
        for _ in 0..self.width + 2 {
            write!(f, "-")?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rectangle {
    pub const fn top(self) -> i32 {
        self.y
    }
    pub const fn bottom(self) -> i32 {
        self.y + self.h
    }
    pub const fn left(self) -> i32 {
        self.x
    }
    pub const fn right(self) -> i32 {
        self.x + self.w
    }

    pub const fn distance_to(self, other: Self) -> i32 {
        let vertical = if self.top() >= other.bottom() {
            self.top() - other.bottom()
        } else if self.bottom() <= other.top() {
            other.top() - self.bottom()
        } else {
            -1
        };

        let horizontal = if self.left() >= other.right() {
            self.left() - other.right()
        } else if self.right() <= other.left() {
            other.left() - self.right()
        } else {
            -1
        };

        if vertical == -1 && horizontal == -1 {
            -1
        } else if vertical == -1 {
            horizontal
        } else if horizontal == -1 {
            vertical
        } else {
            vertical + horizontal
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Vector(i32, i32);
impl Add for Vector {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}
impl Sub for Vector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0, self.1 - other.1)
    }
}
impl Mul<i32> for Vector {
    type Output = Self;

    fn mul(self, other: i32) -> Self {
        Self(self.0 * other, self.1 * other)
    }
}
impl Vector {
    pub const fn abs(self) -> i32 {
        self.0.abs() + self.1.abs()
    }
}

static CARDINALS: [Vector; 4] = [Vector(0, -1), Vector(1, 0), Vector(0, 1), Vector(-1, 0)];

pub struct Dungeon<'a> {
    n_room_tries: u32,
    rooms: Vec<Rectangle>,
    /// For each open position in the dungeon, the index of the connected region
    /// that that position is a part of.
    regions: HashMap<Vector, i32>,
    curr_region: i32,
    stage: &'a mut Stage,
}

impl<'a> Dungeon<'a> {
    const EXTRA_CONNECTOR_CHANCE: i32 = 20;
    const WINDING_PERCENT: i32 = 0;
    const ROOM_EXTRA_SIZE: i32 = 0;

    pub fn new(stage: &'a mut Stage) -> Self {
        Self {
            n_room_tries: 50,
            rooms: Vec::new(),
            regions: HashMap::new(),
            curr_region: -1,
            stage,
        }
    }

    pub fn generate(&mut self) {
        assert!(
            !(self.stage.width % 2 == 0 || self.stage.height % 2 == 0),
            "Stage width and height must be odd"
        );

        self.regions = HashMap::new();

        println!("Adding rooms");
        self.add_rooms();
        println!("stage: \n{}", self.stage);

        // Fill in all of the empty space with mazes.
        println!("Adding mazes");
        for y in (1..self.stage.height).step_by(2) {
            for x in (1..self.stage.width).step_by(2) {
                let pos = Vector(x, y);
                if self.get_tile(pos) != Tile::Wall {
                    continue;
                }
                self.grow_maze(pos);
            }
        }
        println!("stage: \n{}", self.stage);

        // Connect all of the regions with mazes.
        println!("Connecting regions");
        self.connect_regions();
        println!("stage: \n{}", self.stage);

        // Remove dead ends.
        println!("Removing dead ends");
        self.remove_dead_ends();
        println!("stage: \n{}", self.stage);
    }

    fn grow_maze(&mut self, start: Vector) {
        let mut cells = Vec::new();
        let mut last_dir = Vector(0, 0);

        self.start_region();
        self.carve(start, Tile::Floor);

        cells.push(start);
        while let Some(&cell) = cells.last() {
            let mut unmade_cells = Vec::new();

            for &dir in &CARDINALS {
                if self.can_carve(cell, dir) {
                    unmade_cells.push(dir);
                }
            }

            if unmade_cells.is_empty() {
                cells.pop();
                last_dir = Vector(0, 0);
            } else {
                let dir = if unmade_cells.contains(&last_dir)
                    && rand::Rng::gen_range(&mut rand::thread_rng(), 1..=100)
                        > Self::WINDING_PERCENT
                {
                    last_dir
                } else {
                    unmade_cells[rand::random::<usize>() % unmade_cells.len()]
                };

                assert!(CARDINALS.contains(&dir));

                self.carve(cell + dir, Tile::Floor);
                self.carve(cell + dir * 2, Tile::Floor);

                cells.push(cell + dir * 2);
                last_dir = dir;
            }
        }
    }

    fn add_rooms(&mut self) {
        'outer: for _ in 0..self.n_room_tries {
            // Pick a random room size. The funny math here does two things:
            // - It makes sure rooms are odd-sized to line up with maze.
            // - It avoids creating rooms that are too rectangular: too tall and
            //   narrow or too wide and flat.
            // TODO: This isn't very flexible or tunable. Do something better here.
            let size = rand::thread_rng().gen_range(1..=3 + Self::ROOM_EXTRA_SIZE) * 2 + 1;
            let rectangularity = rand::thread_rng().gen_range(0..=1 + (size / 2)) * 2;
            let mut width = size;
            let mut height = size;
            if rand::thread_rng().gen_bool(0.5) {
                width += rectangularity;
            } else {
                height += rectangularity;
            }

            let x = rand::thread_rng().gen_range(0..(self.stage.width - width) / 2) * 2 + 1;
            let y = rand::thread_rng().gen_range(0..(self.stage.height - height) / 2) * 2 + 1;

            let room = Rectangle {
                x,
                y,
                w: width,
                h: height,
            };

            for &other in &self.rooms {
                if room.distance_to(other) <= 0 {
                    continue 'outer;
                }
            }

            self.rooms.push(room);

            self.start_region();

            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    self.carve(Vector(x, y), Tile::Floor);
                }
            }
        }
    }

    fn connect_regions(&mut self) {
        // Find all of the tiles that can connect two (or more) regions.
        let mut connector_regions = Vec::new();
        for y in 1..self.stage.height - 1 {
            for x in 1..self.stage.width - 1 {
                let pos = Vector(x, y);
                if self.get_tile(pos) != Tile::Wall {
                    continue;
                }

                let mut regions = Vec::new();
                for &dir in &CARDINALS {
                    let region = self.regions.get(&(pos + dir));
                    if let Some(&region) = region {
                        if !regions.contains(&region) {
                            regions.push(region);
                        }
                    }
                }

                if regions.len() < 2 {
                    continue;
                }

                connector_regions.push((pos, regions));
            }
        }

        let mut connectors = connector_regions
            .iter()
            .map(|(pos, _)| *pos)
            .collect::<Vec<_>>();

        // Keep track of which regions have been merged. This maps an original
        // region index to the one it has been merged to.
        let mut merged_regions = HashMap::new();
        let mut open_regions = HashSet::new();
        for i in 0..=self.curr_region {
            merged_regions.insert(i, i);
            open_regions.insert(i);
        }

        // Keep connecting regions until we're down to one.
        while open_regions.len() > 1 {
            let connector = connectors[rand::random::<usize>() % connectors.len()];

            // Carve the connection.
            self.add_junction(connector);

            // Merge the connected regions. We'll pick one region (arbitrarily) and
            // map all of the other regions to its index.
            let regions = connector_regions
                .iter()
                .find(|(pos, _)| *pos == connector)
                .unwrap()
                .1
                .iter()
                .map(|&region| merged_regions[&region])
                .collect::<Vec<_>>();
            let dest = *regions.first().unwrap();
            let sources = regions.iter().skip(1).copied().collect::<Vec<_>>();

            // Merge all of the affected regions. We have to look at *all* of the
            // regions because other regions may have previously been merged with
            // some of the ones we're merging now.
            for i in 0..=self.curr_region {
                if sources.contains(&merged_regions[&i]) {
                    merged_regions.insert(i, dest);
                }
            }

            // The sources are no longer in use.
            for source in sources {
                open_regions.remove(&source);
            }

            // Remove any connectors that aren't needed anymore.
            connectors.retain(|&pos| {
                !(|| {
                    // Don't allow connectors right next to each other.
                    if (connector - pos).abs() < 2 {
                        return true;
                    }

                    // If the connector no long spans different regions, we don't need it.
                    let regions = connector_regions
                        .iter()
                        .find(|(p, _)| *p == pos)
                        .unwrap()
                        .1
                        .iter()
                        .map(|&region| merged_regions[&region])
                        .collect::<HashSet<_>>();

                    if regions.len() > 1 {
                        return false;
                    }

                    // This connector isn't needed, but connect it occasionally so that the
                    // dungeon isn't singly-connected.
                    if rand::thread_rng().gen_ratio(1, Self::EXTRA_CONNECTOR_CHANCE as u32) {
                        self.add_junction(pos);
                    }

                    true
                })()
            });
        }
    }

    fn add_junction(&mut self, pos: Vector) {
        if rand::thread_rng().gen_ratio(1, 4) {
            self.set_tile(
                pos,
                if rand::thread_rng().gen_ratio(1, 3) {
                    Tile::OpenDoor
                } else {
                    Tile::Floor
                },
            );
        } else {
            self.set_tile(pos, Tile::ClosedDoor);
        }
    }

    fn remove_dead_ends(&mut self) {
        let mut done = false;

        while !done {
            done = true;

            for y in 1..self.stage.height - 1 {
                for x in 1..self.stage.width - 1 {
                    let pos = Vector(x, y);
                    if self.get_tile(pos) == Tile::Wall {
                        continue;
                    }

                    // If it only has one exit, it's a dead end.
                    let mut exits = 0;
                    for &dir in &CARDINALS {
                        let neighbor = pos + dir;
                        if self.get_tile(neighbor) != Tile::Wall {
                            exits += 1;
                        }
                    }

                    if exits != 1 {
                        continue;
                    }

                    done = false;
                    self.set_tile(pos, Tile::Wall);
                }
            }
        }
    }

    fn can_carve(&self, pos: Vector, direction: Vector) -> bool {
        // Must end in bounds.
        if !self.stage.contains(pos + direction * 3) {
            return false;
        }

        // Destination must not be open.
        self.get_tile(pos + direction * 2) == Tile::Wall
    }

    fn start_region(&mut self) {
        self.curr_region += 1;
    }

    fn carve(&mut self, pos: Vector, tile: Tile) {
        self.set_tile(pos, tile);
        self.regions.insert(pos, self.curr_region);
    }

    pub fn get_tile(&self, pos: Vector) -> Tile {
        self.stage.get(pos).unwrap()
    }

    fn set_tile(&mut self, pos: Vector, tile: Tile) {
        self.stage.set(pos, tile);
    }
}
