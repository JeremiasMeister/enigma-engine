use enigma_3d::material::Material;
use enigma_3d::material::TextureType;
use glium::Display;
use glium::glutin::surface::WindowSurface;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::editor::state::{MaterialDef, ProjectState, ShaderChoice};
use crate::project::resource;

pub fn realize(
    def: &MaterialDef,
    project: &ProjectState,
    display: Display<WindowSurface>,
) -> Result<Material, RealizeError> {
    let mut mat = match &def.shader {
        ShaderChoice::PbrLit => Material::lit_pbr(display.clone(), def.transparent),
        ShaderChoice::Unlit => Material::unlit(display.clone(), def.transparent),
        ShaderChoice::Custom(_shader_uuid) => {
            // Custom shaders deferred — fall back to PBR for v1
            Material::lit_pbr(display.clone(), def.transparent)
        }
    };
    mat.set_name(&def.name);
    mat.set_color(def.color);
    mat.set_emissive_strength(def.emissive_strength);
    mat.set_roughness_strength(def.roughness_strength);
    mat.set_metallic_strength(def.metallic_strength);
    mat.set_normal_strength(def.normal_strength);
    if def.transparent {
        mat.set_transparency_strength(def.transparency_strength);
    }

    apply_texture(&mut mat, project, def.albedo,    TextureType::Albedo)?;
    apply_texture(&mut mat, project, def.normal,    TextureType::Normal)?;
    apply_texture(&mut mat, project, def.roughness, TextureType::Roughness)?;
    apply_texture(&mut mat, project, def.metallic,  TextureType::Metallic)?;
    apply_texture(&mut mat, project, def.emissive,  TextureType::Emissive)?;

    Ok(mat)
}

fn apply_texture(
    mat: &mut Material,
    project: &ProjectState,
    slot: Option<Uuid>,
    kind: TextureType,
) -> Result<(), RealizeError> {
    if let Some(uuid) = slot {
        let bytes = resource::bytes(project, uuid).map_err(|_| RealizeError::TextureNotFound)?;
        mat.set_texture_from_resource(&bytes, kind);
    }
    Ok(())
}

pub fn material_hash(def: &MaterialDef) -> u64 {
    let mut h = DefaultHasher::new();
    def.name.hash(&mut h);
    format!("{:?}", def.shader).hash(&mut h);
    def.albedo.hash(&mut h);
    def.normal.hash(&mut h);
    def.roughness.hash(&mut h);
    def.metallic.hash(&mut h);
    def.emissive.hash(&mut h);
    def.color.iter().for_each(|f| f.to_bits().hash(&mut h));
    def.emissive_strength.to_bits().hash(&mut h);
    def.roughness_strength.to_bits().hash(&mut h);
    def.metallic_strength.to_bits().hash(&mut h);
    def.normal_strength.to_bits().hash(&mut h);
    def.transparent.hash(&mut h);
    def.transparency_strength.to_bits().hash(&mut h);
    h.finish()
}

#[derive(Debug)]
pub enum RealizeError {
    TextureNotFound,
    ShaderNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_hash_stable_for_equal_defs() {
        let a = MaterialDef::default_pbr("m".into());
        let b = a.clone();
        assert_eq!(material_hash(&a), material_hash(&b));
    }

    #[test]
    fn material_hash_differs_for_changed_color() {
        let a = MaterialDef::default_pbr("m".into());
        let mut b = a.clone();
        b.color = [0.5, 0.5, 0.5];
        assert_ne!(material_hash(&a), material_hash(&b));
    }

    #[test]
    fn material_hash_differs_for_changed_albedo() {
        let a = MaterialDef::default_pbr("m".into());
        let mut b = a.clone();
        b.albedo = Some(Uuid::new_v4());
        assert_ne!(material_hash(&a), material_hash(&b));
    }
}
