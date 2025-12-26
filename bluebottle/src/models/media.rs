#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
/// A collection of series or movies.
pub struct Collection {}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
/// A long form movie/film.
///
/// It contains no child media entries.
pub struct Movie {}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
/// A series contains multiple episodes.
pub struct Series {
    pub episodes: Vec<Episode>,
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
/// An episode is single part or chunk of a [Series].
pub struct Episode {}
