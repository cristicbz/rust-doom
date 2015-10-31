use std::cell::Cell;
use math::{Mat4, Vec3f};
use num::Zero;

/// A Camera object abstracts a projection-modelview transform, by providing concepts like
/// position and orientation.
pub struct Camera {
    // View parameters.
    position: Vec3f, // Camera location. (equiv to negative translation)
    yaw: f32, // Left-right look (rotation around y-axis).
    pitch: f32, // Up-down look (rotation around x-axis).
    roll: f32, // Tilt left-right (rotation around z-axis).
    modelview: Cell<Mat4>, // Cached modelview transform.
    dirty: Cell<bool>, // Whether the cached modelview is out of date.

    // Projection parameters.
    fov: f32, // Horizontal field of view.
    aspect_ratio: f32, // Viewport aspect ratio (width / height).
    near: f32, // Near plane Z.
    far: f32, // Far plane Z.
    projection: Mat4, // Projection matrix computed from parameters above.
}

impl Camera {
    /// Creates a camera in (0, 0, 0) loking in direction (0, 0, -1), with
    /// specified perspective parameters.
    pub fn new(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Camera {
        Camera {
            position: Vec3f::zero(),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,

            fov: fov,
            aspect_ratio: aspect_ratio,
            near: near,
            far: far, // Whereeeever you are.
            projection: Mat4::new_perspective(fov, aspect_ratio, near, far),

            modelview: Cell::new(Mat4::new_identity()),
            dirty: Cell::new(true),
        }
    }

    /// Returns the modelview matrix associated with this camera.
    pub fn modelview(&self) -> Mat4 {
        if self.dirty.get() {
            self.dirty.set(false);
            self.modelview.set(Mat4::new_euler_rotation(self.yaw, self.pitch, self.roll) *
                               Mat4::new_translation(-self.position));
        }
        self.modelview.get()
    }

    /// Returns the projection matrix associated with this camera.
    pub fn projection(&self) -> &Mat4 {
        &self.projection
    }

    /// Returns the world coordinates of the camera.
    pub fn position(&self) -> &Vec3f {
        &self.position
    }

    /// Returns the yaw (rotation around Y, bottom to top, axis) of the camera.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Returns the pitch (rotation around X, left to right, axis) of the camera.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Returns the roll (rotation around Z, back to front, axis) of the camera.
    pub fn roll(&self) -> f32 {
        self.roll
    }

    /// Changes the yaw of the camera (rotation around Y, bottom to top, axis).
    pub fn set_yaw(&mut self, value: f32) -> &mut Camera {
        self.yaw = value;
        self.dirty.set(true);
        self
    }

    /// Changes the pitch of the camera (rotation around X, left to right, axis).
    pub fn set_pitch(&mut self, value: f32) -> &mut Camera {
        self.pitch = value;
        self.dirty.set(true);
        self
    }

    /// Changes the roll of the camera (rotation around Z, back to front, axis).
    pub fn set_roll(&mut self, value: f32) -> &mut Camera {
        self.roll = value;
        self.dirty.set(true);
        self
    }

    /// Moves the camera with to an absolute position.
    pub fn set_position(&mut self, value: Vec3f) -> &mut Camera {
        self.position = value;
        self.dirty.set(true);
        self
    }

    /// Moves the camera with a relative vector.
    pub fn move_by(&mut self, by: Vec3f) -> &mut Camera {
        self.position = self.position + by;
        self.dirty.set(true);
        self
    }

    /// Changes the parameters of the perspective projection. Passing None for a
    /// parameter leaves it unchanged.
    pub fn update_perspective(&mut self,
                              fov: Option<f32>,
                              aspect_ratio: Option<f32>,
                              near: Option<f32>,
                              far: Option<f32>)
                              -> &mut Camera {
        self.fov = fov.unwrap_or(self.fov);
        self.aspect_ratio = aspect_ratio.unwrap_or(self.aspect_ratio);
        self.near = near.unwrap_or(self.near);
        self.far = far.unwrap_or(self.far);

        self.projection = Mat4::new_perspective(self.fov, self.aspect_ratio, self.near, self.far);
        self
    }
}

#[cfg(test)]
mod test {
    use super::Camera;
    use math::{Mat4, Vec3f};

    #[test]
    fn projection() {
        let mut camera = Camera::new(45.0, 16.0 / 9.0, 0.1, 10.0);
        assert_eq!(&Mat4::new_perspective(45.0, 16.0 / 9.0, 0.1, 10.0),
                   camera.projection());

        camera.update_perspective(Some(90.0), None, None, None);
        assert_eq!(&Mat4::new_perspective(90.0, 16.0 / 9.0, 0.1, 10.0),
                   camera.projection());

        camera.update_perspective(Some(75.0), None, None, Some(100.0));
        assert_eq!(&Mat4::new_perspective(75.0, 16.0 / 9.0, 0.1, 100.0),
                   camera.projection());

        camera.update_perspective(None, Some(1.0), None, None);
        assert_eq!(&Mat4::new_perspective(75.0, 1.0, 0.1, 100.0),
                   camera.projection());

        camera.update_perspective(None, None, Some(1.0), None);
        assert_eq!(&Mat4::new_perspective(75.0, 1.0, 1.0, 100.0),
                   camera.projection());
    }

    #[test]
    fn modelview() {
        let mut camera = Camera::new(45.0, 16.0 / 9.0, 0.1, 10.0);
        camera.set_yaw(0.0);
        camera.set_pitch(0.0);
        camera.set_roll(0.0);
        camera.set_position(Vec3f::new(0.0, 0.0, 0.0));
        assert_eq!(camera.yaw(), 0.0);
        assert_eq!(camera.pitch(), 0.0);
        assert_eq!(camera.roll(), 0.0);
        assert_eq!(camera.position(), &Vec3f::new(0.0, 0.0, 0.0));

        camera.set_yaw(1.0);
        camera.set_pitch(2.0);
        camera.set_roll(3.0);
        camera.set_position(Vec3f::new(4.0, 5.0, 6.0));
        assert_eq!(camera.yaw(), 1.0);
        assert_eq!(camera.pitch(), 2.0);
        assert_eq!(camera.roll(), 3.0);
        assert_eq!(camera.position(), &Vec3f::new(4.0, 5.0, 6.0));

        assert!(camera.modelview().approx_eq(&(Mat4::new_euler_rotation(1.0, 2.0, 3.0) *
                                               Mat4::new_translation(Vec3f::new(-4.0,
                                                                                -5.0,
                                                                                -6.0))),
                                             1e-16));
    }
}
