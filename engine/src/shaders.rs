use super::entities::{Entities, Entity, EntityId};
use super::errors::{ErrorKind, Result};
use super::platform;
use super::system::InfallibleSystem;
use super::window::Window;
use crate::internal_derive::DependenciesFrom;

use failchain::ResultExt;
use glium::program::{Program, ProgramCreationInput};
use idcontain::IdMapVec;
use log::{debug, error};
use std::fs::File;
use std::io::Read;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct ShaderId(pub EntityId);

pub struct ShaderConfig {
    pub root_path: PathBuf,
}

pub struct Shaders {
    map: IdMapVec<Entity, Shader>,
    root: PathBuf,
}

impl Shaders {
    pub fn add(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        asset_path: &'static str,
    ) -> Result<ShaderId> {
        let mut fragment_path = self.root.clone();
        fragment_path.push(asset_path);

        let mut vertex_path = fragment_path.clone();
        fragment_path.set_extension("frag");
        vertex_path.set_extension("vert");

        let mut fragment_source = format!("#version {}\n", platform::GLSL_VERSION_STRING);
        let mut vertex_source = fragment_source.clone();

        debug!(
            "Loading shader {:?} (from {}, fragment={:?} and vert={:?})",
            name, asset_path, fragment_path, vertex_path
        );
        read_utf8_file(&fragment_path, &mut fragment_source)
            .chain_err(|| ErrorKind::ResourceIo("fragment shader", name))?;
        read_utf8_file(&vertex_path, &mut vertex_source)
            .chain_err(|| ErrorKind::ResourceIo("vertex shader", name))?;

        let program = Program::new(
            window.facade(),
            ProgramCreationInput::SourceCode {
                vertex_shader: &vertex_source,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                geometry_shader: None,
                fragment_shader: &fragment_source,
                transform_feedback_varyings: None,
                // TODO(cristicbz): More configurable things! SRGB should not be hard coded.
                outputs_srgb: true,
                uses_point_size: false,
            },
        )
        .map_err(ErrorKind::glium(name))?;
        debug!("Shader {:?} loaded successfully", name);
        let id = entities.add(parent, name)?;
        self.map.insert(id, Shader { program });
        debug!("Added shader {:?} {:?} as child of {:?}.", name, id, parent);
        Ok(ShaderId(id))
    }

    pub fn get(&self, shader_id: ShaderId) -> Option<&Program> {
        self.map.get(shader_id.0).map(|shader| &shader.program)
    }
}

pub struct Shader {
    program: Program,
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    config: &'context ShaderConfig,
    entities: &'context Entities,
}

impl<'context> InfallibleSystem<'context> for Shaders {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "shaders"
    }

    fn create(deps: Dependencies) -> Self {
        Shaders {
            map: IdMapVec::with_capacity(32),
            root: deps.config.root_path.clone(),
        }
    }

    fn update(&mut self, deps: Dependencies) {
        for &entity in deps.entities.last_removed() {
            if self.map.remove(entity).is_some() {
                debug!("Removed shader {:?}.", entity);
            }
        }
    }

    fn teardown(&mut self, deps: Dependencies) {
        self.update(deps);
    }

    fn destroy(mut self, deps: Dependencies) {
        self.update(deps);
        if !self.map.is_empty() {
            error!("Shaders leaked, {} instances.", self.map.len());
        }
    }
}

fn read_utf8_file(path: &Path, into: &mut String) -> IoResult<()> {
    File::open(path)?.read_to_string(into).map(|_| ())
}
