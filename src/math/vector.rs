use num::{Float, One, Zero};
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Add, Div, Index, IndexMut, Mul, Neg, Sub};

pub type Vec2f = Vec2<f32>;
pub type Vec3f = Vec3<f32>;
pub type Vec4f = Vec4<f32>;

pub type Vec2<T> = VectorCons<T, VectorCons<T, VectorNil<T>>>;
pub type Vec3<T> = VectorCons<T, Vec2<T>>;
pub type Vec4<T> = VectorCons<T, Vec3<T>>;


pub trait Vector: Mul<<Self as Vector>::Scalar, Output=Self>
                + Div<<Self as Vector>::Scalar, Output=Self>
                + Add<Output=Self> + Sub<Output=Self> + Zero
                + Clone + PartialEq + PartialOrd
                + Index<usize, Output=<Self as Vector>::Scalar>
                + IndexMut<usize> {

    type Scalar: Field;

    fn dot(&self, rhs: &Self) -> Self::Scalar;

    fn get(&self, index: usize) -> Option<&Self::Scalar>;

    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Scalar>;

    fn len(&self) -> usize;

    #[inline]
    fn squared_norm(&self) -> Self::Scalar {
        self.dot(self)
    }

    #[inline]
    fn norm(&self) -> Self::Scalar
        where Self::Scalar: Float
    {
        self.squared_norm().sqrt()
    }

    #[inline]
    fn normalize(&mut self)
        where Self::Scalar: Float
    {
        *self = self.clone().normalized();
    }

    #[inline]
    fn normalized(self) -> Self
        where Self::Scalar: Float
    {
        let norm = self.norm();
        if norm == Self::Scalar::zero() {
            Self::zero()
        } else {
            self / norm
        }
    }
}


pub trait Field: Mul<Output=Self> + Div<Output=Self>
               + Add<Output=Self> + Sub<Output=Self>
               + Zero + One + Clone + PartialEq + PartialOrd {}

impl<S> Field for S
    where S: Mul<Output=S> + Div<Output=S>
           + Add<Output=S> + Sub<Output=S>
           + Zero + One + Clone + PartialEq + PartialOrd {}


#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct VectorNil<Scalar: Field>(PhantomData<Scalar>);

impl<Scalar: Field> Mul<Scalar> for VectorNil<Scalar> {
    type Output = Self;
    #[inline]
    fn mul(self, _rhs: Scalar) -> Self {
        self
    }
}

impl<Scalar: Field> Div<Scalar> for VectorNil<Scalar> {
    type Output = Self;
    #[inline]
    fn div(self, _rhs: Scalar) -> Self {
        self
    }
}

impl<Scalar: Field> Add for VectorNil<Scalar> {
    type Output = Self;
    #[inline]
    fn add(self, _rhs: Self) -> Self {
        self
    }
}

impl<Scalar: Field> Sub for VectorNil<Scalar> {
    type Output = Self;
    #[inline]
    fn sub(self, _rhs: Self) -> Self {
        self
    }
}

impl<Scalar> Neg for VectorNil<Scalar>
        where Scalar: Field + Neg<Output=Scalar> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        self
    }
}

impl<Scalar: Field> Zero for VectorNil<Scalar> {
    #[inline]
    fn zero() -> Self {
        VectorNil(PhantomData)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        true
    }
}

impl<Scalar: Field> Index<usize> for VectorNil<Scalar> {
    type Output = Scalar;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        panic!("index out of bounds: the len is 0 but the index is {}",
               index);
    }
}

impl<Scalar: Field> IndexMut<usize> for VectorNil<Scalar> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        panic!("index out of bounds: the len is 0 but the index is {}",
               index);
    }
}

impl<Scalar: Field> Vector for VectorNil<Scalar> {
    type Scalar = Scalar;

    #[inline]
    fn dot(&self, _rhs: &Self) -> Scalar {
        Scalar::zero()
    }

    #[inline]
    fn get(&self, _index: usize) -> Option<&Scalar> {
        None
    }

    #[inline]
    fn get_mut(&mut self, _index: usize) -> Option<&mut Scalar> {
        None
    }

    #[inline]
    fn len(&self) -> usize {
        0
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct VectorCons<Scalar: Field, Tail: Vector<Scalar = Scalar>>(Scalar, Tail);

impl<Scalar, Tail> Mul<Scalar> for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Scalar) -> Self {
        VectorCons(self.0 * rhs.clone(), self.1 * rhs)
    }
}

impl<Scalar, Tail> Div<Scalar> for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Scalar) -> Self {
        VectorCons(self.0 / rhs.clone(), self.1 / rhs)
    }
}

impl<Scalar, Tail> Add for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        VectorCons(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl<Scalar, Tail> Sub for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        VectorCons(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl<Scalar, Tail> Neg for VectorCons<Scalar, Tail>
        where Scalar: Field + Neg<Output=Scalar>,
              Tail: Vector<Scalar=Scalar> + Neg<Output=Tail> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        VectorCons(-self.0, -self.1)
    }
}

impl<Scalar, Tail> Zero for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    #[inline]
    fn zero() -> Self {
        VectorCons(Scalar::zero(), Tail::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero() && self.1.is_zero()
    }
}

impl<Scalar, Tail> Index<usize> for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    type Output = Scalar;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| {
                panic!("index out of bounds: the len is {} but the index is {}",
                       self.len(),
                       index);
            })
    }
}

impl<Scalar, Tail> IndexMut<usize> for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.len();
        self.get_mut(index)
            .unwrap_or_else(|| {
                panic!("index out of bounds: the len is {} but the index is {}",
                       len,
                       index);
            })
    }
}

impl<Scalar, Tail> Vector for VectorCons<Scalar, Tail>
        where Scalar: Field, Tail: Vector<Scalar=Scalar> {
    type Scalar = Scalar;

    #[inline]
    fn dot(&self, rhs: &Self) -> Scalar {
        self.0.clone() * rhs.0.clone() + self.1.dot(&rhs.1)
    }

    #[inline]
    fn len(&self) -> usize {
        1 + self.1.len()
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&Scalar> {
        if index == 0 {
            Some(&self.0)
        } else {
            self.1.get(index - 1)
        }
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut Scalar> {
        if index == 0 {
            Some(&mut self.0)
        } else {
            self.1.get_mut(index - 1)
        }
    }
}

impl<Scalar: Field> Vec2<Scalar> {
    #[inline]
    pub fn new(x: Scalar, y: Scalar) -> Self {
        VectorCons(x, VectorCons(y, VectorNil(PhantomData)))
    }

    #[inline]
    pub fn cross(&self, rhs: &Self) -> Scalar {
        self.0.clone() * (rhs.1).0.clone() - (self.1).0.clone() * rhs.0.clone()
    }

    #[inline]
    pub fn angle(&self) -> Scalar
        where Scalar: Float
    {
        self.0.clone().atan2((self.1).0.clone())
    }

    #[inline]
    pub fn normal(self) -> Vec2<Scalar>
        where Scalar: Neg<Output = Scalar>
    {
        Vec2::new(-(self.1).0, self.0)
    }

    #[inline]
    pub fn swap(&mut self) {
        mem::swap(&mut self.0, &mut (self.1).0);
    }
}

impl<Scalar: Field> Vec3<Scalar> {
    #[inline]
    pub fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        VectorCons(x, VectorCons(y, VectorCons(z, VectorNil(PhantomData))))
    }

    #[inline]
    pub fn cross(self, rhs: Vec3<Scalar>) -> Vec3<Scalar> {
        let (lx, ly, lz) = (self.0, (self.1).0, ((self.1).1).0);
        let (rx, ry, rz) = (rhs.0, (rhs.1).0, ((rhs.1).1).0);
        let (lx2, ly2, lz2) = (lx.clone(), ly.clone(), lz.clone());
        let (rx2, ry2, rz2) = (rx.clone(), ry.clone(), rz.clone());
        Vec3::new(ly * rz - lz * ry,
                  lz2 * rx - lx * rz2,
                  lx2 * ry2 - ly2 * rx2)
    }
}

impl<Scalar: Field> Debug for VectorNil<Scalar> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "[]")
    }
}

impl<Scalar, Tail> Debug for VectorCons<Scalar, Tail>
        where Scalar: Field + Debug, Tail: Vector<Scalar=Scalar> + FmtTail {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(fmt, "[{:?}", self.0));
        self.1.fmt_tail(fmt)
    }
}

pub trait FmtTail {
    fn fmt_tail(&self, fmt: &mut fmt::Formatter) -> fmt::Result;
}

impl<Scalar: Field> FmtTail for VectorNil<Scalar> {
    fn fmt_tail(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "]")
    }
}

impl<Scalar, Tail> FmtTail for VectorCons<Scalar, Tail>
        where Scalar: Field + Debug, Tail: Vector<Scalar=Scalar> + FmtTail {
    fn fmt_tail(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(fmt, ", {:?}", self.0));
        self.1.fmt_tail(fmt)
    }
}
