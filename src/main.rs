mod app;
mod core;
mod frontend;
mod backend;

#[macro_use]
extern crate log;
extern crate log4rs;
extern crate chrono;
extern crate csv;

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

#[macro_use]
extern crate bitflags;
extern crate bit_set;
extern crate cgmath;

extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate piston;

extern crate rand;
extern crate num;
extern crate itertools;

#[macro_use]
extern crate enum_primitive;
extern crate gfx_text;

extern crate rustc_serialize as serialize;

fn main() {
	use log4rs::config::*;
	use log4rs::append::console::*;
	use std::env;
	let args = env::args().collect::<Vec<_>>();
	
	let config = Config::builder()
		.appender(Appender::builder().build("stdout".to_string(),
		                                    Box::new(ConsoleAppender::builder().build())))
		.logger(Logger::builder().build("gfx_device_gl".to_string(), log::LogLevelFilter::Error))
		.logger(Logger::builder().build("rust_oids".to_string(), log::LogLevelFilter::Info))
		.build(Root::builder().appender("stdout".to_string()).build(log::LogLevelFilter::Info));
	log4rs::init_config(config.unwrap()).unwrap();
	app::run(&args);
}
