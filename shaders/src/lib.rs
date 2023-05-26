#![cfg_attr(target_arch = "spirv", no_std)]

use spirv_std::glam::{Vec4, Vec2, vec2, vec3, Vec3, Mat4};
use spirv_std::spirv;

#[spirv(fragment)]
pub fn main_fs(
    in_colour: Vec3,
    output: &mut Vec4
) {
    *output = in_colour.extend(1.00);
}

const POSITIONS: [Vec2; 3] = [
    vec2(0.0, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
];

const COLOURS: [Vec3; 3] = [
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
];

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CameraUniforms {
    pub proj: Mat4,
    pub view: Mat4
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(push_constant)] in_camera_uniforms: &CameraUniforms,
    #[spirv(vertex_index)] vertex_index: i32,
    #[spirv(position, invariant)] out_position: &mut Vec4,
    out_colour: &mut Vec3
) {
    *out_position = in_camera_uniforms.proj * in_camera_uniforms.view * POSITIONS[vertex_index as usize].extend(0.0).extend(1.0);
    *out_colour = COLOURS[vertex_index as usize];
}