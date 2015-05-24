use gl;
use gl::types::GLenum;
use math::{Mat4, Vec2f};
use shader::{Shader, Uniform};
use std::rc::Rc;
use std::vec::Vec;
use texture::Texture;
use vbo::VertexBuffer;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct StepId(usize);

pub struct Renderer {
    steps: Vec<RenderStep>,
    time: f32
}
impl Renderer {
    pub fn new() -> Renderer {
        Renderer { steps: Vec::new(), time: 0.0 }
    }

    pub fn add_step(&mut self, step: RenderStep) -> StepId {
        self.steps.push(step);
        StepId(self.steps.len() - 1)
    }

    pub fn step_mut(&mut self, id: StepId) -> &mut RenderStep {
        &mut self.steps[id.0]
    }

    pub fn render(&mut self, delta_time: f32, projection: &Mat4, modelview: &Mat4)
            -> &Renderer {
        check_gl_unsafe!(gl::Enable(gl::CULL_FACE));
        self.time += delta_time;
        for step in self.steps.iter() {
            step.render_setup(projection, modelview, self.time).render_done();
        }
        self
    }
}


pub struct RenderStep {
    shader: Shader,
    shared_tex: Vec<SharedTextureBinding>,
    unique_tex: Vec<TextureBinding>,
    static_vbos: Vec<VertexBuffer>,
    u_modelview: Option<Uniform>,
    u_projection: Option<Uniform>,
    u_time: Option<Uniform>,
}
impl RenderStep {
    pub fn new(shader: Shader) -> RenderStep {
        RenderStep {
            u_modelview: shader.get_uniform("u_modelview"),
            u_projection: shader.get_uniform("u_projection"),
            u_time: shader.get_uniform("u_time"),
            shader: shader,
            shared_tex: Vec::new(),
            unique_tex: Vec::new(),
            static_vbos: Vec::new(),
        }
    }

    pub fn shader(&mut self) -> &mut Shader { &mut self.shader }

    //pub fn get_uniform(&self, name: &str) -> Option<Uniform> {
    //    self.shader.get_uniform(name)
    //}

    pub fn add_constant_f32v(&mut self, name: &str, value: &[f32]) -> &mut RenderStep {
        let uniform = self.shader.expect_uniform(name);
        self.shader.bind_mut().set_uniform_f32v(uniform, value).unbind();
        self
    }

    pub fn add_constant_f32(&mut self, name: &str, value: f32) -> &mut RenderStep {
        let uniform = self.shader.expect_uniform(name);
        self.shader.bind_mut().set_uniform_f32(uniform, value).unbind();
        self
    }

    pub fn add_constant_vec2f(&mut self, name: &str, value: &Vec2f) -> &mut RenderStep {
        let uniform = self.shader.expect_uniform(name);
        self.shader.bind_mut().set_uniform_vec2f(uniform, value).unbind();
        self
    }

    pub fn add_shared_texture(&mut self, name: &str, texture: Rc<Texture>,
                              unit: usize) -> &mut RenderStep {
        let uniform = self.shader.expect_uniform(name);
        self.shader.bind_mut().set_uniform_i32(uniform, unit as i32).unbind();
        self.shared_tex.push((unit as GLenum + gl::TEXTURE0, texture));
        self
    }

    pub fn add_unique_texture(&mut self, name: &str, texture: Texture,
                              unit: usize) -> &mut RenderStep {
        let uniform = self.shader.expect_uniform(name);
        self.shader.bind_mut().set_uniform_i32(uniform, unit as i32).unbind();
        self.unique_tex.push((unit as GLenum + gl::TEXTURE0, texture));
        self
    }

    pub fn add_static_vbo(&mut self, vbo: VertexBuffer) -> &mut RenderStep {
        self.static_vbos.push(vbo);
        self
    }

    fn render_setup(&self, projection: &Mat4, modelview: &Mat4, time: f32)
           -> &RenderStep {
        self.shader.bind();
        self.u_modelview.map(|u| self.shader.set_uniform_mat4(u, modelview));
        self.u_projection.map(|u| self.shader.set_uniform_mat4(u, projection));
        self.u_time.map(|u| self.shader.set_uniform_f32(u, time));

        for &(unit, ref texture) in self.shared_tex.iter() {
            texture.bind(unit);
        }
        for &(unit, ref texture) in self.unique_tex.iter() {
            texture.bind(unit);
        }

        for vbo in self.static_vbos.iter() { vbo.draw_triangles(); }
        self
    }

    fn render_done(&self) -> &RenderStep {
        for &(unit, ref texture) in self.shared_tex.iter() {
            texture.unbind(unit);
        }
        for &(unit, ref texture) in self.unique_tex.iter() {
            texture.unbind(unit);
        }
        self.shader.unbind();
        self
    }

}


type TextureBinding = (GLenum, Texture);
type SharedTextureBinding = (GLenum, Rc<Texture>);
