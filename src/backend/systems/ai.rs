use super::*;
use std::f32::consts;
use std::collections::HashMap;
use backend::obj;
use backend::obj::Identified;
use backend::obj::Transformable;
use backend::world;
use backend::world::agent;
use backend::world::agent::Personality;
use backend::world::agent::TypedAgent;
use backend::world::segment;
use backend::world::segment::Intent;
use cgmath::*;
use core::geometry::Position;
use itertools::Itertools;

type IdPositionMap = HashMap<obj::Id, Position>;

pub struct AiSystem {
	beacons: Box<[Position]>,
	targets: IdPositionMap,
}

impl Updateable for AiSystem {}

impl System for AiSystem {
	fn from_world(&mut self, world: &world::World) {
		self.beacons = world.emitters().iter().map(|e| e.transform().position).collect::<Vec<_>>().into_boxed_slice();
		self.targets = world.agents(agent::AgentType::Resource)
			.iter()
			.filter(|&(_, ref v)| v.state.is_active())
			.map(|(_, v)| (v.id(), v.transform().position))
			.collect::<HashMap<_, _>>();
	}

	fn to_world(&self, world: &mut world::World) {
		Self::update_minions(&self.targets,
		                     &self.beacons,
		                     &mut world.agents_mut(agent::AgentType::Minion));
	}
}

impl Default for AiSystem {
	fn default() -> Self {
		AiSystem {
			beacons: Box::new([]),
			targets: HashMap::new(),
		}
	}
}

impl AiSystem {
	fn update_minions(targets: &IdPositionMap, beacons: &[Position], minions: &mut agent::AgentMap) {

		fn nearest_beacon<'a>(beacons: &'a [Position], p: &'a Position) -> &'a Position {
			beacons.iter()
				.fold1(|n, b| if (p - n).length2() < (p - b).length2() { n } else { b })
				.unwrap_or(p)
		}

		for (_, agent) in minions.iter_mut() {
			let brain = agent.brain().clone();
			let core = agent.first_segment(segment::CORE);
			let head = agent.first_segment(segment::SENSOR);
			if let Some(sensor) = head {
				let p0 = sensor.transform.position;
				let radar_range = sensor.mesh.shape.radius() * 10.;
				let current_target = agent.state.target().clone();
				let current_target_position = agent.state.target_position().clone();
				// if our original target is dead then we need to find another one
				let new_target: Option<(obj::Id, Position)> = match current_target {
					None => {
						targets.iter()
							.find(|&(_, &p)| (p - p0).length() < radar_range)
							.map(|(&id, &position)| (id, position))
					}
					Some(id) => targets.get(&id).map(|&position| (id, position)),
				};
				// and failing that again, we target
				match new_target {
					None => agent.state.retarget(None, *nearest_beacon(beacons, &current_target_position)),
					Some((id, position)) => agent.state.retarget(Some(id), position),
				};
				// find where our target is in the world
				let target_position = agent.state.target_position().clone();
				// and transform the world position into the head's frame
				let t0 = target_position - sensor.transform.position;
				let t = t0.normalize_to(t0.length().min(radar_range));
				// direction in which the head is pointing, normalized
				let s = Matrix2::from_angle(rad(sensor.transform.angle)) * (-Position::unit_y());
				// some proprioception, feeding back the angle betweent the neck and the first torso
				let neck_angle = consts::PI + sensor.transform.angle -
				                 core.map(|t| t.transform.angle).unwrap_or(sensor.transform.angle);
				// we pass the relative position of the target decomposed in our frame of reference to the neural network
				// expecting four components we can use as thresholds
				let r = agent.brain().response(&[neck_angle, t.dot(s), t.perp_dot(s), 0.]);
				const POWER_BOOST: f32 = 100.;

				let segments = &mut agent.segments_mut();
				for segment in segments.iter_mut() {
					let flags = &segment.flags;
					if flags.contains(segment::ACTUATOR) {
						let power = segment.state.get_charge() * segment.mesh.shape.radius().powi(2) * POWER_BOOST;
						let f = Matrix2::from_angle(rad(segment.transform.angle)) * Position::unit_y() * power;
						let intent =
							if let Some(refs) = segment.state.last_touched {
								match refs.id().type_of() {
									agent::AgentType::Resource => Intent::Idle,
									_ => {
										let fear: f32 = brain.fear();
										Intent::RunAway(f * fear)
									}
								}
							} else if flags.contains(segment::RUDDER | segment::LEFT) && r[0] > brain.hunger() {
								Intent::Move(-f)
							} else if flags.contains(segment::RUDDER | segment::RIGHT) && r[1] > brain.hunger() {
								Intent::Move(-f)
							} else if flags.contains(segment::THRUSTER) && r[2] > brain.haste() {
								Intent::Move(f)
							} else if flags.contains(segment::BRAKE) && r[3] > brain.prudence() {
								Intent::Brake(-f)
							} else {
								Intent::Idle
							};
						match intent {
							Intent::Idle => segment.state.set_target_charge(brain.rest()),
							Intent::Move(_) => segment.state.set_target_charge(brain.thrust()),
							Intent::Brake(_) => segment.state.set_target_charge(brain.thrust()),
							Intent::RunAway(_) => segment.state.set_charge(brain.thrust()),
						}
						segment.state.intent = intent;
					}
				}
			}
		}
	}
}
