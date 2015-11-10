use Vec3f;

#[derive(Clone, Debug, PartialEq)]
pub struct ContactInfo {
    pub time: f32,
    pub normal: Vec3f,
}
