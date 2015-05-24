use math::{Mat4, Vec3, Vec3f};
use cached::Cached;

/// A Camera object abstracts a projection-view transform.
///
/// Use `with_transform` to obtain the projection-view transform and the `set_*`
/// `update_*` and `move_by` methods to change the camera's location, rotation
/// and intrinsic (projection) properties.
pub struct Camera {
    // View parameters.
    position: Vec3f,  // Camera location. (equiv to negative translation)
    yaw: f32,         // Left-right look (rotation around y-axis).
    pitch: f32,       // Up-down look (rotation around x-axis).
    roll: f32,        // Tilt left-right (rotation around z-axis).

    // Projection parameters.
    fov: f32,           // Horizontal field of view.
    aspect_ratio: f32,  // Viewport aspect ratio (width / height).
    near: f32,          // Near plane Z.
    far: f32,           // Far plane Z.
    projection: Mat4,   // Projection matrix computed from parameters above.

    modelview: Cached<Mat4>,  // Cached modelview transform.
}

impl Camera {
    /// Creates a camera in (0, 0, 0) loking in direction (0, 0, -1), with
    /// specified perspective parameters.
    pub fn new(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Camera {
        let camera = Camera {
            position: Vec3::zero(), yaw: 0.0, pitch: 0.0, roll: 0.0,

            fov: fov, aspect_ratio: aspect_ratio,
            near: near, far: far,  // Whereeeever you are.
            projection: Mat4::new_perspective(fov, aspect_ratio, near, far),

            modelview: Cached::invalidated(Mat4::new_identity()),
        };
        camera
    }

    pub fn modelview(&self) -> &Mat4 {
        self.modelview.get(|modelview| {
            *modelview = Mat4::new_euler_rotation(self.yaw, self.pitch, self.roll)
                       * Mat4::new_translation(-self.position);
        })
    }

    pub fn projection(&self) -> &Mat4 { &self.projection }
    pub fn position(&self) -> &Vec3f { &self.position }
    pub fn yaw(&self) -> f32 { self.yaw }
    pub fn pitch(&self) -> f32 { self.pitch }
    pub fn roll(&self) -> f32 { self.roll }

    pub fn set_yaw(&mut self, value: f32) -> &mut Camera {
        self.yaw = value;
        self.modelview.invalidate();
        self
    }

    pub fn set_pitch(&mut self, value: f32) -> &mut Camera {
        self.pitch = value;
        self.modelview.invalidate();
        self
    }

    pub fn set_roll(&mut self, value: f32) -> &mut Camera {
        self.roll = value;
        self.modelview.invalidate();
        self
    }

    /// Moves the camera with to an absolute position.
    pub fn set_position(&mut self, value: Vec3f) -> &mut Camera {
        self.position = value;
        self.modelview.invalidate();
        self
    }

    /// Moves the camera with a relative vector.
    pub fn move_by(&mut self, by: Vec3f) -> &mut Camera {
        self.position = self.position + by;
        self.modelview.invalidate();
        self
    }

    /// Changes the parameters of the perspective projection. Passing None for a
    /// parameter leaves it unchanged.
    pub fn update_perspective(&mut self,
                              fov: Option<f32>,
                              aspect_ratio: Option<f32>,
                              near: Option<f32>,
                              far: Option<f32>) -> &mut Camera {
        self.fov = fov.unwrap_or(self.fov);
        self.aspect_ratio = aspect_ratio.unwrap_or(self.aspect_ratio);
        self.near = near.unwrap_or(self.near);
        self.far = far.unwrap_or(self.far);

        self.projection = Mat4::new_perspective(
            self.fov, self.aspect_ratio, self.near, self.far);
        self
    }
}
