//! Viewport selection uses the same transformed sprite quads as drawing.
use crate::scene_controls::Flags;
use kiln_core::Vec2;
use kiln_runtime::{GameWorld, GlobalTransform, SourceId, Sprite};
use std::collections::HashMap;

fn quads(
    world: &GameWorld,
    flags: &HashMap<String, Flags>,
) -> Vec<(String, kiln_render::SpriteInstance, u64)> {
    let mut result = Vec::new();
    for (entity, source, pose, sprite) in world
        .world_ref()
        .query::<(hecs::Entity, &SourceId, &GlobalTransform, &Sprite)>()
        .iter()
    {
        if !flags.get(&source.0).is_some_and(|f| !f.locked && !f.hidden) {
            continue;
        }
        let order = world
            .world_ref()
            .get::<&kiln_runtime::SceneDrawOrder>(entity)
            .map(|v| v.0)
            .unwrap_or(entity.id() as u64);
        result.push((source.0.clone(), sprite.draw_instance(pose.0), order));
    }
    result.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));
    result
}

/// Pick the frontmost visible/unlocked authored sprite, including rotation/pivot/scale.
pub(crate) fn pick(
    world: &GameWorld,
    flags: &HashMap<String, Flags>,
    point: Vec2,
) -> Option<String> {
    let mut hits = Vec::new();
    for (entity, source, pose, sprite) in world
        .world_ref()
        .query::<(hecs::Entity, &SourceId, &GlobalTransform, &Sprite)>()
        .iter()
    {
        if !flags.get(&source.0).is_some_and(|f| !f.locked && !f.hidden) {
            continue;
        }
        let quad = sprite.draw_instance(pose.0);
        let delta = point - Vec2::from_array(quad.position);
        let (sin, cos) = quad.rotation.sin_cos();
        let local = Vec2::new(
            delta.x * cos + delta.y * sin,
            -delta.x * sin + delta.y * cos,
        );
        if quad.size.iter().all(|v| v.is_finite() && v.abs() > 0.0)
            && local.x.abs() <= quad.size[0].abs() / 2.0
            && local.y.abs() <= quad.size[1].abs() / 2.0
        {
            let order = world
                .world_ref()
                .get::<&kiln_runtime::SceneDrawOrder>(entity)
                .map(|v| v.0)
                .unwrap_or(entity.id() as u64);
            hits.push((sprite.layer, order, source.0.clone()));
        }
    }
    hits.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    hits.pop().map(|(_, _, id)| id)
}

/// Marquee selects intersecting world AABBs of visible/unlocked transformed quads.
pub(crate) fn rectangle(
    world: &GameWorld,
    flags: &HashMap<String, Flags>,
    a: Vec2,
    b: Vec2,
) -> Vec<String> {
    let low = a.min(b);
    let high = a.max(b);
    quads(world, flags)
        .into_iter()
        .filter_map(|(id, quad, _)| {
            let (sin, cos) = quad.rotation.sin_cos();
            let half = Vec2::new(
                (quad.size[0] * cos).abs() + (quad.size[1] * sin).abs(),
                (quad.size[0] * sin).abs() + (quad.size[1] * cos).abs(),
            ) / 2.0;
            let center = Vec2::from_array(quad.position);
            let left = center - half;
            let right = center + half;
            (left.x <= high.x && right.x >= low.x && left.y <= high.y && right.y >= low.y)
                .then_some(id)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::Transform;
    fn add(world: &mut GameWorld, id: &str, pose: Transform, sprite: Sprite, order: u64) {
        world.world_mut().spawn((
            SourceId(id.into()),
            GlobalTransform(pose),
            sprite,
            kiln_runtime::SceneDrawOrder(order),
        ));
    }
    #[test]
    fn picking_follows_painter_order_and_excludes_locked_hidden_and_nested_ids() {
        let mut world = GameWorld::new();
        let mut flags = HashMap::from([
            ("back".into(), Flags::default()),
            ("front".into(), Flags::default()),
        ]);
        add(
            &mut world,
            "front",
            Transform::new(50.0, 50.0),
            Sprite::new(1.0, 1.0, 1.0, 1.0),
            2,
        );
        add(
            &mut world,
            "back",
            Transform::new(50.0, 50.0),
            Sprite::new(1.0, 1.0, 1.0, 1.0),
            1,
        );
        assert_eq!(
            pick(&world, &flags, Vec2::new(50.0, 50.0)).as_deref(),
            Some("front")
        );
        flags.get_mut("front").unwrap().hidden = true;
        assert_eq!(
            pick(&world, &flags, Vec2::new(50.0, 50.0)).as_deref(),
            Some("back")
        );
        flags.get_mut("back").unwrap().locked = true;
        assert!(pick(&world, &flags, Vec2::new(50.0, 50.0)).is_none());
        assert!(rectangle(&world, &flags, Vec2::ZERO, Vec2::splat(100.0)).is_empty());
        add(
            &mut world,
            "nested",
            Transform::new(50.0, 50.0),
            Sprite::new(1.0, 1.0, 1.0, 1.0),
            3,
        );
        assert!(pick(&world, &flags, Vec2::new(50.0, 50.0)).is_none());
    }
    #[test]
    fn picking_and_marquee_use_pivot_rotation_negative_scale_and_pixels_per_unit() {
        let mut world = GameWorld::new();
        let flags = HashMap::from([("rotated".into(), Flags::default())]);
        let mut pose = Transform::new(100.0, 100.0);
        pose.rotation = std::f32::consts::FRAC_PI_2;
        pose.scale = Vec2::splat(-2.0);
        let mut sprite = Sprite::new(1.0, 1.0, 1.0, 1.0).with_size(40.0, 10.0);
        sprite.pivot = [0.0, 0.5];
        sprite.pixels_per_unit = 2.0;
        add(&mut world, "rotated", pose, sprite, 0);
        assert_eq!(
            pick(&world, &flags, Vec2::new(100.0, 80.0)).as_deref(),
            Some("rotated")
        );
        assert!(pick(&world, &flags, Vec2::new(112.0, 80.0)).is_none());
        assert_eq!(
            rectangle(
                &world,
                &flags,
                Vec2::new(105.0, 78.0),
                Vec2::new(99.0, 82.0)
            ),
            vec!["rotated"]
        );
    }
}
