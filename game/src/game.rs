use super::errors::Result;
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
use std::marker::PhantomData;
use std::path::PathBuf;

pub struct Game(Box<dyn AbstractGame>);

impl Game {
    pub fn new(config: &GameConfig) -> Result<Self> {
        let context = ContextBuilder::new()
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
            .build()?;

        Ok(Self(ContextWrapper::boxed(context)))
    }

    pub fn run(&mut self) -> Result<()> {
        let result = self.0.run();
        result.and(self.0.destroy())
    }

    pub fn num_levels(&self) -> usize {
        self.0.num_levels()
    }

    pub fn load_level(&mut self, level_index: usize) -> Result<()> {
        self.0.load_level(level_index)
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = self.0.destroy();
    }
}

pub struct ContextWrapper<WadIndexT, ContextT> {
    context: ContextT,
    phantom: PhantomData<WadIndexT>,
}

impl<WadIndexT, ContextT> ContextWrapper<WadIndexT, ContextT>
where
    ContextT: Context + Peek<WadSystem, WadIndexT> + 'static,
    WadIndexT: 'static,
{
    fn boxed(context: ContextT) -> Box<dyn AbstractGame> {
        Box::new(Self {
            context,
            phantom: PhantomData,
        })
    }
}

pub trait AbstractGame {
    fn run(&mut self) -> Result<()>;
    fn destroy(&mut self) -> Result<()>;
    fn num_levels(&self) -> usize;
    fn load_level(&mut self, level_index: usize) -> Result<()>;
}

impl<WadIndexT, ContextT> AbstractGame for ContextWrapper<WadIndexT, ContextT>
where
    ContextT: Context + Peek<WadSystem, WadIndexT> + 'static,
{
    fn num_levels(&self) -> usize {
        let wad = self.context.peek();
        wad.archive.num_levels()
    }

    fn load_level(&mut self, level_index: usize) -> Result<()> {
        let wad = self.context.peek_mut();
        wad.change_level(level_index);
        self.context.step()?;
        self.context.step()?;
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.context.run()?;
        Ok(())
    }

    fn destroy(&mut self) -> Result<()> {
        self.context.destroy()?;
        Ok(())
    }
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
