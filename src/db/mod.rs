pub mod surreal_http;
pub mod entities;

pub enum DbResult<T> {
    Existing(T),
    New(T),
}
