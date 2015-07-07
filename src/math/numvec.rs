use num::Float;
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::fmt;
use std::fmt::Debug;

pub type Vec2f = Vec2<f32>;
pub type Vec3f = Vec3<f32>;
pub type Vec4f = Vec4<f32>;

pub trait Numvec<T: Float + Copy>
       : Mul<T, Output = Self> + Div<T, Output = Self> + Copy + Sized {
    fn dot(&self, other: &Self) -> T;

    fn squared_norm(&self) -> T { self.dot(self) }
    fn norm(&self) -> T { self.squared_norm().sqrt() }

    fn normalize(&mut self) {
        *self = self.normalized();
    }

    fn normalized(&self) -> Self {
        let norm = self.norm();
        *self / norm
    }
}

#[repr(C, packed)]
#[derive(PartialEq, Copy, Clone)]
pub struct Vec2<T: Float + Sized + PartialEq> {
    pub x: T,
    pub y: T,
}
impl<T: Float + Sized + PartialEq> Vec2<T> {
    pub fn new(x: T, y: T) -> Vec2<T> { Vec2 { x: x, y: y } }

    pub fn zero()   -> Vec2<T> { Vec2 { x: T::zero(), y: T::zero() } }
    pub fn x_axis() -> Vec2<T> { Vec2 { x:  T::one(), y: T::zero() } }
    pub fn y_axis() -> Vec2<T> { Vec2 { x: T::zero(), y:  T::one() } }

    pub fn cross(&self, rhs: &Vec2<T>) -> T {
        self.x * rhs.y - self.y * rhs.x
    }

    pub fn angle(&self) -> T {
        self.y.atan2(self.x)
    }

    pub fn normal(&self) -> Vec2<T> {
        Vec2::new(-self.y, self.x)
    }
}
impl<T: Float + Sized + PartialEq> Numvec<T> for Vec2<T> {
    fn dot(&self, rhs: &Vec2<T>) -> T {
        self.x * rhs.x + self.y * rhs.y
    }
}
impl<T: Float + Sized + PartialEq> Add for Vec2<T> {
    type Output = Vec2<T>;

    fn add(self, rhs: Vec2<T>) -> Vec2<T> {
        return Vec2::new(self.x + rhs.x, self.y + rhs.y);
    }
}
impl<T: Float + Sized + PartialEq> Sub for Vec2<T> {
    type Output = Vec2<T>;

    fn sub(self, rhs: Vec2<T>) -> Vec2<T> {
        return Vec2::new(self.x - rhs.x, self.y - rhs.y);
    }
}
impl<T: Float + Sized + PartialEq> Mul<T> for Vec2<T> {
    type Output = Vec2<T>;

    fn mul(self, rhs: T) -> Vec2<T> {
        return Vec2::new(self.x * rhs, self.y * rhs);
    }
}
impl<T: Float + Sized + PartialEq> Div<T> for Vec2<T> {
    type Output = Vec2<T>;

    fn div(self, rhs: T) -> Vec2<T> {
        return Vec2::new(self.x / rhs, self.y / rhs);
    }
}
impl<T: Float + Sized + PartialEq> Neg for Vec2<T> {
    type Output = Vec2<T>;

    fn neg(self) -> Vec2<T> {
        return Vec2::new(-self.x, -self.y);
    }
}
impl<T: Float  + Debug> Debug for Vec2<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Vec2({:?}, {:?})", self.x, self.y)
    }
}


#[repr(C, packed)]
#[derive(PartialEq, Copy, Clone)]
pub struct Vec3<T: Float + Sized + PartialEq> {
    pub x: T,
    pub y: T,
    pub z: T,
}
impl<T: Float + Sized + PartialEq> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Vec3<T> { Vec3 { x: x, y: y, z: z } }

    pub fn zero()   -> Vec3<T> {
        Vec3 { x: T::zero(), y: T::zero(), z: T::zero() }
    }

    pub fn x_axis() -> Vec3<T> {
        Vec3 { x: T::one(), y: T::zero(), z: T::zero() }
    }

    pub fn y_axis() -> Vec3<T> {
        Vec3 { x: T::zero(), y: T::one(), z: T::zero() }
    }

    pub fn z_axis() -> Vec3<T> {
        Vec3 { x: T::zero(), y: T::zero(), z:  T::one() }
    }

    pub fn cross(&self, rhs: &Vec3<T>) -> Vec3<T> {
        Vec3 { x: self.y * rhs.z - self.z * rhs.y,
               y: self.z * rhs.x - self.x * rhs.z,
               z: self.x * rhs.y - self.y * rhs.x }
    }
}
impl<T: Float + Sized + PartialEq> Numvec<T> for Vec3<T> {
    fn dot(&self, rhs: &Vec3<T>) -> T {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
}
impl<T: Float + Sized + PartialEq> Add for Vec3<T> {
    type Output = Vec3<T>;

    fn add(self, rhs: Vec3<T>) -> Vec3<T> {
        return Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z);
    }
}
impl<T: Float + Sized + PartialEq> Sub for Vec3<T> {
    type Output = Vec3<T>;

    fn sub(self, rhs: Vec3<T>) -> Vec3<T> {
        return Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z);
    }
}
impl<T: Float + Sized + PartialEq> Mul<T> for Vec3<T> {
    type Output = Vec3<T>;

    fn mul(self, rhs: T) -> Vec3<T> {
        return Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs);
    }
}
impl<T: Float + Sized + PartialEq> Div<T> for Vec3<T> {
    type Output = Vec3<T>;

    fn div(self, rhs: T) -> Vec3<T> {
        return Vec3::new(self.x / rhs, self.y / rhs, self.z / rhs);
    }
}
impl<T: Float + Sized + PartialEq> Neg for Vec3<T> {
    type Output = Vec3<T>;

    fn neg(self) -> Vec3<T> {
        return Vec3::new(-self.x, -self.y, -self.z);
    }
}
impl<T: Float + Debug> Debug for Vec3<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Vec3({:?}, {:?}, {:?})", self.x, self.y, self.z)
    }
}


#[repr(C, packed)]
#[derive(PartialEq, Copy, Clone)]
pub struct Vec4<T: Float + Sized + PartialEq> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}
impl<T: Float + Sized + PartialEq> Vec4<T> {
    pub fn new(x: T, y: T, z: T, w: T) -> Vec4<T> {
        Vec4 { x: x, y: y, z: z, w: w }
    }

    pub fn zero()   -> Vec4<T> {
        Vec4 { x: T::zero(), y: T::zero(), z: T::zero(), w: T::zero() }
    }

    pub fn x_axis() -> Vec4<T> {
        Vec4 { x: T::one(), y: T::zero(), z: T::zero(), w: T::zero() }
    }

    pub fn y_axis() -> Vec4<T> {
        Vec4 { x: T::zero(), y: T::one(), z: T::zero(), w: T::zero() }
    }

    pub fn z_axis() -> Vec4<T> {
        Vec4 { x: T::zero(), y: T::zero(), z: T::one(), w: T::zero() }
    }

    pub fn w_axis() -> Vec4<T> {
        Vec4 { x: T::zero(), y: T::zero(), z: T::zero(), w: T::one() }
    }
}
impl<T: Float + Sized + PartialEq> Numvec<T> for Vec4<T> {
    fn dot(&self, rhs: &Vec4<T>) -> T {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }
}
impl<T: Float + Sized + PartialEq> Add for Vec4<T> {
    type Output = Vec4<T>;

    fn add(self, rhs: Vec4<T>) -> Vec4<T> {
        return Vec4::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z, self.w + rhs.w);
    }
}
impl<T: Float + Sized + PartialEq> Sub for Vec4<T> {
    type Output = Vec4<T>;

    fn sub(self, rhs: Vec4<T>) -> Vec4<T> {
        return Vec4::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z, self.w - rhs.w);
    }
}
impl<T: Float + Sized + PartialEq> Mul<T> for Vec4<T> {
    type Output = Vec4<T>;

    fn mul(self, rhs: T) -> Vec4<T> {
        return Vec4::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs);
    }
}
impl<T: Float + Sized + PartialEq> Div<T> for Vec4<T> {
    type Output = Vec4<T>;

    fn div(self, rhs: T) -> Vec4<T> {
        return Vec4::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs);
    }
}
impl<T: Float + Sized + PartialEq> Neg for Vec4<T> {
    type Output = Vec4<T>;

    fn neg(self) -> Vec4<T> {
        return Vec4::new(-self.x, -self.y, -self.z, -self.w);
    }
}
