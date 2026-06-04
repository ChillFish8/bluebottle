//! Card family. A base bordered glass surface plus the higher-level tiles
//! built on top of it.

mod core;
mod film_facts;
mod library_count;

pub use core::{Card, card};

pub use film_facts::{FilmFacts, film_facts};
pub use library_count::library_count;
