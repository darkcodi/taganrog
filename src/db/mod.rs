pub mod surreal_http;
pub mod tag;

pub enum DbResult<T> {
    Existing(T),
    New(T),
}
