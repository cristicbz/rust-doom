use num::{Float, One, Zero};
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

pub type Vec2f = Vec2<f32>;
pub type Vec3f = Vec3<f32>;
pub type Vec4f = Vec4<f32>;

pub trait Vector
    : Mul<<Self as Vector>::Scalar, Output = Self>
    + Div<<Self as Vector>::Scalar, Output = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Zero
    + Clone
    + PartialEq
    + PartialOrd
    + Index<usize, Output = <Self as Vector>::Scalar>
    + IndexMut<usize> {
    type Scalar: Field;

    fn dot(&self, rhs: &Self) -> Self::Scalar;

    #[inline]
    fn squared_norm(&self) -> Self::Scalar {
        self.dot(self)
    }

    #[inline]
    fn norm(&self) -> Self::Scalar
    where
        Self::Scalar: Float,
    {
        self.squared_norm().sqrt()
    }

    #[inline]
    fn normalize(&mut self)
    where
        Self::Scalar: Float,
    {
        *self = self.clone().normalized();
    }

    #[inline]
    fn normalized(self) -> Self
    where
        Self::Scalar: Float,
    {
        let norm = self.norm();
        if norm == Self::Scalar::zero() {
            Self::zero()
        } else {
            self / norm
        }
    }
}


pub trait Field
    : Mul<Output = Self>
    + Div<Output = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Zero
    + One
    + Copy
    + Clone
    + PartialEq
    + PartialOrd {
}

impl<S> Field for S
where
    S: Mul<Output = S>
        + Div<Output = S>
        + Add<Output = S>
        + Sub<Output = S>
        + Zero
        + One
        + Copy
        + PartialEq
        + PartialOrd,
{
}


// Vec2
#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Vec2<Scalar: Field>(pub [Scalar; 2]);

impl<Scalar: Field> Vec2<Scalar> {
    #[inline]
    pub fn new(x: Scalar, y: Scalar) -> Self {
        Vec2([x, y])
    }

    #[inline]
    pub fn cross(&self, rhs: &Self) -> Scalar {
        self[0] * rhs[1] - self[1] * rhs[0]
    }

    #[inline]
    pub fn angle(&self) -> Scalar
    where
        Scalar: Float,
    {
        self[1].atan2(self[0])
    }

    #[inline]
    pub fn normal(self) -> Vec2<Scalar>
    where
        Scalar: Neg<Output = Scalar>,
    {
        Vec2::new(-self[1], self[0])
    }

    #[inline]
    pub fn swap(&mut self) {
        self.0.swap(0, 1)
    }
}

impl<Scalar: Field + Neg<Output = Scalar>> Neg for Vec2<Scalar> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Vec2([-self[0], -self[1]])
    }
}

impl<Scalar: Field> Vector for Vec2<Scalar> {
    type Scalar = Scalar;

    #[inline]
    fn dot(&self, rhs: &Self) -> Scalar {
        self[0] * rhs[0] + self[1] * rhs[1]
    }
}

impl<Scalar: Field> Zero for Vec2<Scalar> {
    #[inline]
    fn zero() -> Self {
        Vec2::new(Scalar::zero(), Scalar::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self[0].is_zero() && self[1].is_zero()
    }
}

impl<Scalar: Field> Mul<Scalar> for Vec2<Scalar> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Scalar) -> Self {
        Vec2([self[0] * rhs, self[1] * rhs])
    }
}

impl<Scalar: Field> Div<Scalar> for Vec2<Scalar> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Scalar) -> Self {
        Vec2([self[0] / rhs, self[1] / rhs])
    }
}

impl<Scalar: Field> Add<Vec2<Scalar>> for Vec2<Scalar> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Vec2<Scalar>) -> Self {
        Vec2([self[0] + rhs[0], self[1] + rhs[1]])
    }
}

impl<Scalar: Field> Sub<Vec2<Scalar>> for Vec2<Scalar> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Vec2<Scalar>) -> Self {
        Vec2([self[0] - rhs[0], self[1] - rhs[1]])
    }
}

impl<Scalar: Field> Index<usize> for Vec2<Scalar> {
    type Output = Scalar;

    #[inline]
    fn index(&self, index: usize) -> &Scalar {
        &self.0[index]
    }
}

impl<Scalar: Field> IndexMut<usize> for Vec2<Scalar> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Scalar {
        &mut self.0[index]
    }
}


// Vec3
#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Vec3<Scalar: Field>(pub [Scalar; 3]);

impl<Scalar: Field> Vec3<Scalar> {
    #[inline]
    pub fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Vec3([x, y, z])
    }

    #[inline]
    pub fn cross(self, rhs: Vec3<Scalar>) -> Vec3<Scalar> {
        let (lx, ly, lz) = (self[0], self[1], self[2]);
        let (rx, ry, rz) = (rhs[0], rhs[1], rhs[2]);
        Vec3::new(ly * rz - lz * ry, lz * rx - lx * rz, lx * ry - ly * rx)
    }
}

impl<Scalar: Field> Vector for Vec3<Scalar> {
    type Scalar = Scalar;

    #[inline]
    fn dot(&self, rhs: &Self) -> Scalar {
        self[0] * rhs[0] + self[1] * rhs[1] + self[2] * rhs[2]
    }
}

impl<Scalar: Field> Zero for Vec3<Scalar> {
    #[inline]
    fn zero() -> Self {
        Vec3::new(Scalar::zero(), Scalar::zero(), Scalar::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self[0].is_zero() && self[1].is_zero() && self[2].is_zero()
    }
}

impl<Scalar: Field + Neg<Output = Scalar>> Neg for Vec3<Scalar> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Vec3([-self[0], -self[1], -self[2]])
    }
}

impl<Scalar: Field> Mul<Scalar> for Vec3<Scalar> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Scalar) -> Self {
        Vec3([self[0] * rhs, self[1] * rhs, self[2] * rhs])
    }
}

impl<Scalar: Field> Mul<Vec3<Scalar>> for Vec3<Scalar> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Vec3<Scalar>) -> Self {
        Vec3([self[0] * rhs[0], self[1] * rhs[1], self[2] * rhs[2]])
    }
}

impl<Scalar: Field> Div<Scalar> for Vec3<Scalar> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Scalar) -> Self {
        Vec3([self[0] / rhs, self[1] / rhs, self[2] / rhs])
    }
}

impl<Scalar: Field> Add<Vec3<Scalar>> for Vec3<Scalar> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Vec3<Scalar>) -> Self {
        Vec3([self[0] + rhs[0], self[1] + rhs[1], self[2] + rhs[2]])
    }
}

impl<Scalar: Field> Sub<Vec3<Scalar>> for Vec3<Scalar> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Vec3<Scalar>) -> Self {
        Vec3([self[0] - rhs[0], self[1] - rhs[1], self[2] - rhs[2]])
    }
}

impl<Scalar: Field> Index<usize> for Vec3<Scalar> {
    type Output = Scalar;

    #[inline]
    fn index(&self, index: usize) -> &Scalar {
        &self.0[index]
    }
}

impl<Scalar: Field> IndexMut<usize> for Vec3<Scalar> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Scalar {
        &mut self.0[index]
    }
}


// Vec4
#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Vec4<Scalar: Field>(pub [Scalar; 4]);

impl<Scalar: Field> Vec4<Scalar> {
    #[inline]
    pub fn new(x: Scalar, y: Scalar, z: Scalar, w: Scalar) -> Self {
        Vec4([x, y, z, w])
    }
}

impl<Scalar: Field> Vector for Vec4<Scalar> {
    type Scalar = Scalar;

    #[inline]
    fn dot(&self, rhs: &Self) -> Scalar {
        self[0] * rhs[0] + self[1] * rhs[1] + self[2] * rhs[2] + self[3] * rhs[3]
    }
}

impl<Scalar: Field> Zero for Vec4<Scalar> {
    #[inline]
    fn zero() -> Self {
        Vec4::new(
            Scalar::zero(),
            Scalar::zero(),
            Scalar::zero(),
            Scalar::zero(),
        )
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self[0].is_zero() && self[1].is_zero() && self[2].is_zero() && self[3].is_zero()
    }
}

impl<Scalar: Field + Neg<Output = Scalar>> Neg for Vec4<Scalar> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Vec4([-self[0], -self[1], -self[2], -self[3]])
    }
}

impl<Scalar: Field> Mul<Scalar> for Vec4<Scalar> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Scalar) -> Self {
        Vec4([self[0] * rhs, self[1] * rhs, self[2] * rhs, self[3] * rhs])
    }
}

impl<Scalar: Field> Div<Scalar> for Vec4<Scalar> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Scalar) -> Self {
        Vec4([self[0] / rhs, self[1] / rhs, self[2] / rhs, self[3] / rhs])
    }
}

impl<Scalar: Field> Add<Vec4<Scalar>> for Vec4<Scalar> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Vec4<Scalar>) -> Self {
        Vec4(
            [
                self[0] + rhs[0],
                self[1] + rhs[1],
                self[2] + rhs[2],
                self[3] + rhs[3],
            ],
        )
    }
}

impl<Scalar: Field> Sub<Vec4<Scalar>> for Vec4<Scalar> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Vec4<Scalar>) -> Self {
        Vec4(
            [
                self[0] - rhs[0],
                self[1] - rhs[1],
                self[2] - rhs[2],
                self[3] - rhs[3],
            ],
        )
    }
}

impl<Scalar: Field> Index<usize> for Vec4<Scalar> {
    type Output = Scalar;

    #[inline]
    fn index(&self, index: usize) -> &Scalar {
        &self.0[index]
    }
}

impl<Scalar: Field> IndexMut<usize> for Vec4<Scalar> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Scalar {
        &mut self.0[index]
    }
}
