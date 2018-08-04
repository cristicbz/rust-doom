pub struct Nil;
pub struct Cons<HeadT, TailT> {
    pub head: HeadT,
    pub tail: TailT,
}

pub enum Zero {}
pub type OnePlus<IndexT> = (IndexT,);

pub trait Pluck<LookupT, IndexT> {
    type Rest;

    fn pluck(self) -> (LookupT, Self::Rest);
}

pub trait Peek<LookupT, IndexT> {
    fn peek(&self) -> &LookupT;
    fn peek_mut(&mut self) -> &mut LookupT;
}

pub trait PluckInto<LookupT> {
    fn pluck_into(self) -> LookupT;
}

impl<T> PluckInto<T> for T {
    fn pluck_into(self) -> T {
        self
    }
}

impl<'a, T> PluckInto<&'a T> for &'a mut T {
    fn pluck_into(self) -> &'a T {
        self
    }
}

impl<LookupT, IndexT, ListT> Peek<LookupT, IndexT> for ListT
where
    for<'a> &'a ListT: Pluck<&'a LookupT, IndexT>,
    for<'a> &'a mut ListT: Pluck<&'a mut LookupT, IndexT>,
{
    fn peek(&self) -> &LookupT {
        self.pluck().0
    }

    fn peek_mut(&mut self) -> &mut LookupT {
        self.pluck().0
    }
}

impl<LookupT, HeadT, TailT> Pluck<LookupT, Zero> for Cons<HeadT, TailT>
where
    HeadT: PluckInto<LookupT>,
{
    type Rest = TailT;

    fn pluck(self) -> (LookupT, Self::Rest) {
        (self.head.pluck_into(), self.tail)
    }
}

impl<'a, LookupT, HeadT, TailT> Pluck<LookupT, Zero> for &'a Cons<HeadT, TailT>
where
    &'a HeadT: PluckInto<LookupT>,
{
    type Rest = &'a TailT;

    fn pluck(self) -> (LookupT, Self::Rest) {
        ((&self.head).pluck_into(), &self.tail)
    }
}

impl<'a, LookupT, HeadT, TailT> Pluck<LookupT, Zero> for &'a mut Cons<HeadT, TailT>
where
    &'a mut HeadT: PluckInto<LookupT>,
{
    type Rest = &'a mut TailT;

    fn pluck(self) -> (LookupT, Self::Rest) {
        ((&mut self.head).pluck_into(), &mut self.tail)
    }
}

impl<LookupT, HeadT, TailT, IndexT> Pluck<LookupT, OnePlus<IndexT>> for Cons<HeadT, TailT>
where
    TailT: Pluck<LookupT, IndexT>,
{
    type Rest = Cons<HeadT, TailT::Rest>;

    fn pluck(self) -> (LookupT, Self::Rest) {
        let (lookup, tail_rest) = self.tail.pluck();
        (
            lookup,
            Cons {
                head: self.head,
                tail: tail_rest,
            },
        )
    }
}

impl<'a, LookupT, HeadT, TailT, IndexT> Pluck<LookupT, OnePlus<IndexT>> for &'a Cons<HeadT, TailT>
where
    &'a TailT: Pluck<LookupT, IndexT>,
{
    type Rest = Cons<&'a HeadT, <&'a TailT as Pluck<LookupT, IndexT>>::Rest>;

    fn pluck(self) -> (LookupT, Self::Rest) {
        let (lookup, tail_rest) = self.tail.pluck();
        (
            lookup,
            Cons {
                head: &self.head,
                tail: tail_rest,
            },
        )
    }
}

impl<'a, LookupT, HeadT, TailT, IndexT> Pluck<LookupT, OnePlus<IndexT>>
    for &'a mut Cons<HeadT, TailT>
where
    &'a mut TailT: Pluck<LookupT, IndexT>,
{
    type Rest = Cons<&'a mut HeadT, <&'a mut TailT as Pluck<LookupT, IndexT>>::Rest>;

    fn pluck(self) -> (LookupT, Self::Rest) {
        let (lookup, tail_rest) = self.tail.pluck();
        (
            lookup,
            Cons {
                head: &mut self.head,
                tail: tail_rest,
            },
        )
    }
}

pub trait PluckList<NeedlesT, IndicesT> {
    type ListRest;

    fn pluck_list(self) -> (NeedlesT, Self::ListRest);
}

impl<ListT> PluckList<Nil, ()> for ListT {
    type ListRest = ListT;
    fn pluck_list(self) -> (Nil, ListT) {
        (Nil, self)
    }
}

impl<NeedleHeadT, NeedleTailT, HeadIndicesT, TailIndicesT, HaystackT>
    PluckList<Cons<NeedleHeadT, NeedleTailT>, (HeadIndicesT, TailIndicesT)> for HaystackT
where
    Self: Pluck<NeedleHeadT, HeadIndicesT>,
    <Self as Pluck<NeedleHeadT, HeadIndicesT>>::Rest: PluckList<NeedleTailT, TailIndicesT>,
{
    type ListRest = <<Self as Pluck<NeedleHeadT, HeadIndicesT>>::Rest as PluckList<
        NeedleTailT,
        TailIndicesT,
    >>::ListRest;

    fn pluck_list(self) -> (Cons<NeedleHeadT, NeedleTailT>, Self::ListRest) {
        let (head, rest) = self.pluck();
        let (tail, list_rest) = rest.pluck_list();
        (Cons { head, tail }, list_rest)
    }
}
