use super::vector::{Field, Vec2, Vector};
use num::{Float, NumCast, Zero};

pub type Line2f = Line2<f32>;

#[derive(Copy, Clone, Debug)]
pub struct Line2<T: Copy + Field + Float + NumCast> {
    pub origin: Vec2<T>,
    pub displace: Vec2<T>,
    pub length: T,
}
impl<T: Copy + Field + Float + NumCast> Line2<T> {
    pub fn from_origin_and_displace(origin: Vec2<T>, displace: Vec2<T>) -> Line2<T> {
        let length = displace.norm();
        if length.abs() >= <T as NumCast>::from(1e-16).unwrap() {
            Line2 {
                origin: origin,
                displace: displace / length,
                length,
            }
        } else {
            Line2 {
                origin: origin,
                displace: Vec2::zero(),
                length: T::zero(),
            }
        }
    }

    pub fn from_two_points(origin: Vec2<T>, towards: Vec2<T>) -> Line2<T> {
        Self::from_origin_and_displace(origin, towards - origin)
    }

    pub fn inverted_halfspaces(&self) -> Line2<T> {
        Line2 {
            origin: self.origin,
            displace: -self.displace,
            length: self.length,
        }
    }

    pub fn signed_distance(&self, to: &Vec2<T>) -> T {
        to.cross(&self.displace) + self.displace.cross(&self.origin)
    }

    pub fn segment_intersect_offset(&self, other: &Line2<T>) -> Option<T> {
        self.intersect_offset(other).and_then(|offset| {
            if offset < T::zero() || offset >= self.length {
                return None;
            }

            let other_offset = other.offset_at(&self.at_offset(offset));
            if other_offset < T::zero() || other_offset >= other.length {
                return None;
            }

            Some(offset)
        })
    }

    pub fn offset_at(&self, point: &Vec2<T>) -> T {
        if self.displace[0].abs() > self.displace[1].abs() {
            (point[0] - self.origin[0]) / self.displace[0]
        } else {
            (point[1] - self.origin[1]) / self.displace[1]
        }
    }

    pub fn intersect_offset(&self, other: &Line2<T>) -> Option<T> {
        let denominator = self.displace.cross(&other.displace);
        if denominator.abs() < <T as NumCast>::from(1e-16).unwrap() {
            None
        } else {
            Some(
                (other.origin - self.origin).cross(&other.displace) / denominator,
            )
        }
    }

    pub fn intersect_point(&self, other: &Line2<T>) -> Option<Vec2<T>> {
        self.intersect_offset(other).map(
            |offset| self.at_offset(offset),
        )
    }

    pub fn at_offset(&self, offset: T) -> Vec2<T> {
        self.origin + self.displace * offset
    }
}
