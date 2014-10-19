struct VboBuilder {
    level: &'a wad::Level,
    bounds: &'a TextureMaps,
    sky: Vec<SkyVertex>,
    flats: Vec<FlatVertex>,
    walls: Vec<WallVertex>,
    min_height: i16,
    max_height: i16,
}
impl<'a> VboBuilder<'a> {
    fn build(level: &wad::Level, bounds: &TextureMaps,
             steps: &mut RenderSteps) {
        let (min_height, max_height) = level.sectors
            .iter()
            .map(|s| (s.floor_height, s.ceiling_height))
            .fold((32767, -32768),
                  |(min, max), (f, c)| (if f < min { f } else { min },
                                        if c > max { c } else { max }));

        let mut builder = VboBuilder {
            level: level,
            bounds: bounds,
            flats: Vec::with_capacity(level.subsectors.len() * 4),
            walls: Vec::with_capacity(level.segs.len() * 2 * 4),
            sky: Vec::with_capacity(32),
            min_height: min_height,
            max_height: max_height,
        };
        let root_id = (level.nodes.len() - 1) as ChildId;
        builder.node(&mut Vec::with_capacity(32), root_id);

        let mut vbo = VboBuilder::init_sky_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.sky[]);
        steps.sky.add_static_vbo(vbo);

        let mut vbo = VboBuilder::init_flats_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.flats[]);
        steps.flats.add_static_vbo(vbo);

        let mut vbo = VboBuilder::init_walls_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.walls[]);
        steps.walls.add_static_vbo(vbo);

    }

    fn init_sky_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<SkyVertex>::new(2)
            .attribute_vec3f(0, offset_of!(SkyVertex, _pos))
            .build();
        buffer
    }

    fn init_flats_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<FlatVertex>::new(4)
            .attribute_vec3f(0, offset_of!(FlatVertex, _pos))
            .attribute_vec2f(1, offset_of!(FlatVertex, _atlas_uv))
            .attribute_u8(2, offset_of!(FlatVertex, _num_frames))
            .attribute_u8(3, offset_of!(FlatVertex, _frame_offset))
            .attribute_u16(4, offset_of!(FlatVertex, _light))
            .build();
        buffer
    }

    fn init_walls_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<WallVertex>::new(8)
            .attribute_vec3f(0, offset_of!(WallVertex, _pos))
            .attribute_vec2f(1, offset_of!(WallVertex, _tile_uv))
            .attribute_vec2f(2, offset_of!(WallVertex, _atlas_uv))
            .attribute_f32(3, offset_of!(WallVertex, _tile_width))
            .attribute_f32(4, offset_of!(WallVertex, _scroll_rate))
            .attribute_u8(5, offset_of!(WallVertex, _num_frames))
            .attribute_u8(6, offset_of!(WallVertex, _frame_offset))
            .attribute_u16(7, offset_of!(WallVertex, _light))
            .build();
        buffer
    }

    fn wall_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord),
                 texture_name: &WadName, peg: PegType) {
        if low >= high { return; }
        if is_untextured(texture_name) { return; }
        let bounds = match self.bounds.walls.find(texture_name) {
            None => {
                fail!("wall_quad: No such wall texture '{}'", texture_name);
            },
            Some(bounds) => bounds,
        };

        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (v1, v2) = self.level.seg_vertices(seg);
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = match peg {
            PegTopFloat => (from_wad_height(low + side.y_offset),
                            from_wad_height(low + bounds.size.y as i16 +
                                            side.y_offset)),
            PegBottomFloat => (from_wad_height(high + side.y_offset -
                                               bounds.size.y as i16),
                               from_wad_height(high + side.y_offset)),
            _ => (from_wad_height(low), from_wad_height(high))
        };

        let light_info = self.light_info(sector);
        let height = (high - low) * 100.0;
        let s1 = seg.offset as f32 + side.x_offset as f32;
        let s2 = s1 + (v2 - v1).norm() * 100.0;
        let (t1, t2) = match peg {
            PegTop => (height, 0.0),
            PegBottom => (bounds.size.y, bounds.size.y - height),
            PegBottomLower => {
                // As far as I can tell, this is a special case.
                let sector_height = (sector.ceiling_height -
                                     sector.floor_height) as f32;
                (bounds.size.y + sector_height,
                 bounds.size.y - height + sector_height)
            }
            PegTopFloat | PegBottomFloat => {
                (bounds.size.y, 0.0)
            }
        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        let scroll = if line.special_type == 0x30 { 35.0 }
                     else { 0.0 };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);
        self.wall_vertex(&v1, low,  s1, t1, light_info, scroll, bounds);
        self.wall_vertex(&v2, low,  s2, t1, light_info, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, light_info, scroll, bounds);

        self.wall_vertex(&v2, low,  s2, t1, light_info, scroll, bounds);
        self.wall_vertex(&v2, high, s2, t2, light_info, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, light_info, scroll, bounds);
    }

    fn flat_poly(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let light_info = self.light_info(sector);
        let floor_y = from_wad_height(sector.floor_height);
        let floor_tex = &sector.floor_texture;
        let ceil_y = from_wad_height(sector.ceiling_height);
        let ceil_tex = &sector.ceiling_texture;

        let v0 = points[0];
        if !is_sky_flat(floor_tex) {
            let floor_bounds = self.bounds.flats
                .find(floor_tex)
                .expect(format!("flat: No such floor {}.", floor_tex)[]);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.flat_vertex(&v0, floor_y, light_info, floor_bounds);
                self.flat_vertex(&v1, floor_y, light_info, floor_bounds);
                self.flat_vertex(&v2, floor_y, light_info, floor_bounds);
            }
        } else {
            let min = from_wad_height(self.min_height);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);

                self.sky_vertex(&v0, min);
                self.sky_vertex(&v1, min);
                self.sky_vertex(&v2, min);
            }
        }

        if !is_sky_flat(ceil_tex) {
            let ceiling_bounds = self.bounds.flats
                .find(ceil_tex)
                .expect(format!("flat: No such ceiling {}.", ceil_tex)[]);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.flat_vertex(&v2, ceil_y, light_info, ceiling_bounds);
                self.flat_vertex(&v1, ceil_y, light_info, ceiling_bounds);
                self.flat_vertex(&v0, ceil_y, light_info, ceiling_bounds);
            }
        } else {
            let max = from_wad_height(self.max_height);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);

                self.sky_vertex(&v2, max);
                self.sky_vertex(&v1, max);
                self.sky_vertex(&v0, max);
            }
        }
    }

    fn sky_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord)) {
        if low >= high { return; }
        let (v1, v2) = self.level.seg_vertices(seg);
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.sky_vertex(&v1, low);
        self.sky_vertex(&v2, low);
        self.sky_vertex(&v1, high);

        self.sky_vertex(&v2, low);
        self.sky_vertex(&v2, high);
        self.sky_vertex(&v1, high);
    }

    fn sky_vertex(&mut self, xz: &Vec2f, y: f32) {
        self.sky.push(SkyVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
        });
    }

    fn flat_vertex(&mut self, xz: &Vec2f, y: f32, light_info: u16,
                   bounds: &Bounds) {
        self.flats.push(FlatVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _atlas_uv: bounds.pos,
            _num_frames: bounds.num_frames as u8,
            _frame_offset: bounds.frame_offset as u8,
            _light: light_info,
        });
    }

    fn wall_vertex(&mut self, xz: &Vec2f, y: f32, tile_u: f32, tile_v: f32,
                   light_info: u16, scroll_rate: f32, bounds: &Bounds) {
        self.walls.push(WallVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _tile_uv: Vec2::new(tile_u, tile_v),
            _atlas_uv: bounds.pos,
            _tile_width: bounds.size.x,
            _scroll_rate: scroll_rate,
            _num_frames: bounds.num_frames as u8,
            _frame_offset: bounds.frame_offset as u8,
            _light: light_info,
        });
    }

    fn light_info(&self, sector: &WadSector) -> u16 {
        let light = sector.light;
        let sector_id = self.level.sector_id(sector);
        let sync : u16 = (sector_id as uint * 1664525 + 1013904223) as u16;
        let min_light_or = |if_same| {
            let min = self.level.sector_min_light(sector);
            if min == light { if_same } else { min }
        };
        let (alt_light, light_type, sync) = match sector.sector_type {
            1   => (min_light_or(0),     0, sync), // FLASH
            2|4 => (min_light_or(0),     3, sync), // FAST STROBE
            3   => (min_light_or(0),     1, sync), // SLOW STROBE
            8   => (min_light_or(light), 4, 0),    // GLOW
            12  => (min_light_or(0),     1, 0),    // SLOW STROBE SYNC
            13  => (min_light_or(0),     3, 0),    // FAST STROBE SYNC
            17  => (min_light_or(0),     2, sync), // FLICKER
            _   => (light, 0, 0),
        };
        (((light as u16 >> 3) & 31) << 11) +
        (((alt_light as u16 >> 3) & 31) << 6) +
        (light_type << 3) +
        (sync & 7)
    }

}
