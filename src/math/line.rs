use num::{Float, NumCast};
use vector::{Field, Vec2, Vector};

pub type Line2f = Line2<f32>;

#[derive(Copy, Clone)]
pub struct Line2<T: Copy + Field + Float + NumCast> {
    origin: Vec2<T>,
    displace: Vec2<T>,
}
impl<T: Copy + Field + Float + NumCast> Line2<T> {
    pub fn from_origin_and_displace(origin: Vec2<T>, displace: Vec2<T>) -> Line2<T> {
        Line2 {
            origin: origin,
            displace: displace.normalized(),
        }
    }

    pub fn from_two_points(origin: Vec2<T>, towards: Vec2<T>) -> Line2<T> {
        Line2 {
            origin: origin,
            displace: (towards - origin).normalized(),
        }
    }

    pub fn inverted_halfspaces(&self) -> Line2<T> {
        Line2 {
            origin: self.origin,
            displace: -self.displace,
        }
    }

    pub fn signed_distance(&self, to: &Vec2<T>) -> T {
        to.cross(&self.displace) + self.displace.cross(&self.origin)
    }

    pub fn intersect_offset(&self, other: &Line2<T>) -> Option<T> {
        let numerator = self.displace.cross(&other.displace);
        if numerator.abs() < <T as NumCast>::from(1e-16).unwrap() {
            None
        } else {
            Some((other.origin - self.origin).cross(&other.displace) / numerator)
        }
    }

    pub fn intersect_point(&self, other: &Line2<T>) -> Option<Vec2<T>> {
        self.intersect_offset(other).map(|offset| self.at_offset(offset))
    }

    pub fn at_offset(&self, offset: T) -> Vec2<T> {
        self.origin + self.displace * offset
    }
}
