pub mod interface;
pub mod lifecycle;
pub mod model;
pub mod popup;
pub mod protection;
pub mod store;
pub mod surface;

pub type Result<T> = std::result::Result<T, String>;
