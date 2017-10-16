use super::Vec3f;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ContactInfo {
    pub time: f32,
    pub normal: Vec3f,
}
