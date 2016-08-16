use backend::obj;
use backend::obj::*;
use rand;
use rand::Rng;
use std::collections::HashMap;
use std::f32::consts;
use cgmath;
use cgmath::EuclideanVector;
use num;
use core::color;
use core::color::ToRgb;
use core::geometry::*;

#[derive(Clone)]
pub struct State {
	pub age_seconds: f32,
	pub age_frames: usize,
	pub charge: f32,
	pub target_charge: f32,
	pub tau: f32,
	pub intent: Option<Position>,
}

impl Default for State {
	fn default() -> Self {
		State {
			age_seconds: 0.,
			age_frames: 0,
			charge: 1.,
			target_charge: 0.,
			tau: 2.0,
			intent: None,
		}
	}
}

impl State {
	pub fn update(&mut self, dt: f32) {
		self.age_seconds += dt;
		self.age_frames += 1;
		let alpha = 1. - f32::exp(-dt / self.tau);
		self.charge = self.target_charge * alpha + self.charge * (1. - alpha);
	}

	pub fn with_charge(initial: f32, target: f32) -> Self {
		State {
			charge: initial,
			target_charge: target,
			..Self::default()
		}
	}

	pub fn charge(&self) -> f32 {
		self.charge
	}
}

#[derive(Copy, Clone)]
pub struct Attachment {
	pub index: SegmentIndex,
	pub attachment_point: AttachmentIndex,
}

bitflags! {
	pub flags SegmentFlags: u32 {
		const SENSOR       = 0x1,
		const JOINT        = 0x4,
		const HEAD		   = 0x10,
		const LEG          = 0x20,
		const ARM          = 0x40,
		const TORSO        = 0x100,
		const BELLY        = 0x200,
		const TAIL         = 0x400,
		const LEFT         = 0x1000,
		const RIGHT        = 0x2000,
		const MIDDLE       = 0x4000,
		const THRUSTER     = 0x10000,
		const RUDDER       = 0x20000,
		const BRAKE        = 0x40000,
		const ACTUATOR     = THRUSTER.bits | RUDDER.bits | BRAKE.bits,
	}
}

#[derive(Clone)]
pub struct Segment {
	pub transform: Transform,
	pub index: SegmentIndex,
	pub mesh: Mesh,
	pub material: Material,
	pub livery: Livery,
	pub attached_to: Option<Attachment>,
	pub state: State,
	pub flags: SegmentFlags,
}

impl Segment {
	pub fn new_attachment(&self, attachment_point: AttachmentIndex) -> Option<Attachment> {
		let max = self.mesh.vertices.len() as AttachmentIndex;
		Some(Attachment {
			index: self.index,
			attachment_point: if attachment_point < max {
				attachment_point
			} else {
				max - 1
			},
		})
	}
}

pub struct Agent {
	id: Id,
	segments: Box<[Segment]>,
}

impl Identified for Agent {
	fn id(&self) -> Id {
		self.id
	}
}

impl Transformable for Agent {
	fn transform(&self) -> Transform {
		self.segments.first().unwrap().transform()
	}
	fn transform_to(&mut self, t: Transform) {
		self.segments.first_mut().unwrap().transform_to(t);
	}
}

impl Transformable for Segment {
	fn transform(&self) -> Transform {
		self.transform
	}
	fn transform_to(&mut self, t: Transform) {
		self.transform = t;
	}
}

impl obj::Geometry for Segment {
	fn mesh(&self) -> &Mesh {
		&self.mesh
	}
}

impl obj::Solid for Segment {
	fn material(&self) -> Material {
		self.material
	}
	fn livery(&self) -> Livery {
		self.livery
	}
}

impl obj::Drawable for Segment {
	fn color(&self) -> Rgba {
		let rgba = self.livery.albedo;
		let c = 10. * ((self.state.charge * 0.99) + 0.01);
		[rgba[0] * c, rgba[1] * c, rgba[2] * c, rgba[3] * self.material.density]
	}
}

impl Agent {
	pub fn id(&self) -> Id {
		self.id
	}

	pub fn segments(&self) -> &[Segment] {
		&self.segments
	}

	pub fn segments_mut(&mut self) -> &mut [Segment] {
		&mut self.segments
	}

	pub fn segment(&self, index: SegmentIndex) -> Option<&Segment> {
		self.segments.get(index as usize)
	}

	pub fn segment_mut(&mut self, index: SegmentIndex) -> Option<&mut Segment> {
		self.segments.get_mut(index as usize)
	}
}

struct Randomizer {
	rng: rand::ThreadRng,
}

impl Randomizer {
	fn new() -> Self {
		Randomizer { rng: rand::thread_rng() }
	}

	fn frand<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Float {
		self.rng.gen::<T>() * (max - min) + min
	}

	fn irand<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + Copy {
		self.rng.gen::<T>() % (max - min + T::one()) + min
	}

	fn random_ball(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		Shape::new_ball(radius)
	}

	fn random_box(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio: f32 = self.frand(1.0, 2.0);
		Shape::new_box(radius, ratio)
	}

	fn random_vbar(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio: f32 = self.frand(0.1, 0.2);
		Shape::new_box(radius, ratio)
	}


	fn random_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = self.frand(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 1.5 - self.frand(0., consts::PI);
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_iso_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = self.frand(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_eq_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = consts::PI * 2. / 3.;
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_star(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let n = self.irand(3, 8);
		let ratio1 = self.frand(0.5, 1.0);
		let ratio2 = self.frand(0.7, 0.9) * (1. / ratio1);
		Shape::new_star(n, radius, ratio1, ratio2)
	}

	fn random_poly(&mut self, upside_down: bool) -> Shape {
		let n = self.irand(3, 8);
		self.random_npoly(n, upside_down)
	}

	fn random_npoly(&mut self, n: AttachmentIndex, upside_down: bool) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio1 = f32::cos(consts::PI / n as f32);
		let ratio2 = 1. / ratio1;
		if upside_down {
			Shape::new_star(n, radius * ratio1, ratio2, ratio1)
		} else {
			Shape::new_star(n, radius, ratio1, ratio2)
		}
	}
}

struct AgentBuilder {
	id: Id,
	material: Material,
	livery: Livery,
	state: State,
	segments: Vec<Segment>,
}

impl AgentBuilder {
	fn new(id: Id, material: Material, livery: Livery, state: State) -> Self {
		AgentBuilder {
			id: id,
			material: material,
			livery: livery,
			state: state,
			segments: Vec::new(),
		}
	}

	pub fn start(&mut self, position: Position, angle: f32, shape: &Shape) -> &mut Self {
		let segment = self.new_segment(shape, Winding::CW, position, angle, None, TORSO | MIDDLE);
		self.segments.clear();
		self.segments.push(segment);
		self
	}

	fn id(&mut self, id: Id) -> &mut Self {
		self.id = id;
		self
	}

	#[inline]
	pub fn add(&mut self,
	           parent_index: SegmentIndex,
	           attachment_index_offset: isize,
	           shape: &Shape,
	           flags: SegmentFlags)
	           -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CW,
		          flags | MIDDLE)
	}
	#[inline]
	pub fn addl(&mut self,
	            parent_index: SegmentIndex,
	            attachment_index_offset: isize,
	            shape: &Shape,
	            flags: SegmentFlags)
	            -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CCW,
		          flags | LEFT)
	}
	#[inline]
	pub fn addr(&mut self,
	            parent_index: SegmentIndex,
	            attachment_index_offset: isize,
	            shape: &Shape,
	            flags: SegmentFlags)
	            -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CW,
		          flags | RIGHT)
	}

	pub fn addw(&mut self,
	            parent_index: SegmentIndex,
	            attachment_index_offset: isize,
	            shape: &Shape,
	            winding: Winding,
	            flags: SegmentFlags)
	            -> &mut Self {
		let parent = self.segments[parent_index as usize].clone();//urgh!;
		let parent_pos = parent.transform.position;
		let parent_angle = parent.transform.angle;
		let parent_length = parent.mesh.shape.length() as isize;
		let attachment_index = ((attachment_index_offset + parent_length) % parent_length) as usize;
		let p0 = cgmath::Matrix2::from_angle(cgmath::rad(parent_angle)) * parent.mesh.vertices[attachment_index];
		let angle = f32::atan2(p0.y, p0.x);
		let r0 = p0.length() * parent.mesh.shape.radius();
		let r1 = shape.radius();
		let segment = self.new_segment(shape,
		                               winding,
		                               parent_pos + (p0 * (r0 + r1)),
		                               consts::PI / 2. + angle,
		                               parent.new_attachment(attachment_index as AttachmentIndex),
		                               flags);
		self.segments.push(segment);
		self
	}

	pub fn index(&self) -> SegmentIndex {
		match self.segments.len() {
			0 => 0,
			n => (n - 1) as SegmentIndex,
		}
	}

	fn new_segment(&mut self,
	               shape: &Shape,
	               winding: Winding,
	               position: Position,
	               angle: f32,
	               attachment: Option<Attachment>,
	               flags: SegmentFlags)
	               -> Segment {
		Segment {
			index: self.segments.len() as SegmentIndex,
			transform: Transform::new(position, angle),
			mesh: Mesh::from_shape(shape.clone(), winding),
			material: self.material.clone(),
			livery: self.livery.clone(),
			state: self.state.clone(),
			attached_to: attachment,
			flags: flags,
		}
	}

	pub fn build(&self) -> Agent {
		Agent {
			id: self.id,
			segments: self.segments.clone().into_boxed_slice(),
		}
	}
}

pub struct Flock {
	seq: Id,
	rnd: Randomizer,
	agents: HashMap<Id, Agent>,
}

impl Flock {
	pub fn new() -> Flock {
		Flock {
			seq: 0,
			rnd: Randomizer::new(),
			agents: HashMap::new(),
		}
	}

	pub fn get(&self, id: Id) -> Option<&Agent> {
		self.agents.get(&id)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Agent> {
		self.agents.get_mut(&id)
	}

	pub fn next_id(&mut self) -> Id {
		self.seq = self.seq + 1;
		self.seq
	}

	pub fn new_resource(&mut self, initial_pos: Position, charge: f32) -> Id {
		let albedo = color::YPbPr::new(0.5, self.rnd.frand(-0.5, 0.5), self.rnd.frand(-0.5, 0.5));
		let ball = self.rnd.random_ball();
		let mut builder = AgentBuilder::new(self.next_id(),
		                                    Material { density: 1.0, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    State::with_charge(charge, 0.));
		self.insert(builder.start(initial_pos, 0., &ball).build())
	}

	pub fn new_minion(&mut self, initial_pos: Position, charge: f32) -> Id {
		let albedo = color::Hsl::new(self.rnd.frand(0., 1.), 0.5, 0.5);
		let mut builder = AgentBuilder::new(self.next_id(),
		                                    Material { density: 0.2, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    State::with_charge(0., charge));
		let arm_shape = self.rnd.random_star();
		let leg_shape = self.rnd.random_star();
		let torso_shape = self.rnd.random_npoly(5, true);
		let head_shape = self.rnd.random_iso_triangle();
		let tail_shape = self.rnd.random_vbar();
		let initial_angle = consts::PI / 2. + f32::atan2(initial_pos.y, initial_pos.x);

		let torso = builder.start(initial_pos, initial_angle, &torso_shape)
		                   .index();
		builder.addr(torso, 2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER)
		       .addl(torso, -2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER);

		let head = builder.add(torso, 0, &head_shape, HEAD | SENSOR).index();
		builder.addr(head, 1, &head_shape, HEAD | ACTUATOR | RUDDER)
		       .addl(head, 2, &head_shape, HEAD | ACTUATOR | RUDDER);

		let mut belly = torso;
		let mut belly_mid = torso_shape.mid();
		for _ in 0..self.rnd.irand(0, 4) {
			let belly_shape = self.rnd.random_poly(true);

			belly = builder.add(belly, belly_mid, &belly_shape, BELLY | JOINT).index();
			belly_mid = belly_shape.mid();
			if self.rnd.irand(0, 4) == 0 {
				builder.addr(belly, 2, &arm_shape, ARM | ACTUATOR | RUDDER)
				       .addl(belly, -2, &arm_shape, ARM | ACTUATOR | RUDDER);
			}
		}

		builder.addr(belly, belly_mid - 1, &leg_shape, LEG | ACTUATOR | THRUSTER)
		       .addl(belly,
		             -(belly_mid - 1),
		             &leg_shape,
		             LEG | ACTUATOR | THRUSTER)
		       .add(belly, belly_mid, &tail_shape, TAIL | ACTUATOR | BRAKE);

		self.insert(builder.build())
	}

	fn insert(&mut self, agent: Agent) -> Id {
		let id = agent.id;
		self.agents.insert(id, agent);
		id
	}

	pub fn kill(&mut self, id: &Id) {
		self.agents.remove(id);
	}

	pub fn agents(&self) -> &HashMap<Id, Agent> {
		&self.agents
	}

	pub fn agents_mut(&mut self) -> &mut HashMap<Id, Agent> {
		&mut self.agents
	}
}

#[repr(packed)]
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub struct AgentRefs {
	pub agent_id: obj::Id,
	pub segment_index: obj::SegmentIndex,
	pub bone_index: obj::BoneIndex,
}

impl Default for AgentRefs {
	fn default() -> AgentRefs {
		AgentRefs {
			agent_id: 0xdeadbeef,
			segment_index: 0xff,
			bone_index: 0xff,
		}
	}
}

impl AgentRefs {
	pub fn with_id(id: obj::Id) -> AgentRefs {
		AgentRefs { agent_id: id, ..Default::default() }
	}

	pub fn with_segment(id: obj::Id, segment_index: obj::SegmentIndex) -> AgentRefs {
		AgentRefs {
			agent_id: id,
			segment_index: segment_index,
			..Default::default()
		}
	}

	pub fn with_bone(id: obj::Id, segment_index: obj::SegmentIndex, bone_index: obj::BoneIndex) -> AgentRefs {
		AgentRefs {
			agent_id: id,
			segment_index: segment_index,
			bone_index: bone_index,
		}
	}
}

pub struct World {
	pub extent: Rect,
	pub fence: obj::Mesh,
	pub players: Flock,
	pub minions: Flock,
	pub friendly_fire: Flock,
	pub enemies: Flock,
	pub enemy_fire: Flock,
	pub resources: Flock,
	pub props: Flock,
}

pub trait WorldState {
	fn minion(&self, id: obj::Id) -> Option<&Agent>;
}

impl WorldState for World {
	fn minion(&self, id: obj::Id) -> Option<&Agent> {
		self.minions.get(id)
	}
}

impl World {
	pub fn new() -> Self {
		World {
			extent: Rect {
				min: Position::new(-550., -550.),
				max: Position::new(550., 550.),
			},
			fence: Mesh::from_shape(Shape::new_ball(500.), Winding::CW),
			players: Flock::new(),
			minions: Flock::new(),
			friendly_fire: Flock::new(),
			enemies: Flock::new(),
			enemy_fire: Flock::new(),
			resources: Flock::new(),
			props: Flock::new(),
		}
	}

	pub fn new_resource(&mut self, pos: Position) -> obj::Id {
		self.minions.new_resource(pos, 0.3)
	}

	pub fn new_minion(&mut self, pos: Position) -> obj::Id {
		self.minions.new_minion(pos, 0.3)
	}

	pub fn friend_mut(&mut self, id: obj::Id) -> Option<&mut Agent> {
		self.minions.get_mut(id)
	}
}
