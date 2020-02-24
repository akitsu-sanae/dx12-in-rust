use std::fmt::Debug;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Vec3<T: Debug + Clone> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: Debug + Clone> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Vec3 { x: x, y: y, z: z }
    }
}
