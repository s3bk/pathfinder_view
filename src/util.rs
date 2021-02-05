use pathfinder_geometry::vector::Vector2I;

fn round_to_16(i: i32) -> i32 {
    (i + 15) & !0xf
}
pub fn round_v_to_16(v: Vector2I) -> Vector2I {
    Vector2I::new(round_to_16(v.x()), round_to_16(v.y()))
}
