use num::Float;
use std::fmt;
use std::ops::{Add, Sub, Mul};

use numvec::Vec3f;

/// The type of matrix elements.
pub type Scalar = f32;

/// A 4x4 matrix type stored in column-major order for interoperability with
/// OpenGL.
///
/// Supports the creation of isometries and projections in homogenous
/// coordinates.  In terms of operations, only transposition and multiplication
/// are currently supported (and not super-efficiently implemented).
///
/// _Note:_ The 16 elements are stored in place, so copies are not cheap.
#[repr(packed)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Mat4 {
    data: [Scalar; 16]
}

impl Mat4 {
    pub fn new(m00: Scalar, m01: Scalar, m02: Scalar, m03: Scalar,
               m10: Scalar, m11: Scalar, m12: Scalar, m13: Scalar,
               m20: Scalar, m21: Scalar, m22: Scalar, m23: Scalar,
               m30: Scalar, m31: Scalar, m32: Scalar, m33: Scalar) -> Mat4 {
        // In my mind vectors are columns, hence matrices need to be transposed
        // to the OpenGL memory order.
        Mat4 { data: [m00, m10, m20, m30, m01, m11, m21, m31,
                      m02, m12, m22, m32, m03, m13, m23, m33] }
    }

    pub fn new_identity() -> Mat4 {
        Mat4::new(1.0, 0.0, 0.0, 0.0,
                  0.0, 1.0, 0.0, 0.0,
                  0.0, 0.0, 1.0, 0.0,
                  0.0, 0.0, 0.0, 1.0)
    }

    /// Creates a perspective projection matrix from.
    ///
    /// The parameters are:
    /// * `fov_degrees`  - Horizontal field of view.
    /// * `aspect_ratio` - Ratio between width and height of the view.
    /// * `near`, `far`  - The Z coordinate of the near and far planes.
    pub fn new_perspective(fov_degrees: Scalar, aspect_ratio: Scalar,
                           near: Scalar, far: Scalar) -> Mat4 {
        let fov = (3.1415926538 * fov_degrees) / 180.0;
        let f = 1.0 / (fov * 0.5).tan();

        Mat4::new(
            f / aspect_ratio, 0.0, 0.0, 0.0,
            0.0, f, 0.0, 0.0,
            0.0, 0.0, (far + near) / (near - far), 2.0*far*near / (near - far),
            0.0, 0.0, -1.0, 0.0)
    }

    /// Creates a matrix which rotates points by `angle_radians` around `axis`.
    pub fn new_axis_rotation(axis: &Vec3f, angle_radians: Scalar) -> Mat4 {
        let ca = angle_radians.cos();
        let sa = angle_radians.sin();
        let nca = 1.0 - ca;
        let u = axis;
        Mat4::new(
            ca + u.x*u.x*nca, u.x*u.y*nca - u.z*sa, u.x*u.z*nca + u.y*sa, 0.0,
            u.y*u.x*nca + u.z*sa, ca + u.y*u.y*nca, u.y*u.z*nca - u.x*sa, 0.0,
            u.z*u.x*nca - u.y*sa, u.z*u.y*nca + u.x*sa, ca + u.z*u.z*nca, 0.0,
            0.0, 0.0, 0.0, 1.0
        )
    }

    /// Creates a rotation matrix from the three _Euler angles_.
    pub fn new_euler_rotation(yaw: f32, pitch: f32, roll: f32) -> Mat4 {
        let (ca, sa) = (pitch.cos(), pitch.sin());
        let (cb, sb) = (yaw.cos(), yaw.sin());
        let (cc, sc) = (roll.cos(), roll.sin());

        Mat4::new(
             cb * cc,                -cb * sc,                 sb,      0.0,
             sa * sb * cc + ca * sc, -sa * sb * sc + ca * cc, -sa * cb, 0.0,
            -ca * sb * cc + sa * sc,  ca * sb * sc + sa * cc,  ca * cb, 0.0,
             0.0,                     0.0,                     0.0,     1.0)
    }

    /// Creates a translation matrix which maps points `p` to `p + by`.
    pub fn new_translation(by: Vec3f) -> Mat4 {
        Mat4::new(1.0, 0.0, 0.0, by.x,
                  0.0, 1.0, 0.0, by.y,
                  0.0, 0.0, 1.0, by.z,
                  0.0, 0.0, 0.0,  1.0)
    }

    /// Returns the transpose of the matrix (columns swapped with rows).
    pub fn transposed(&self) -> Mat4 {
        let m = &self.data;
        // m is in column-major order, so calling with new in row-order will
        // transpose it.
        Mat4::new(m[0], m[1], m[2], m[3],
                  m[4], m[5], m[6], m[7],
                  m[8], m[9], m[10], m[11],
                  m[12], m[13], m[14], m[15])
    }

    pub fn get(&self, row: usize, column: usize) -> Scalar {
        self.data[column * 4 + row]
    }

    pub fn as_scalar_ptr(&self) -> *const Scalar {
        self.data.as_ptr()
    }

    pub fn approx_eq(&self, rhs: &Mat4, tol: Scalar) -> bool {
        self.data.iter().zip(rhs.data.iter()).all(|(x, y)| (x - y).abs() <= tol)
    }
}

impl fmt::Debug for Mat4 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter,
               "[{:10.3e} {:10.3e} {:10.3e} {:10.3e};\n\
                 {:10.3e} {:10.3e} {:10.3e} {:10.3e};\n\
                 {:10.3e} {:10.3e} {:10.3e} {:10.3e};\n\
                 {:10.3e} {:10.3e} {:10.3e} {:10.3e}]",
               self.get(0, 0), self.get(0, 1), self.get(0, 2), self.get(0, 3),
               self.get(1, 0), self.get(1, 1), self.get(1, 2), self.get(1, 3),
               self.get(2, 0), self.get(2, 1), self.get(2, 2), self.get(2, 3),
               self.get(3, 0), self.get(3, 1), self.get(3, 2), self.get(3, 3))
    }
}

impl<'a, 'b> Mul<&'a Mat4> for &'b Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: &'a Mat4) -> Mat4 {
        let l = &self.data;
        let r = &rhs.data;
        Mat4 {
            data: [l[0] * r[0] + l[4] * r[1] + l[8] * r[2] + l[12] * r[3],
                   l[1] * r[0] + l[5] * r[1] + l[9] * r[2] + l[13] * r[3],
                   l[2] * r[0] + l[6] * r[1] + l[10] * r[2] + l[14] * r[3],
                   l[3] * r[0] + l[7] * r[1] + l[11] * r[2] + l[15] * r[3],

                   l[0] * r[4] + l[4] * r[5] + l[8] * r[6] + l[12] * r[7],
                   l[1] * r[4] + l[5] * r[5] + l[9] * r[6] + l[13] * r[7],
                   l[2] * r[4] + l[6] * r[5] + l[10] * r[6] + l[14] * r[7],
                   l[3] * r[4] + l[7] * r[5] + l[11] * r[6] + l[15] * r[7],

                   l[0] * r[8] + l[4] * r[9] + l[ 8] * r[10] + l[12] * r[11],
                   l[1] * r[8] + l[5] * r[9] + l[ 9] * r[10] + l[13] * r[11],
                   l[2] * r[8] + l[6] * r[9] + l[10] * r[10] + l[14] * r[11],
                   l[3] * r[8] + l[7] * r[9] + l[11] * r[10] + l[15] * r[11],

                   l[0] * r[12] + l[4] * r[13] + l[ 8] * r[14] + l[12] * r[15],
                   l[1] * r[12] + l[5] * r[13] + l[ 9] * r[14] + l[13] * r[15],
                   l[2] * r[12] + l[6] * r[13] + l[10] * r[14] + l[14] * r[15],
                   l[3] * r[12] + l[7] * r[13] + l[11] * r[14] + l[15] * r[15]],
        }
    }
}

impl<'a, 'b> Add<&'a Mat4> for &'b Mat4 {
    type Output = Mat4;

    fn add(self, rhs: &'a Mat4) -> Mat4 {
        let l = &self.data;
        let r = &rhs.data;
        Mat4 {
            data: [l[0] + r[0], l[1] + r[1], l[2] + r[2], l[3] + r[3],
                   l[4] + r[4], l[5] + r[5], l[6] + r[6], l[7] + r[7],
                   l[8] + r[8], l[9] + r[9], l[10] + r[10], l[11] + r[11],
                   l[12] + r[12], l[13] + r[13], l[14] + r[14], l[15] + r[15]],
        }
    }
}

impl<'a, 'b> Sub<&'a Mat4> for &'b Mat4 {
    type Output = Mat4;

    fn sub(self, rhs: &'a Mat4) -> Mat4 {
        let l = &self.data;
        let r = &rhs.data;
        Mat4 {
            data: [l[0] - r[0], l[1] - r[1], l[2] - r[2], l[3] - r[3],
                   l[4] - r[4], l[5] - r[5], l[6] - r[6], l[7] - r[7],
                   l[8] - r[8], l[9] - r[9], l[10] - r[10], l[11] - r[11],
                   l[12] - r[12], l[13] - r[13], l[14] - r[14], l[15] - r[15]],
        }
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Mat4 { &self * &rhs }
}

impl Add<Mat4> for Mat4 {
    type Output = Mat4;

    fn add(self, rhs: Mat4) -> Mat4 { &self + &rhs }
}

impl Sub<Mat4> for Mat4 {
    type Output = Mat4;

    fn sub(self, rhs: Mat4) -> Mat4 { &self - &rhs }
}

impl PartialEq for Mat4 {
    fn eq(&self, rhs: &Mat4) -> bool {
        self.data.iter().zip(rhs.data.iter()).all(|(x, y)| x == y)
    }
}

#[cfg(test)]
mod test {
    use super::Mat4;

    #[test]
    fn test_mul() {
        let a = Mat4::new(4.0,    8.0,    1.0,    6.0,
                          9.0,    4.0,    2.0,    1.0,
                          4.0,    3.0,    9.0,    3.0,
                          2.0,    4.0,    9.0,    4.0);

        let b = Mat4::new(8.0,    6.0,    5.0,    7.0,
                          1.0,    7.0,    3.0,    2.0,
                          1.0,    6.0,    7.0,    4.0,
                          2.0,    5.0,    2.0,    6.0);


        let exp_ab = Mat4::new(53.0,   116.0,    63.0,    84.0,
                               80.0,    99.0,    73.0,    85.0,
                               50.0,   114.0,    98.0,    88.0,
                               37.0,   114.0,    93.0,    82.0);
        assert_eq!(exp_ab, a * b);
    }
}
