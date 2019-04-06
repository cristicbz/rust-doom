use super::errors::{ErrorKind, Result};
use super::game_shaders::GameShaders;
use super::hud::{Bindings as HudBindings, Hud};
use super::level::Level;
use super::player::{Bindings as PlayerBindings, Config as PlayerConfig, Player};
use super::wad_system::{Config as WadConfig, WadSystem};
use super::SHADER_ROOT;
use engine::type_list::Peek;
use engine::{
    Context, ContextBuilder, Entities, FrameTimers, Input, Materials, Meshes, Projections,
    RenderPipeline, Renderer, ShaderConfig, Shaders, System, TextRenderer, Tick, TickConfig,
    Transforms, Uniforms, Window, WindowConfig,
};
use failchain::ResultExt;
use std::marker::PhantomData;
use std::path::PathBuf;

pub trait Game {
    fn run(&mut self) -> Result<()>;
    fn destroy(&mut self) -> Result<()>;
    fn num_levels(&self) -> usize;
    fn load_level(&mut self, level_index: usize) -> Result<()>;
}

#[derive(Clone)]
pub struct GameConfig {
    pub wad_file: PathBuf,
    pub metadata_file: PathBuf,
    pub fov: f32,
    pub width: u32,
    pub height: u32,
    pub version: &'static str,
    pub initial_level_index: usize,
}

pub fn create(config: &GameConfig) -> Result<impl Game> {
    let context = (|| {
        ContextBuilder::new()
            // Engine configs and systems.
            .inject(TickConfig {
                timestep: 1.0 / 60.0,
            })
            .inject(WindowConfig {
                width: config.width,
                height: config.height,
                title: format!("Rusty Doom v{}", config.version),
            })
            .inject(ShaderConfig {
                root_path: SHADER_ROOT.into(),
            })
            .system(Tick::bind())?
            .system(FrameTimers::bind())?
            .system(Window::bind())?
            .system(Input::bind())?
            .system(Entities::bind())?
            .system(Transforms::bind())?
            .system(Projections::bind())?
            .system(Shaders::bind())?
            .system(Uniforms::bind())?
            .system(Meshes::bind())?
            .system(Materials::bind())?
            .system(RenderPipeline::bind())?
            .system(TextRenderer::bind())?
            // Game configs and systems.
            .inject(WadConfig {
                wad_path: config.wad_file.clone(),
                metadata_path: config.metadata_file.clone(),
                initial_level_index: config.initial_level_index,
            })
            .inject(HudBindings::default())
            .inject(PlayerBindings::default())
            .inject(PlayerConfig::default())
            .system(WadSystem::bind())?
            .system(GameShaders::bind())?
            .system(Level::bind())?
            .system(Hud::bind())?
            .system(Player::bind())?
            .system(Renderer::bind())?
            .build()
    })()
    .chain_err(|| ErrorKind("during setup".to_owned()))?;

    Ok(GameImpl::new(context))
}

struct GameImpl<WadIndexT, ContextT>
where
    ContextT: Context + Peek<WadSystem, WadIndexT>,
{
    context: ContextT,
    phantom: PhantomData<WadIndexT>,
}

impl<WadIndexT, ContextT> GameImpl<WadIndexT, ContextT>
where
    ContextT: Context + Peek<WadSystem, WadIndexT>,
{
    fn new(context: ContextT) -> Self {
        Self {
            context,
            phantom: PhantomData,
        }
    }
}

impl<WadIndexT, ContextT> Game for GameImpl<WadIndexT, ContextT>
where
    ContextT: Context + Peek<WadSystem, WadIndexT>,
{
    fn run(&mut self) -> Result<()> {
        self.context
            .run()
            .chain_err(|| ErrorKind("during run".to_owned()))?;
        self.context
            .destroy()
            .chain_err(|| ErrorKind("during shutdown".to_owned()))?;
        Ok(())
    }

    fn num_levels(&self) -> usize {
        let wad = self.context.peek();
        wad.archive.num_levels()
    }

    fn load_level(&mut self, level_index: usize) -> Result<()> {
        let wad = self.context.peek_mut();
        wad.change_level(level_index);
        self.context
            .step()
            .chain_err(|| ErrorKind("during load_level first step".to_owned()))?;
        self.context
            .step()
            .chain_err(|| ErrorKind("during load_level second step".to_owned()))?;
        Ok(())
    }

    fn destroy(&mut self) -> Result<()> {
        self.context
            .destroy()
            .chain_err(|| ErrorKind("during explicit destroy".to_owned()))?;
        Ok(())
    }
}

impl<WadIndexT, ContextT> Drop for GameImpl<WadIndexT, ContextT>
where
    ContextT: Context + Peek<WadSystem, WadIndexT>,
{
    fn drop(&mut self) {
        let _ = self.context.destroy();
    }
}
