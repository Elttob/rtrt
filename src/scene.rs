use anyhow::Result;
use glam::{Mat4, Vec3, Vec2};
use obj::Obj;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GeoVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3]
}

pub struct Camera {
    pub position: Vec3,
    pub pitch_yaw_radians: Vec2,
    pub fov_radians: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub aspect_ratio: f32
}

impl Camera {
    pub fn to_projection_matrix(
        &self,
    ) -> Mat4 {
        Mat4::from_scale(glam::vec3(1.0, -1.0, 1.0)) * Mat4::perspective_lh(self.fov_radians, self.aspect_ratio, self.z_near, self.z_far)
    }

    pub fn to_view_matrix(
        &self
    ) -> Mat4 {
        let rotation = Mat4::from_rotation_x(self.pitch_yaw_radians.x) * Mat4::from_rotation_y(self.pitch_yaw_radians.y);
        let translation = Mat4::from_translation(-self.position);
        rotation * translation
    }
}

pub struct Scene {
    pub vertices: Vec<GeoVertex>
}

impl Scene {
    pub fn from_obj_righthanded(
        obj: Obj<obj::Vertex, u16>
    ) -> Result<Self> {
        let vertices = obj.indices.iter()
            .map(|index| *obj.vertices.get(*index as usize).unwrap())
            .map(|vertex| GeoVertex {
                position: [vertex.position[0], vertex.position[1], -vertex.position[2]],
                normal: [vertex.normal[0], vertex.normal[1], -vertex.normal[2]]
            })
            .collect();

        Ok(Self {
            vertices
        })
    }
}