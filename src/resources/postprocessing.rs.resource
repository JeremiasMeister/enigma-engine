//! Native project effects, shared by the game and the project editor.
use enigma_3d::postprocessing::stack::{EffectRegistry, Parameter};
use enigma_3d::postprocessing::PostProcessingEffect;
use glium::{uniform, Surface};

pub fn register(registry: &mut EffectRegistry) -> Result<(), String> {
    registry.register("project.tint", "Project Tint", vec![
        Parameter::color("tint", "Tint", [1.0, 0.75, 0.5]),
        Parameter::float("strength", "Strength", 0.25, 0.0, 1.0),
    ], |display, parameters| {
        Ok(Tint {
            program: glium::Program::from_source(display,
                enigma_3d::resources::post_processing_vertex(),
                include_str!("tint.glsl"), None).map_err(|e| e.to_string())?,
            tint: parameters.color("tint")?,
            strength: parameters.float("strength")?,
        })
    }, |effect, parameters| {
        effect.tint = parameters.color("tint")?;
        effect.strength = parameters.float("strength")?;
        Ok(())
    })
}

struct Tint {
    program: glium::Program,
    tint: [f32; 3],
    strength: f32,
}

impl PostProcessingEffect for Tint {
    fn render(&self,
        _app: &enigma_3d::AppState,
        vertices: &glium::VertexBuffer<enigma_3d::geometry::Vertex>,
        indices: &glium::IndexBuffer<u32>,
        target: &mut glium::framebuffer::SimpleFrameBuffer,
        source: &glium::Texture2d,
        _depth: &glium::texture::DepthTexture2d,
        _buffers: &Vec<glium::Texture2d>,
    ) {
        let uniforms = uniform! { scene: source, tint: self.tint, strength: self.strength };
        if let Err(error) = target.draw(vertices, indices, &self.program, &uniforms, &Default::default()) {
            eprintln!("Project Tint: {error}");
        }
    }
}
