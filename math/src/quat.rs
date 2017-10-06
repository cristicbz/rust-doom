use super::{Vec3, Vec3f, Mat4};
use num::Zero;
use std::f32;
use std::ops::Mul;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Quat([f32; 4]);

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct EulerAngles {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}
impl Default for Quat {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl Quat {
    #[inline]
    pub fn identity() -> Self {
        Quat([1.0, 0.0, 0.0, 0.0])
    }

    #[inline]
    pub fn x_rotation(angle: f32) -> Self {
        let half_angle = angle * 0.5;
        Quat([half_angle.cos(), half_angle.sin(), 0.0, 0.0])
    }

    #[inline]
    pub fn y_rotation(angle: f32) -> Self {
        let half_angle = angle * 0.5;
        Quat([half_angle.cos(), 0.0, half_angle.sin(), 0.0])
    }

    #[inline]
    pub fn z_rotation(angle: f32) -> Self {
        let half_angle = angle * 0.5;
        Quat([half_angle.cos(), 0.0, 0.0, half_angle.sin()])
    }

    #[inline]
    pub fn w(&self) -> f32 {
        self.0[0]
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.0[1]
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.0[2]
    }

    #[inline]
    pub fn z(&self) -> f32 {
        self.0[3]
    }

    #[inline]
    pub fn x_axis(&self) -> Vec3f {
        self.rotate_vector(&Vec3::new(1.0, 0.0, 0.0))
    }

    #[inline]
    pub fn y_axis(&self) -> Vec3f {
        self.rotate_vector(&Vec3::new(0.0, 1.0, 0.0))
    }

    #[inline]
    pub fn z_axis(&self) -> Vec3f {
        self.rotate_vector(&Vec3::new(0.0, 0.0, 1.0))
    }

    #[inline]
    pub fn pitch(&self) -> f32 {
        let q = &self.0;
        let y = q[2] * q[2] + q[0] * q[0] - 0.5;
        let z = q[3] * q[2] - q[1] * q[0];
        f32::atan2(y, z)
    }

    #[inline]
    pub fn renormalize(&mut self) {
        let values = &mut self.0;
        let norm_squared = values[0] * values[0] + values[1] * values[1] + values[2] * values[2] +
            values[3] * values[3];
        if norm_squared <= f32::EPSILON {
            *values = [1.0, 0.0, 0.0, 0.0];
        } else {
            let inverse_norm = norm_squared.sqrt().recip();
            values[0] *= inverse_norm;
            values[1] *= inverse_norm;
            values[2] *= inverse_norm;
            values[3] *= inverse_norm;
        }
    }

    #[inline]
    pub fn conjugate(&self) -> Self {
        let values = &self.0;
        Quat([values[0], -values[1], -values[2], -values[3]])
    }

    #[inline]
    pub fn rotate_vector(&self, vector: &Vec3f) -> Vec3f {
        // We need to compute (p * v * p.conjugate()). First we do `p * v`:
        let left = (self * vector).0;
        let right = &self.0;

        // Instead of doing the result of that times `p.conjugate`, we can be more efficient by
        // skipping the computation for the first element (which we know will be zero). We also roll
        // the conjugate into the product by reversing the sign of everything with `right[1 ... 3]`.
        let ax = left[0] * right[1];
        let ay = left[0] * right[2];
        let az = left[0] * right[3];

        let bt = left[1] * right[0];
        let by = left[1] * right[2];
        let bz = left[1] * right[3];

        let ct = left[2] * right[0];
        let cx = left[2] * right[1];
        let cz = left[2] * right[3];

        let dt = left[3] * right[0];
        let dx = left[3] * right[1];
        let dy = left[3] * right[2];

        Vec3::new(bt - ax - cz + dy, ct - ay - dx + bz, dt - az - by + cx)
    }
}

impl<'a, 'b> Mul<&'a Quat> for &'b Quat {
    type Output = Quat;

    #[inline]
    fn mul(self, other: &'a Quat) -> Quat {
        let (left, right) = (&self.0, &other.0);

        let at = left[0] * right[0];
        let ax = left[0] * right[1];
        let ay = left[0] * right[2];
        let az = left[0] * right[3];

        let bt = left[1] * right[0];
        let bx = left[1] * right[1];
        let by = left[1] * right[2];
        let bz = left[1] * right[3];

        let ct = left[2] * right[0];
        let cx = left[2] * right[1];
        let cy = left[2] * right[2];
        let cz = left[2] * right[3];

        let dt = left[3] * right[0];
        let dx = left[3] * right[1];
        let dy = left[3] * right[2];
        let dz = left[3] * right[3];

        Quat(
            [
                at - bx - cy - dz,
                bt + ax + cz - dy,
                ct + ay + dx - bz,
                dt + az + by - cx,
            ],
        )
    }
}

impl<'a, 'b> Mul<&'a Vec3f> for &'b Quat {
    type Output = Quat;

    #[inline]
    fn mul(self, other: &'a Vec3f) -> Quat {
        let (left, right) = (&self.0, &other.0);

        let ax = left[0] * right[0];
        let ay = left[0] * right[1];
        let az = left[0] * right[2];

        let bx = left[1] * right[0];
        let by = left[1] * right[1];
        let bz = left[1] * right[2];

        let cx = left[2] * right[0];
        let cy = left[2] * right[1];
        let cz = left[2] * right[2];

        let dx = left[3] * right[0];
        let dy = left[3] * right[1];
        let dz = left[3] * right[2];

        Quat([-bx - cy - dz, ax + cz - dy, ay + dx - bz, az + by - cx])
    }
}

impl Mul<Quat> for Quat {
    type Output = Quat;

    #[inline]
    fn mul(self, other: Quat) -> Self {
        &self * &other
    }
}


impl<'a> From<&'a EulerAngles> for Quat {
    #[inline]
    fn from(angles: &'a EulerAngles) -> Self {
        let half_yaw = angles.yaw * 0.5;
        let half_pitch = angles.pitch * 0.5;
        let half_roll = angles.roll * 0.5;

        let cos_yaw = half_yaw.cos();
        let sin_yaw = half_yaw.sin();
        let cos_roll = half_roll.cos();
        let sin_roll = half_roll.sin();
        let cos_pitch = half_pitch.cos();
        let sin_pitch = half_pitch.sin();

        let mut quat = Quat(
            [
                cos_yaw * cos_roll * cos_pitch + sin_yaw * sin_roll * sin_pitch,

                cos_yaw * cos_roll * sin_pitch + sin_yaw * sin_roll * cos_pitch,
                sin_yaw * cos_roll * cos_pitch - cos_yaw * sin_roll * sin_pitch,
                cos_yaw * sin_roll * cos_pitch - sin_yaw * cos_roll * sin_pitch,
            ],
        );
        quat.renormalize();
        quat
    }
}

impl From<EulerAngles> for Quat {
    #[inline]
    fn from(angles: EulerAngles) -> Self {
        Self::from(&angles)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Transform {
    pub rotation: Quat,
    pub translation: Vec3f,
    pub scale: Vec3f,
}

impl Transform {
    pub fn inverse(&self) -> Self {
        // TODO(cristicbz): Invert scale!
        let rotation = self.rotation.conjugate();
        let translation = -rotation.rotate_vector(&self.translation);
        Transform {
            rotation,
            translation,
            scale: self.scale,
        }
    }

    pub fn then(&self, child: &Transform) -> Self {
        Transform {
            rotation: self.rotation * child.rotation,
            translation: self.translation + self.rotation.rotate_vector(&child.translation),
            scale: self.scale * child.scale,
        }
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Transform {
            rotation: Quat::identity(),
            translation: Vec3::zero(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl<'a> From<&'a Transform> for Mat4 {
    #[inline]
    fn from(transform: &'a Transform) -> Mat4 {
        let rotation = &transform.rotation.0;
        let translation = &transform.translation.0;
        let scale = &transform.scale.0;

        let aa = rotation[0] * rotation[0];
        let ab = rotation[0] * rotation[1];
        let ac = rotation[0] * rotation[2];
        let ad = rotation[0] * rotation[3];

        let bb = rotation[1] * rotation[1];
        let bc = rotation[1] * rotation[2];
        let bd = rotation[1] * rotation[3];

        let cc = rotation[2] * rotation[2];
        let cd = rotation[2] * rotation[3];

        let dd = rotation[3] * rotation[3];

        Mat4::new(
            (aa + bb - cc - dd) * scale[0],
            2.0 * (bc - ad) * scale[0],
            2.0 * (bd + ac) * scale[0],
            translation[0],

            2.0 * (bc + ad) * scale[1],
            (aa - bb + cc - dd) * scale[1],
            2.0 * (cd - ab) * scale[1],
            translation[1],

            2.0 * (bd - ac) * scale[2],
            2.0 * (cd + ab) * scale[2],
            (aa - bb - cc + dd) * scale[2],
            translation[2],

            0.0,
            0.0,
            0.0,
            1.0,
        )
    }
}

impl From<Quat> for Mat4 {
    #[inline]
    fn from(quat: Quat) -> Mat4 {
        Self::from(Transform {
            rotation: quat,
            ..Transform::default()
        })
    }
}

impl From<Transform> for Mat4 {
    #[inline]
    fn from(transform: Transform) -> Mat4 {
        Self::from(&transform)
    }
}

#[cfg(test)]
mod tests {
    use super::{EulerAngles, Quat};
    use super::super::{Vec3f, Vec3};
    use std::f32::consts::{PI, FRAC_PI_2};

    macro_rules! assert_vec_eq {
        ($left:expr, $right:expr) => {{
            let left: Vec3f = $left;
            let right: Vec3f = $right;
            assert!((left[0] - right[0]).abs() <= ::std::f32::EPSILON &&
                    (left[1] - right[1]).abs() <= ::std::f32::EPSILON &&
                    (left[2] - right[2]).abs() <= ::std::f32::EPSILON,
                    "{:?} != {:?}", left, right);
        }}
    }

    #[test]
    fn euler() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        let mx = Vec3::new(-1.0, 0.0, 0.0);
        let my = Vec3::new(0.0, -1.0, 0.0);
        let mz = Vec3::new(0.0, 0.0, -1.0);

        let identity = Quat::identity();

        let zero = Quat::from(EulerAngles {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
        });

        let yaw_pi = Quat::from(EulerAngles {
            yaw: PI,
            pitch: 0.0,
            roll: 0.0,
        });
        let yaw_pi_2 = Quat::from(EulerAngles {
            yaw: FRAC_PI_2,
            pitch: 0.0,
            roll: 0.0,
        });

        let pitch_pi = Quat::from(EulerAngles {
            yaw: 0.0,
            pitch: PI,
            roll: 0.0,
        });
        let pitch_pi_2 = Quat::from(EulerAngles {
            yaw: 0.0,
            pitch: FRAC_PI_2,
            roll: 0.0,
        });

        let roll_pi = Quat::from(EulerAngles {
            yaw: 0.0,
            pitch: 0.0,
            roll: PI,
        });
        let roll_pi_2 = Quat::from(EulerAngles {
            yaw: 0.0,
            pitch: 0.0,
            roll: FRAC_PI_2,
        });

        assert_vec_eq!(identity.rotate_vector(&x), x);
        assert_vec_eq!(identity.rotate_vector(&y), y);
        assert_vec_eq!(identity.rotate_vector(&z), z);

        assert_vec_eq!(zero.rotate_vector(&x), x);
        assert_vec_eq!(zero.rotate_vector(&y), y);
        assert_vec_eq!(zero.rotate_vector(&z), z);

        assert_vec_eq!(yaw_pi.rotate_vector(&x), mx);
        assert_vec_eq!(yaw_pi.rotate_vector(&y), y);
        assert_vec_eq!(yaw_pi.rotate_vector(&z), mz);

        assert_vec_eq!(yaw_pi_2.rotate_vector(&x), mz);
        assert_vec_eq!(yaw_pi_2.rotate_vector(&y), y);
        assert_vec_eq!(yaw_pi_2.rotate_vector(&z), x);

        assert_vec_eq!(pitch_pi.rotate_vector(&x), x);
        assert_vec_eq!(pitch_pi.rotate_vector(&y), my);
        assert_vec_eq!(pitch_pi.rotate_vector(&z), mz);

        assert_vec_eq!(pitch_pi_2.rotate_vector(&x), x);
        assert_vec_eq!(pitch_pi_2.rotate_vector(&y), z);
        assert_vec_eq!(pitch_pi_2.rotate_vector(&z), my);

        assert_vec_eq!(roll_pi.rotate_vector(&x), mx);
        assert_vec_eq!(roll_pi.rotate_vector(&y), my);
        assert_vec_eq!(roll_pi.rotate_vector(&z), z);

        assert_vec_eq!(roll_pi_2.rotate_vector(&x), y);
        assert_vec_eq!(roll_pi_2.rotate_vector(&y), mx);
        assert_vec_eq!(roll_pi_2.rotate_vector(&z), z);
    }
}
