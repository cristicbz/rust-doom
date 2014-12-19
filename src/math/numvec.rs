use std::num::{Float, FloatMath};
use std::ops::{Add, Sub, Mul};
use std::fmt;
use std::fmt::Show;

pub type Vec2f = Vec2<f32>;
pub type Vec3f = Vec3<f32>;
pub type Vec4f = Vec4<f32>;

pub trait Numvec<T : Float + FloatMath + Copy>
        : Add<Self, Self> + Mul<T, Self> + Div<T, Self> + Copy {
    fn dot(&self, other : &Self) -> T;

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

#[packed]
#[repr(C)]
#[deriving(Copy)]
pub struct Vec2<T: Float + FloatMath> {
    pub x : T,
    pub y : T,
}
impl<T : Float + FloatMath> Vec2<T> {
    pub fn new(x: T, y: T) -> Vec2<T> { Vec2 { x: x, y: y } }

    pub fn zero()   -> Vec2<T> { Vec2 { x: Float::zero(), y: Float::zero() } }
    pub fn x_axis() -> Vec2<T> { Vec2 { x:  Float::one(), y: Float::zero() } }
    pub fn y_axis() -> Vec2<T> { Vec2 { x: Float::zero(), y:  Float::one() } }

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
impl<T : Float + FloatMath> Numvec<T> for Vec2<T> {
    fn dot(&self, rhs : &Vec2<T>) -> T {
        self.x * rhs.x + self.y * rhs.y
    }
}
impl<T : Float + FloatMath> Add<Vec2<T>, Vec2<T>> for Vec2<T> {
    fn add(self, rhs : Vec2<T>) -> Vec2<T> {
        return Vec2::new(self.x + rhs.x, self.y + rhs.y);
    }
}
impl<T : Float + FloatMath> Sub<Vec2<T>, Vec2<T>> for Vec2<T> {
    fn sub(self, rhs : Vec2<T>) -> Vec2<T> {
        return Vec2::new(self.x - rhs.x, self.y - rhs.y);
    }
}
impl<T : Float + FloatMath> Mul<T, Vec2<T>> for Vec2<T> {
    fn mul(self, rhs : T) -> Vec2<T> {
        return Vec2::new(self.x * rhs, self.y * rhs);
    }
}
impl<T : Float + FloatMath> Div<T, Vec2<T>> for Vec2<T> {
    fn div(self, rhs : T) -> Vec2<T> {
        return Vec2::new(self.x / rhs, self.y / rhs);
    }
}
impl<T : Float + FloatMath> Neg<Vec2<T>> for Vec2<T> {
    fn neg(&self) -> Vec2<T> {
        return Vec2::new(-self.x, -self.y);
    }
}
impl<T : Float + FloatMath + Show> Show for Vec2<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Vec2({}, {})", self.x, self.y)
    }
}


#[packed]
#[repr(C)]
#[deriving(Copy)]
pub struct Vec3<T: Float + FloatMath> {
    pub x : T,
    pub y : T,
    pub z : T,
}
impl<T : Float + FloatMath> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Vec3<T> { Vec3 { x: x, y: y, z: z } }

    pub fn zero()   -> Vec3<T> { Vec3 { x: Float::zero(), y: Float::zero(), z: Float::zero() } }
    pub fn x_axis() -> Vec3<T> { Vec3 { x:  Float::one(), y: Float::zero(), z: Float::zero() } }
    pub fn y_axis() -> Vec3<T> { Vec3 { x: Float::zero(), y:  Float::one(), z: Float::zero() } }
    pub fn z_axis() -> Vec3<T> { Vec3 { x: Float::zero(), y: Float::zero(), z:  Float::one() } }

    pub fn cross(&self, rhs : &Vec3<T>) -> Vec3<T> {
        Vec3 { x: self.y * rhs.z - self.z * rhs.y,
               y: self.z * rhs.x - self.x * rhs.z,
               z: self.x * rhs.y - self.y * rhs.x }
    }
}
impl<T : Float + FloatMath> Numvec<T> for Vec3<T> {
    fn dot(&self, rhs : &Vec3<T>) -> T {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
}
impl<T : Float + FloatMath> Add<Vec3<T>, Vec3<T>> for Vec3<T> {
    fn add(self, rhs : Vec3<T>) -> Vec3<T> {
        return Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z);
    }
}
impl<T : Float + FloatMath> Sub<Vec3<T>, Vec3<T>> for Vec3<T> {
    fn sub(self, rhs : Vec3<T>) -> Vec3<T> {
        return Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z);
    }
}
impl<T : Float + FloatMath> Mul<T, Vec3<T>> for Vec3<T> {
    fn mul(self, rhs : T) -> Vec3<T> {
        return Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs);
    }
}
impl<T : Float + FloatMath> Div<T, Vec3<T>> for Vec3<T> {
    fn div(self, rhs : T) -> Vec3<T> {
        return Vec3::new(self.x / rhs, self.y / rhs, self.z / rhs);
    }
}
impl<T : Float + FloatMath> Neg<Vec3<T>> for Vec3<T> {
    fn neg(&self) -> Vec3<T> {
        return Vec3::new(-self.x, -self.y, -self.z);
    }
}
impl<T : Float + FloatMath + Show> Show for Vec3<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Vec3({}, {}, {})", self.x, self.y, self.z)
    }
}


#[packed]
#[repr(C)]
#[deriving(Copy)]
pub struct Vec4<T: Float + FloatMath> {
    pub x : T,
    pub y : T,
    pub z : T,
    pub w : T,
}
impl<T : Float + FloatMath> Vec4<T> {
    pub fn new(x: T, y: T, z: T, w: T) -> Vec4<T> {
        Vec4 { x: x, y: y, z: z, w: w }
    }

    pub fn zero()   -> Vec4<T> {
        Vec4 { x: Float::zero(), y: Float::zero(), z: Float::zero(), w: Float::zero() }
    }

    pub fn x_axis() -> Vec4<T> {
        Vec4 { x: Float::one(), y: Float::zero(), z: Float::zero(), w: Float::zero() }
    }

    pub fn y_axis() -> Vec4<T> {
        Vec4 { x: Float::zero(), y: Float::one(), z: Float::zero(), w: Float::zero() }
    }

    pub fn z_axis() -> Vec4<T> {
        Vec4 { x: Float::zero(), y: Float::zero(), z: Float::one(), w: Float::zero() }
    }

    pub fn w_axis() -> Vec4<T> {
        Vec4 { x: Float::zero(), y: Float::zero(), z: Float::zero(), w: Float::one() }
    }
}
impl<T : Float + FloatMath> Numvec<T> for Vec4<T> {
    fn dot(&self, rhs : &Vec4<T>) -> T {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }
}
impl<T : Float + FloatMath> Add<Vec4<T>, Vec4<T>> for Vec4<T> {
    fn add(self, rhs : Vec4<T>) -> Vec4<T> {
        return Vec4::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z,
                         self.w + rhs.w);
    }
}
impl<T : Float + FloatMath> Sub<Vec4<T>, Vec4<T>> for Vec4<T> {
    fn sub(self, rhs : Vec4<T>) -> Vec4<T> {
        return Vec4::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z,
                         self.w - rhs.w);
    }
}
impl<T : Float + FloatMath> Mul<T, Vec4<T>> for Vec4<T> {
    fn mul(self, rhs : T) -> Vec4<T> {
        return Vec4::new(self.x * rhs, self.y * rhs, self.z * rhs,
                         self.w * rhs);
    }
}
impl<T : Float + FloatMath> Div<T, Vec4<T>> for Vec4<T> {
    fn div(self, rhs : T) -> Vec4<T> {
        return Vec4::new(self.x / rhs, self.y / rhs, self.z / rhs,
                         self.w / rhs);
    }
}
impl<T : Float + FloatMath> Neg<Vec4<T>> for Vec4<T> {
    fn neg(&self) -> Vec4<T> {
        return Vec4::new(-self.x, -self.y, -self.z, -self.w);
    }
}
