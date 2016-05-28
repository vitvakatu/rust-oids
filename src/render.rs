use gfx;
use gfx::traits::FactoryExt;

extern crate cgmath;

pub static VERTEX_SRC: &'static [u8] = b"
    #version 150 core

    layout (std140) uniform cb_VertexArgs {
        uniform mat4 u_Proj;
        uniform mat4 u_View;
        uniform mat4 u_Model;
    };

    in vec3 a_Pos;
    in vec3 a_Normal;
    in vec2 a_TexCoord;

    out VertexData {
        vec4 Position;
        vec3 Normal;
        vec2 TexCoord;
    } v_Out;

    void main() {
        v_Out.Position = u_Model * vec4(a_Pos, 1.0);
        v_Out.Normal = mat3(u_Model) * a_Normal;
        v_Out.TexCoord = a_TexCoord;
        gl_Position = u_Proj * u_View * v_Out.Position;
    }
";

pub static FRAGMENT_SRC: &'static [u8] = b"
    #version 150 core
    #define MAX_NUM_TOTAL_LIGHTS 512

    layout (std140) uniform cb_FragmentArgs {
        int u_LightCount;
    };

    struct Light {
        vec4 propagation;
        vec4 center;
        vec4 color;
    };

    layout (std140) uniform u_Lights {
        Light light[MAX_NUM_TOTAL_LIGHTS];
    };

    in VertexData {
        vec4 Position;
        vec3 Normal;
        vec2 TexCoord;
    } v_In;

    out vec4 o_Color;

    void main() {
        vec4 kd = vec4(1.0, 1.0, 1.0, 1.0);
        vec4 color = vec4(0.0, 0.0, 0.0, 0.0);
        for (int i = 0; i < u_LightCount; i++) {
            vec4 delta = light[i].center - v_In.Position;
            float dist = length(delta);
            float inv_dist = 1. / dist;
            vec4 light_to_point_normal = delta * inv_dist;
            float intensity = dot(light[i].propagation.xyz, vec3(1., inv_dist, inv_dist * inv_dist));
            color += kd * light[i].color * intensity * max(0, dot(light_to_point_normal, vec4(v_In.Normal, 0.)));
        }
        o_Color = color;
    }
";

/// Placeholder Color format
pub type ColorFormat = gfx::format::Rgba8;
/// Placeholder Depth Format
pub type DepthFormat = gfx::format::DepthStencil;


// placeholder
gfx_vertex_struct!(VertexPosNormal {
	pos: [f32; 3] = "a_Pos",
	normal: [f32; 3] = "a_Normal",
	tex_coord: [f32; 2] = "a_TexCoord",
});

pub type GFormat = [f32; 4];

pub type M44 = cgmath::Matrix4<f32>;

pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

gfx_defines!(
    constant PointLight {
        propagation: [f32; 4] = "propagation",
        center: [f32; 4] = "center",
        color: [f32; 4] = "color",
    }

    constant VertexArgs {
        proj: [[f32; 4]; 4] = "u_Proj",
        view: [[f32; 4]; 4] = "u_View",
        model: [[f32; 4]; 4] = "u_Model",
    }

    constant FragmentArgs {
        light_count: i32 = "u_LightCount",
    }

    pipeline shaded {
        vbuf: gfx::VertexBuffer<VertexPosNormal> = (),
        vertex_args: gfx::ConstantBuffer<VertexArgs> = "cb_VertexArgs",
        fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
        lights: gfx::ConstantBuffer<PointLight> = "u_Lights",
        out_ka: gfx::RenderTarget<gfx::format::Rgba8> = "o_Color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
);

pub struct DrawShaded<R: gfx::Resources> {
	vertex: gfx::handle::Buffer<R, VertexArgs>,
	fragment: gfx::handle::Buffer<R, FragmentArgs>,
	lights: gfx::handle::Buffer<R, PointLight>,
	pso: gfx::pso::PipelineState<R, shaded::Meta>,
}

pub struct Camera {
	pub projection: M44,
	pub view: M44,
}

impl<R: gfx::Resources> DrawShaded<R> {
	pub fn new<F>(factory: &mut F) -> DrawShaded<R>
		where R: gfx::Resources,
		      F: gfx::Factory<R> {
		let lights = factory.create_constant_buffer(512);
		let vertex = factory.create_constant_buffer(1);
		let fragment = factory.create_constant_buffer(1);
		let pso = factory.create_pipeline_simple(VERTEX_SRC, FRAGMENT_SRC, shaded::new())
			.unwrap();

		DrawShaded {
			vertex: vertex,
			fragment: fragment,
			lights: lights,
			pso: pso,
		}
	}

	pub fn begin_frame<C: gfx::CommandBuffer<R>>(&self,
	                                             encoder: &mut gfx::Encoder<R, C>,
	                                             target: &gfx::handle::RenderTargetView<R, ColorFormat>,
	                                             depth: &gfx::handle::DepthStencilView<R, DepthFormat>) {
		// clear
		encoder.clear(&target, BLACK);
		encoder.clear_depth(&depth, 1.0f32);
	}

	pub fn end_frame<C: gfx::CommandBuffer<R>, D: gfx::Device<Resources = R, CommandBuffer = C>>(&self,
	                                           encoder: &mut gfx::Encoder<R, C>,
	                                           device: &mut D) {
		encoder.flush(device);
	}

	pub fn cleanup<C: gfx::CommandBuffer<R>, D: gfx::Device<Resources = R, CommandBuffer = C>>(&self, device: &mut D) {
		device.cleanup();
	}

	pub fn setup<C: gfx::CommandBuffer<R>>(&self,
	                                       encoder: &mut gfx::Encoder<R, C>,
	                                       camera: &Camera,
	                                       transform: &M44,
	                                       lights: &Vec<PointLight>) {

		let mut lights_buf = lights.clone();

		let count = lights_buf.len();
		while lights_buf.len() < 512 {
			lights_buf.push(PointLight {
				propagation: [0., 0., 0., 0.],
				color: [0., 0., 0., 0.],
				center: [0., 0., 0., 0.],
			})
		}
		// only one draw call per frame just to prove the point
		encoder.update_buffer(&self.lights, &lights_buf[..], 0).unwrap();

		encoder.update_constant_buffer(&self.vertex,
		                               &VertexArgs {
			                               proj: camera.projection.into(),
			                               view: camera.view.into(),
			                               model: transform.clone().into(),
		                               });

		encoder.update_constant_buffer(&self.fragment, &FragmentArgs { light_count: count as i32 });
	}

	pub fn draw<C: gfx::CommandBuffer<R>>(&self,
	                                      encoder: &mut gfx::Encoder<R, C>,
	                                      vertices: &gfx::handle::Buffer<R, VertexPosNormal>,
	                                      indices: &gfx::Slice<R>,
	                                      color: &gfx::handle::RenderTargetView<R, ColorFormat>,
	                                      output_depth: &gfx::handle::DepthStencilView<R, DepthFormat>) {

		encoder.draw(&indices,
		             &self.pso,
		             &shaded::Data {
			             vbuf: vertices.clone(),
			             fragment_args: self.fragment.clone(),
			             vertex_args: self.vertex.clone(),
			             lights: self.lights.clone(),
			             out_ka: color.clone(),
			             out_depth: output_depth.clone(),
		             });
	}
}
