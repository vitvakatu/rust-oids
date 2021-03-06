use cgmath;
use cgmath::Vector2;
use cgmath::ApproxEq;
use core::util::Initial;

pub type Position = Vector2<f32>;
pub type Translation = Vector2<f32>;
pub type Velocity = Vector2<f32>;
pub type Angle = f32;
pub type Rotation = f32;
pub type Spin = f32;

pub type M44 = cgmath::Matrix4<f32>;

#[derive(Clone, Default)]
pub struct Size {
	pub width: f32,
	pub height: f32,
}

#[derive(Clone)]
pub struct Transform {
	pub position: Position,
	pub angle: Angle,
}

#[derive(Clone)]
pub struct Motion {
	pub velocity: Velocity,
	pub spin: Spin,
}

#[derive(Copy, Clone)]
pub struct Rect {
	pub min: Position,
	pub max: Position,
}

impl Rect {
	pub fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
		Rect {
			min: Position::new(left, bottom),
			max: Position::new(right, top),
		}
	}

	pub fn bottom_left(&self) -> Position {
		self.min
	}
	pub fn top_right(&self) -> Position {
		self.max
	}

	pub fn bottom_right(&self) -> Position {
		Position::new(self.max.x, self.min.y)
	}

	pub fn top_left(&self) -> Position {
		Position::new(self.min.x, self.max.y)
	}
}

impl Initial for Position {
	fn initial() -> Self {
		Position::new(0., 0.)
	}
}

impl Default for Transform {
	fn default() -> Transform {
		Transform {
			position: Position::new(0., 0.),
			angle: 0.,
		}
	}
}
impl Transform {
	pub fn new(position: Position, angle: f32) -> Self {
		Transform {
			position: position,
			angle: angle,
			..Transform::default()
		}
	}
	pub fn from_position(position: Position) -> Self {
		Transform { position: position, ..Transform::default() }
	}
}

pub fn origin() -> Position {
	Position::new(0., 0.)
}

#[derive(Clone, PartialEq)]
enum VertexType {
	Plus,
	Minus,
	Flat,
}

pub struct PolygonType {
	count: [usize; 3],
}

impl PolygonType {
	fn classify_vertex(v0: &Position, v1: &Position, v2: &Position) -> VertexType {
		let x = (v1 - v0).perp_dot(v2 - v0);
		if x.approx_eq(&0.) {
			VertexType::Flat
		} else if x > 0. {
			VertexType::Plus
		} else {
			VertexType::Minus
		}
	}

	pub fn classify(v: &[Position]) -> Self {
		let mut count = [0usize; 3];

		let n = v.len();
		for i in 0..n {
			let vertex_type = Self::classify_vertex(&v[(i + n - 1) % n], &v[i], &v[(i + 1) % n]);
			count[vertex_type as usize] += 1;
		}

		PolygonType { count: count }
	}

	pub fn is_convex(&self) -> bool {
		self.count[VertexType::Plus as usize] == 0 || self.count[VertexType::Minus as usize] == 0
	}

	#[allow(dead_code)]
	pub fn is_concave(&self) -> bool {
		self.count[VertexType::Plus as usize] > 0 && self.count[VertexType::Minus as usize] > 0
	}

	#[allow(dead_code)]
	pub fn has_flat_vertices(&self) -> bool {
		self.count[VertexType::Flat as usize] > 0
	}
}
