use super::prelude::*;
use super::{BaseFloat, NumCast, Pnt2, Vec2};

#[derive(Copy, Clone, Debug)]
pub struct Line2<T: BaseFloat> {
    pub origin: Pnt2<T>,
    pub displace: Vec2<T>,
    pub length: T,
}

impl<T: BaseFloat> Line2<T> {
    pub fn from_origin_and_displace(origin: Pnt2<T>, displace: Vec2<T>) -> Line2<T> {
        let length = displace.magnitude();
        if length.abs() >= <T as NumCast>::from(1e-16).unwrap() {
            Line2 {
                origin,
                displace: displace / length,
                length,
            }
        } else {
            Line2 {
                origin,
                displace: Vec2::zero(),
                length: T::zero(),
            }
        }
    }

    pub fn from_two_points(origin: Pnt2<T>, towards: Pnt2<T>) -> Line2<T> {
        Self::from_origin_and_displace(origin, towards - origin)
    }

    pub fn inverted_halfspaces(&self) -> Line2<T> {
        Line2 {
            origin: self.origin,
            displace: -self.displace,
            length: self.length,
        }
    }

    pub fn signed_distance(&self, to: Pnt2<T>) -> T {
        to.to_vec().perp_dot(self.displace) + self.displace.perp_dot(self.origin.to_vec())
    }

    pub fn segment_intersect_offset(&self, other: &Line2<T>) -> Option<T> {
        self.intersect_offset(other).and_then(|offset| {
            if offset < T::zero() || offset >= self.length {
                return None;
            }

            let other_offset = other.offset_at(self.at_offset(offset));
            if other_offset < T::zero() || other_offset >= other.length {
                return None;
            }

            Some(offset)
        })
    }

    pub fn offset_at(&self, point: Pnt2<T>) -> T {
        if self.displace.x.abs() > self.displace.y.abs() {
            (point.x - self.origin.x) / self.displace.x
        } else {
            (point.y - self.origin.y) / self.displace.y
        }
    }

    pub fn intersect_offset(&self, other: &Line2<T>) -> Option<T> {
        let denominator = self.displace.perp_dot(other.displace);
        if denominator.abs() < <T as NumCast>::from(1e-16).unwrap() {
            None
        } else {
            Some((other.origin - self.origin).perp_dot(other.displace) / denominator)
        }
    }

    pub fn intersect_point(&self, other: &Line2<T>) -> Option<Pnt2<T>> {
        self.intersect_offset(other)
            .map(|offset| self.at_offset(offset))
    }

    pub fn at_offset(&self, offset: T) -> Pnt2<T> {
        self.origin + self.displace * offset
    }
}
