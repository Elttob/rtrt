use std::f32::consts::TAU;

use glam::{Vec2, Vec3, vec3};
use winit::event::{ElementState, VirtualKeyCode, DeviceEvent, MouseButton, WindowEvent};

#[derive(Clone, Copy)]
pub struct InputSnapshot {
	pub move_axes: Vec3,
	pub pitch_yaw_radians: Vec2,
	pub left_mouse: bool,
	pub right_mouse: bool
}

pub struct Input {
	move_front: bool,
	move_back: bool,
	move_left: bool,
	move_right: bool,
	move_up: bool,
	move_down: bool,
	pitch_yaw_radians: Vec2,
	left_mouse: bool,
	right_mouse: bool
}

impl Input {
	pub const MOUSE_DPI: f32 = 8000.0;
	pub const LOOK_SPEED: f32 = 5.0 / Self::MOUSE_DPI;

	pub fn process_device_events(
		&mut self,
		event: &DeviceEvent
	)  {
		match event {
			DeviceEvent::MouseMotion { delta } => {
				self.pitch_yaw_radians = Vec2::new(
					(self.pitch_yaw_radians.x - delta.1 as f32 * Self::LOOK_SPEED).clamp(-TAU / 4.0, TAU / 4.0),
					(self.pitch_yaw_radians.y - delta.0 as f32 * Self::LOOK_SPEED).rem_euclid(TAU)
				);
			},

			DeviceEvent::Key(input) => {
				if let Some(keycode) = input.virtual_keycode {
					let is_pressed = input.state == ElementState::Pressed;
					match keycode {
						VirtualKeyCode::W | VirtualKeyCode::Up => {
							self.move_front = is_pressed;
						},
						VirtualKeyCode::A | VirtualKeyCode::Left => {
							self.move_left = is_pressed;
						},
						VirtualKeyCode::S | VirtualKeyCode::Down => {
							self.move_back = is_pressed;
						},
						VirtualKeyCode::D | VirtualKeyCode::Right => {
							self.move_right = is_pressed;
						},
						VirtualKeyCode::E => {
							self.move_up = is_pressed;
						},
						VirtualKeyCode::Q => {
							self.move_down = is_pressed;
						},
						_ => {}
					}
				}
			}

			_ => {}
		}
	}

	pub fn process_window_events(
		&mut self,
		event: &WindowEvent
	) {
		match event {
			WindowEvent::MouseInput { state, button, .. } => {
				if *button == MouseButton::Left {
					self.left_mouse = *state == ElementState::Pressed;
				} else if *button == MouseButton::Right {
					self.right_mouse = *state == ElementState::Pressed;
				}
			},

			_ => {}
		}
	}

	pub fn new(
		pitch_yaw_radians: Vec2
	) -> Self {
		Self {
			move_front: false,
			move_back: false,
			move_left: false,
			move_right: false,
			pitch_yaw_radians,
			move_up: false,
			move_down: false,
			left_mouse: false,
			right_mouse: false
		}
	}

	pub fn _snapshot(&self) -> InputSnapshot {
		InputSnapshot {
			move_axes: vec3(
				if self.move_right { 1.0 } else { 0.0 } - if self.move_left { 1.0 } else { 0.0 },
				if self.move_up { 1.0 } else { 0.0 } - if self.move_down { 1.0 } else { 0.0 },
				if self.move_front { 1.0 } else { 0.0 } - if self.move_back { 1.0 } else { 0.0 }
			),
			pitch_yaw_radians: self.pitch_yaw_radians,
			left_mouse: self.left_mouse,
			right_mouse: self.right_mouse
		}
	}
}