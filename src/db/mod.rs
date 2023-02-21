pub mod surreal_http;
pub mod entities;
pub mod id;

pub enum DbResult<T> {
    Existing(T),
    New(T),
}
