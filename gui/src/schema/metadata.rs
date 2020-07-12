use polyengine_core::*;

pub enum Metadata {
    Text(String),
    Integer(i64),
    Float(f64),
    Unsigned(u64),
    Vec2i(Vector2i),
    Vec2f(Vector2f),
    Vec2u(Vector2u),
}