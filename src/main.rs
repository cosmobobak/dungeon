#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use dungeon::Stage;

use crate::dungeon::Dungeon;

mod dungeon;

fn main() {
    let mut stage = Stage::new(301, 71);
    println!("{stage}");
    println!();
    let mut dungeon_generator = Dungeon::new(&mut stage);
    dungeon_generator.generate();
    // println!("{stage}");
}
