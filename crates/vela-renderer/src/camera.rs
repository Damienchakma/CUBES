use glam::{Mat4, Vec3, camera::rh};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraProjection {
    Perspective { vertical_fov_degrees: f32 },
    Orthographic { vertical_size: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub projection: CameraProjection,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(4.5, 3.5, 6.0),
            target: Vec3::ZERO,
            projection: CameraProjection::Perspective {
                vertical_fov_degrees: 60.0,
            },
            near: 0.05,
            far: 500.0,
        }
    }
}

impl Camera {
    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        let view = rh::view::look_at_mat4(self.position, self.target, Vec3::Y);
        let projection = match self.projection {
            CameraProjection::Perspective {
                vertical_fov_degrees,
            } => rh::proj::directx::perspective(
                vertical_fov_degrees.to_radians(),
                aspect_ratio.max(0.001),
                self.near,
                self.far,
            ),
            CameraProjection::Orthographic { vertical_size } => {
                let half_height = vertical_size * 0.5;
                let half_width = half_height * aspect_ratio.max(0.001);
                rh::proj::directx::orthographic(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    self.near,
                    self.far,
                )
            }
        };
        projection * view
    }
}
