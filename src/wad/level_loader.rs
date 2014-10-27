


struct LevelLoader {
}
impl LevelLoader {
    fn node(&mut self, lines: &mut Vec<Line2f>, id: ChildId) {
        let (id, is_leaf) = parse_child_id(id);
        if is_leaf {
            self.subsector(lines[mut], id);
            return;
        }

        let node = &self.level.nodes[id];
        let partition = Line2::from_origin_and_displace(
            from_wad_coords(node.line_x, node.line_y),
            from_wad_coords(node.step_x, node.step_y));
        lines.push(partition);
        self.node(lines, node.left);
        lines.pop();

        lines.push(partition.inverted_halfspaces());
        self.node(lines, node.right);
        lines.pop();
    }

    fn subsector(&mut self, lines: &[Line2f], id: uint) {
        let segs = self.level.ssector_segs(&self.level.subsectors[id]);

        // The vector contains all (2D) points which are part of the subsector:
        // implicit (intersection of BSP lines) and explicit (seg vertices).
        let mut points = Vec::with_capacity(segs.len() * 3);
        let mut seg_lines = Vec::with_capacity(segs.len());

        // First add the explicit points.
        for seg in segs.iter() {
            let (v1, v2) = self.level.seg_vertices(seg);
            points.push(v1);
            points.push(v2);
            seg_lines.push(Line2::from_two_points(v1, v2));

            // Also push the wall segments.
            self.seg(seg);
        }

        // The convex polyon defined at the intersection of the partition lines,
        // intersected with the half-volumes of the segs form the 'implicit'
        // points.
        for i_line in range(0, lines.len() - 1) {
            for j_line in range(i_line + 1, lines.len()) {
                let (l1, l2) = (&(*lines)[i_line], &(*lines)[j_line]);
                let point = match l1.intersect_point(l2) {
                    Some(p) => p,
                    None => continue
                };

                let dist = |l: &Line2f| l.signed_distance(&point);

                // The intersection point must lie both within the BSP volume
                // and the segs volume.
                if lines.iter().map(|x| dist(x)).all(|d| d >= -BSP_TOLERANCE)
                   && seg_lines.iter().map(dist).all(|d| d <= SEG_TOLERANCE) {
                    points.push(point);
                }
            }
        }
        if points.len() < 3 {
            warn!("Degenerate source polygon {} ({} vertices).",
                  id, points.len());
        }
        points_to_polygon(&mut points);  // Sort and remove duplicates.
        if points.len() < 3 {
            warn!("Degenerate cannonicalised polygon {} ({} vertices).",
                  id, points.len());
        } else {
            self.flat_poly(self.level.seg_sector(&segs[0]), points[]);
        }
    }

    fn seg(&mut self, seg: &WadSeg) {
        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (min, max) = (self.min_height, self.max_height);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);
        let unpeg_lower = line.lower_unpegged();
        let back_sector = match self.level.seg_back_sector(seg) {
            None => {
                self.wall_quad(seg, (floor, ceil), &side.middle_texture,
                               if unpeg_lower { PegBottom } else { PegTop });
                if is_sky_flat(&sector.ceiling_texture) {
                    self.sky_quad(seg, (ceil, max));
                }
                if is_sky_flat(&sector.floor_texture) {
                    self.sky_quad(seg, (min, floor));
                }
                return
            },
            Some(s) => s
        };

        if is_sky_flat(&sector.ceiling_texture)
                && !is_sky_flat(&back_sector.ceiling_texture)
                && !is_untextured(&side.upper_texture) {
            self.sky_quad(seg, (ceil, max));
        }
        if is_sky_flat(&sector.floor_texture)
                && !is_sky_flat(&back_sector.floor_texture)
                && !is_untextured(&side.lower_texture) {
            self.sky_quad(seg, (min, floor));
        }

        let unpeg_upper = line.upper_unpegged();
        let back_floor = back_sector.floor_height;
        let back_ceil = back_sector.ceiling_height;
        let floor = if back_floor > floor {
            self.wall_quad(seg, (floor, back_floor), &side.lower_texture,
                           if unpeg_lower { PegBottomLower } else { PegTop });
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            if !is_sky_flat(&back_sector.ceiling_texture) {
                self.wall_quad(seg, (back_ceil, ceil), &side.upper_texture,
                               if unpeg_upper { PegTop } else { PegBottom });
            }
            back_ceil
        } else {
            ceil
        };
        self.wall_quad(seg, (floor, ceil), &side.middle_texture,
                       if unpeg_lower { PegTopFloat } else { PegBottomFloat });

    }

}



