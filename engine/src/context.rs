use super::errors::{ErrorKind, Result};
use super::system::{BoundSystem, System};
use super::type_list::{Cons, Nil, Peek, Pluck, PluckInto};
use failchain::ResultExt;
use log::info;
use std::marker::PhantomData;

pub trait Context {
    fn quit_requested(&self) -> bool;
    fn step(&mut self) -> Result<()>;
    fn destroy(&mut self) -> Result<()>;

    fn run(&mut self) -> Result<()> {
        while !self.quit_requested() {
            self.step()?;
        }
        Ok(())
    }
}

pub struct ControlFlow {
    pub quit_requested: bool,
}

pub struct ContextBuilder<SystemListT> {
    systems: SystemListT,
}

impl ContextBuilder<Cons<InjectMut<ControlFlow>, Nil>> {
    pub fn new() -> Self {
        Self {
            systems: Cons {
                head: InjectMut(ControlFlow {
                    quit_requested: false,
                }),
                tail: Nil,
            },
        }
    }
}

impl Default for ContextBuilder<Cons<InjectMut<ControlFlow>, Nil>> {
    fn default() -> Self {
        Self::new()
    }
}

impl<SystemListT> ContextBuilder<SystemListT> {
    pub fn inject<InjectT>(
        self,
        value: InjectT,
    ) -> ContextBuilder<Cons<Inject<InjectT>, SystemListT>> {
        ContextBuilder {
            systems: Cons {
                head: Inject(value),
                tail: self.systems,
            },
        }
    }

    pub fn inject_mut<InjectT>(
        self,
        value: InjectT,
    ) -> ContextBuilder<Cons<InjectMut<InjectT>, SystemListT>> {
        ContextBuilder {
            systems: Cons {
                head: InjectMut(value),
                tail: self.systems,
            },
        }
    }

    pub fn system<SystemT, IndicesT>(
        mut self,
        _: BoundSystem<SystemT>,
    ) -> Result<ContextBuilder<Cons<SystemT, SystemListT>>>
    where
        SystemT: for<'context> RawCreate<'context, SystemListT, IndicesT>,
    {
        let head = SystemT::raw_create(&mut self.systems)?;
        Ok(ContextBuilder {
            systems: Cons {
                head,
                tail: self.systems,
            },
        })
    }

    pub fn build<ControlIndexT, IndicesT>(
        mut self,
    ) -> Result<ContextObject<SystemListT, (ControlIndexT, IndicesT)>>
    where
        SystemListT: SystemList<IndicesT> + Peek<ControlFlow, ControlIndexT>,
    {
        SystemListT::setup_list(&mut self.systems).chain_err(|| ErrorKind::Context("setup"))?;
        info!("Context set up.");
        Ok(ContextObject {
            systems: Some(self.systems),
            indices: PhantomData,
        })
    }
}

pub struct ContextObject<SystemListT, IndicesT> {
    systems: Option<SystemListT>,
    indices: PhantomData<IndicesT>,
}

impl<SystemListT, IndicesT> ContextObject<SystemListT, IndicesT> {
    fn systems_mut(&mut self) -> &mut SystemListT {
        self.systems
            .as_mut()
            .expect("call on destroyed context: systems_mut")
    }

    fn systems(&self) -> &SystemListT {
        self.systems
            .as_ref()
            .expect("call on destroyed context: systems")
    }
}

impl<'a, SystemListT, IndicesT, LookupT, IndexT> Pluck<LookupT, IndexT>
    for &'a ContextObject<SystemListT, IndicesT>
where
    &'a SystemListT: Pluck<LookupT, IndexT>,
{
    type Rest = ();
    fn pluck(self) -> (LookupT, ()) {
        let (lookup, _) = self.systems().pluck();
        (lookup, ())
    }
}

impl<'a, SystemListT, IndicesT, LookupT, IndexT> Pluck<LookupT, IndexT>
    for &'a mut ContextObject<SystemListT, IndicesT>
where
    &'a mut SystemListT: Pluck<LookupT, IndexT>,
{
    type Rest = ();
    fn pluck(self) -> (LookupT, ()) {
        let (lookup, _) = self.systems_mut().pluck();
        (lookup, ())
    }
}

impl<SystemListT, ControlIndexT, IndicesT> Context
    for ContextObject<SystemListT, (ControlIndexT, IndicesT)>
where
    SystemListT: SystemList<IndicesT> + Peek<ControlFlow, ControlIndexT>,
{
    fn quit_requested(&self) -> bool {
        self.systems().peek().quit_requested
    }

    fn step(&mut self) -> Result<()> {
        SystemListT::update_list(self.systems_mut()).chain_err(|| ErrorKind::Context("update"))
    }

    fn destroy(&mut self) -> Result<()> {
        let mut systems = if let Some(systems) = self.systems.take() {
            systems
        } else {
            return Ok(());
        };
        SystemListT::teardown_list(&mut systems).chain_err(|| ErrorKind::Context("teardown"))?;
        info!("Context tore down.");
        SystemListT::destroy_list(systems).chain_err(|| ErrorKind::Context("destruction"))?;
        info!("Context destroyed.");
        Ok(())
    }
}

pub trait DependenciesFrom<ContextT, IndicesT>: Sized {
    fn dependencies_from(context: ContextT) -> Self;
}

impl<ContextT> DependenciesFrom<ContextT, ()> for () {
    fn dependencies_from(_: ContextT) -> Self {}
}

impl<'context, ContextT, IndexT, SystemT> DependenciesFrom<ContextT, IndexT> for &'context SystemT
where
    ContextT: Pluck<&'context SystemT, IndexT>,
{
    fn dependencies_from(context: ContextT) -> Self {
        let (this, _) = context.pluck();
        this
    }
}

impl<'context, ContextT, IndexT, SystemT> DependenciesFrom<ContextT, IndexT>
    for &'context mut SystemT
where
    ContextT: Pluck<&'context mut SystemT, IndexT>,
{
    fn dependencies_from(context: ContextT) -> Self {
        let (this, _) = context.pluck();
        this
    }
}

pub trait SystemList<IndicesT> {
    fn setup_list(&mut self) -> Result<()>;
    fn update_list(&mut self) -> Result<()>;
    fn teardown_list(&mut self) -> Result<()>;
    fn destroy_list(self) -> Result<()>;
}

impl SystemList<()> for Nil {
    fn setup_list(&mut self) -> Result<()> {
        Ok(())
    }

    fn update_list(&mut self) -> Result<()> {
        Ok(())
    }

    fn teardown_list(&mut self) -> Result<()> {
        Ok(())
    }

    fn destroy_list(self) -> Result<()> {
        Ok(())
    }
}

impl<HeadIndicesT, TailIndicesT, HeadT, TailT> SystemList<(HeadIndicesT, TailIndicesT)>
    for Cons<HeadT, TailT>
where
    TailT: SystemList<TailIndicesT>,
    HeadT: for<'context> RawSystem<'context, TailT, HeadIndicesT>,
{
    fn setup_list(&mut self) -> Result<()> {
        self.tail.setup_list()?;
        self.head.raw_setup(&mut self.tail)
    }

    fn update_list(&mut self) -> Result<()> {
        self.tail.update_list()?;
        self.head.raw_update(&mut self.tail)
    }

    fn teardown_list(&mut self) -> Result<()> {
        self.head.raw_teardown(&mut self.tail)?;
        self.tail.teardown_list()
    }

    fn destroy_list(mut self) -> Result<()> {
        self.head.raw_destroy(&mut self.tail)?;
        self.tail.destroy_list()
    }
}

pub struct Inject<InjectT>(pub InjectT);

impl<InjectT> PluckInto<InjectT> for Inject<InjectT> {
    fn pluck_into(self) -> InjectT {
        self.0
    }
}

impl<'a, InjectT> PluckInto<&'a InjectT> for &'a Inject<InjectT> {
    fn pluck_into(self) -> &'a InjectT {
        &self.0
    }
}

impl<'a, InjectT> PluckInto<&'a InjectT> for &'a mut Inject<InjectT> {
    fn pluck_into(self) -> &'a InjectT {
        &self.0
    }
}

pub struct InjectMut<InjectT>(pub InjectT);

impl<InjectT> PluckInto<InjectT> for InjectMut<InjectT> {
    fn pluck_into(self) -> InjectT {
        self.0
    }
}

impl<'a, InjectT> PluckInto<&'a InjectT> for &'a InjectMut<InjectT> {
    fn pluck_into(self) -> &'a InjectT {
        &self.0
    }
}

impl<'a, InjectT> PluckInto<&'a InjectT> for &'a mut InjectMut<InjectT> {
    fn pluck_into(self) -> &'a InjectT {
        &self.0
    }
}

impl<'a, InjectT> PluckInto<&'a mut InjectT> for &'a mut InjectMut<InjectT> {
    fn pluck_into(self) -> &'a mut InjectT {
        &mut self.0
    }
}

pub trait RawSystem<'context, ContextT, IndicesT>: Sized {
    #[inline]
    fn raw_setup(&mut self, _context: &'context mut ContextT) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn raw_update(&mut self, _context: &'context mut ContextT) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn raw_teardown(&mut self, _context: &'context mut ContextT) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn raw_destroy(self, _context: &'context mut ContextT) -> Result<()> {
        Ok(())
    }
}

pub trait RawCreate<'context, ContextT, IndicesT>: RawSystem<'context, ContextT, IndicesT> {
    fn raw_create(context: &'context mut ContextT) -> Result<Self>;
}

impl<'context, ContextT, IndicesT, SystemT> RawSystem<'context, ContextT, IndicesT> for SystemT
where
    ContextT: 'context,
    Self: System<'context>,
    <Self as System<'context>>::Dependencies: DependenciesFrom<&'context mut ContextT, IndicesT>,
{
    #[inline]
    fn raw_setup(&mut self, context: &'context mut ContextT) -> Result<()> {
        info!("Setting up system {:?}...", Self::debug_name());
        self.setup(<Self as System>::Dependencies::dependencies_from(context))
            .chain_err(|| ErrorKind::System("setup", Self::debug_name()))
    }

    #[inline]
    fn raw_update(&mut self, context: &'context mut ContextT) -> Result<()> {
        self.update(<Self as System>::Dependencies::dependencies_from(context))
            .chain_err(|| ErrorKind::System("update", Self::debug_name()))
    }

    #[inline]
    fn raw_teardown(&mut self, context: &'context mut ContextT) -> Result<()> {
        info!("Tearing down system {:?}...", Self::debug_name());
        self.teardown(<Self as System>::Dependencies::dependencies_from(context))
            .chain_err(|| ErrorKind::System("teardown", Self::debug_name()))
    }

    #[inline]
    fn raw_destroy(self, context: &'context mut ContextT) -> Result<()> {
        info!("Destroying system {:?}...", Self::debug_name());
        self.destroy(<Self as System>::Dependencies::dependencies_from(context))
            .chain_err(|| ErrorKind::System("destruction", Self::debug_name()))
    }
}

impl<'context, ContextT, IndicesT, SystemT> RawCreate<'context, ContextT, IndicesT> for SystemT
where
    ContextT: 'context,
    Self: System<'context>,
    <Self as System<'context>>::Dependencies: DependenciesFrom<&'context mut ContextT, IndicesT>,
{
    #[inline]
    fn raw_create(context: &'context mut ContextT) -> Result<Self> {
        info!("Creating system {:?}...", Self::debug_name());
        Self::create(<Self as System>::Dependencies::dependencies_from(context))
            .chain_err(|| ErrorKind::System("creation", Self::debug_name()))
    }
}

impl<'context, ContextT, InjectT> RawSystem<'context, ContextT, ()> for Inject<InjectT> {}
impl<'context, ContextT, InjectT> RawSystem<'context, ContextT, ()> for InjectMut<InjectT> {}
