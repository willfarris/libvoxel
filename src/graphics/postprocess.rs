use cgmath::{Vector3, Vector2};


use super::{shader::Shader, vertex::Vertex, mesh::{Mesh, Texture}};

pub const POSTPROCESS_VERTICES: [Vertex; 6] = [
    Vertex { position: Vector3::new( 1.0, -1.0, 0.0), normal: Vector3::new( 0.0,  0.0, -1.0), tex_coords: Vector2::new(1.0, 0.0) , vtype: 0 },   // Back-bottom-right
    Vertex { position: Vector3::new(-1.0, -1.0, 0.0), normal: Vector3::new( 0.0,  0.0, -1.0), tex_coords: Vector2::new(0.0, 0.0) , vtype: 0 },   // Back-bottom-left
    Vertex { position: Vector3::new(-1.0,  1.0, 0.0), normal: Vector3::new( 0.0,  0.0, -1.0), tex_coords: Vector2::new(0.0, 1.0) , vtype: 0 },   // Back-top-left

    Vertex { position: Vector3::new( 1.0, -1.0, 0.0), normal: Vector3::new( 0.0,  0.0, -1.0), tex_coords: Vector2::new(1.0, 0.0) , vtype: 0 },   // Back-bottom-right
    Vertex { position: Vector3::new(-1.0,  1.0, 0.0), normal: Vector3::new( 0.0,  0.0, -1.0), tex_coords: Vector2::new(0.0, 1.0) , vtype: 0 },   // Back-top-left
    Vertex { position: Vector3::new( 1.0,  1.0, 0.0), normal: Vector3::new( 0.0,  0.0, -1.0), tex_coords: Vector2::new(1.0, 1.0), vtype: 0  }     // Back-top-right
];

pub(crate) struct PostProcessMesh {
    pub(crate) mesh: Option<Mesh>,
    pub(crate) shader: Option<Shader>,
    pub(crate) render_texture: Option<Texture>,
    pub(crate) dimensions: (i32, i32),
}

impl PostProcessMesh {

    pub(crate) fn init(&mut self, shader: Shader, render_texture: Texture, dimensions: (i32, i32)) {   
        self.mesh = Some(Mesh::new(
            POSTPROCESS_VERTICES.to_vec(),
            &render_texture,
            &shader,
        ));

        self.shader = Some(shader);
        self.render_texture = Some(render_texture);
        self.dimensions = dimensions;

    }

    pub(crate) fn render(&mut self, elapsed_time: f32, render_texture_id: u32) {
        let shader = match self.shader.as_mut() {
            Some(s) => s,
            None => return
        };

        let mesh = match self.mesh.as_mut() {
            Some(m) => m,
            None => return
        };
        
        shader.use_program();
        unsafe {
            let sampler_str = crate::c_str!("renderedTexture").as_ptr();
            gl::Uniform1i(gl::GetUniformLocation(shader.id, sampler_str), 0);
            gl::BindTexture(gl::TEXTURE_2D, render_texture_id);
            shader.set_float(crate::c_str!("time"), elapsed_time);
            shader.set_vec3(crate::c_str!("resolution"), &Vector3::new(self.dimensions.0 as f32, self.dimensions.1 as f32, 0.0));
        }
        
        mesh.draw_from_texture(shader, self.render_texture.unwrap().id);
    }
}