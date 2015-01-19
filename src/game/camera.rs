use math::{Mat4, Vec3, Vec3f};
use std::cell::UnsafeCell;

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

    cache: UnsafeCell<CachedTransform>,  // Combined projection*view transform.
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

            cache: UnsafeCell::new(
                CachedTransform{ matrix: Mat4::new_identity(), dirty: true }),
        };
        // Ensure cache is not dirty on return
        unsafe { camera.get_transform_ref(); }
        camera
    }

    pub unsafe fn get_transform_ref(&self) -> &Mat4 {
        (*self.cache.get()).refresh(|| {
           self.projection
               //* Mat4::new_euler_rotation(self.yaw, self.pitch, self.roll)
               * Mat4::new_axis_rotation(&Vec3::x_axis(), self.pitch)
               * Mat4::new_axis_rotation(&Vec3::y_axis(), self.yaw)
               * Mat4::new_translation(-self.position)
        })
    }

    pub fn multiply_transform(&self, rhs: &Mat4) -> Mat4 {
        unsafe { *self.get_transform_ref() * *rhs }
    }

    pub fn get_position(&self) -> &Vec3f { &self.position }
    pub fn get_yaw(&self) -> f32 { self.yaw }
    pub fn get_pitch(&self) -> f32 { self.pitch }
    pub fn get_roll(&self) -> f32 { self.roll }

    pub fn set_yaw(&mut self, value: f32) -> &mut Camera {
        self.yaw = value;
        self.make_dirty();
        self
    }

    pub fn set_pitch(&mut self, value: f32) -> &mut Camera {
        self.pitch = value;
        self.make_dirty();
        self
    }

    pub fn set_roll(&mut self, value: f32) -> &mut Camera {
        self.roll = value;
        self.make_dirty();
        self
    }

    pub fn set_position(&mut self, value: Vec3f) -> &mut Camera {
        self.position = value;
        self.make_dirty();
        self
    }

    pub fn move_by(&mut self, by: Vec3f) -> &mut Camera {
        self.position = self.position + by;
        self.make_dirty();
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
        self.make_dirty();
        self
    }

    fn make_dirty(&mut self) {
        // Actually safe because we've got a mutable self.
        unsafe { (*self.cache.get()).dirty = true; }
    }
}

struct CachedTransform {
    matrix: Mat4,
    dirty: bool,
}

impl CachedTransform {
    fn refresh<F: FnOnce() -> Mat4>(&mut self, compute_fresh: F) -> &Mat4 {
        if self.dirty {
            self.matrix = (compute_fresh)();
            self.dirty = false;
        }
        &self.matrix
    }
}


