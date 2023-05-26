use glam::{Vec3, Vec2};

pub fn look_dir(pitch_yaw_radians: Vec2) -> Vec3 {
	Vec3::new(
		-pitch_yaw_radians.y.sin() * pitch_yaw_radians.x.cos(),
		pitch_yaw_radians.x.sin(),
		pitch_yaw_radians.y.cos() * pitch_yaw_radians.x.cos()
	)
}

pub fn flat_forward_vec(yaw_radians: f32) -> Vec3 {
	Vec3::new(
		-yaw_radians.sin(),
		0.0,
		yaw_radians.cos()
	)
}

pub fn flat_right_vec(yaw_radians: f32) -> Vec3 {
	Vec3::new(
		yaw_radians.cos(),
		0.0,
		yaw_radians.sin()
	)
}