
struct LevelBuilder<'a> {
    level_data: &'a Level,
    sky_builder: SkyBuilder,
    flat_builder: FlatBuilder,
    wall_builder: WallBuilder,
}
impl<'a> LevelBuilder<'a> {
    pub fn new(level_data: &Level,
               sky_builder: SkyBuilder,
               flat_builder: FlatBuilder,
               wall_builder: WallBuilder) {
        LevelBuilder {
            level_data: level_data,
            sky_builder: SkyBuilder,
            flat_builder: FlatBuilder,
            wall_builder: WallBuilder,
        }
    }

    pub fn add_seg(&mut self, seg: &WadSeg) {
    }
}

