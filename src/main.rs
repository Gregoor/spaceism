#![feature(option_result_contains)]

use bevy::{
    prelude::*,
    render::{camera::Camera, pass::ClearColor},
    window::WindowMode,
};
use bevy_contrib_bobox::{BodyHandleToEntity, RapierUtilsPlugin};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::{
    na::{Isometry2, UnitComplex, Vector2},
    physics::{EventQueue, RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent},
    render::RapierRenderPlugin,
};
use rapier2d::{
    dynamics::{RigidBodyBuilder, RigidBodySet},
    geometry::ColliderBuilder,
    ncollide::{narrow_phase::ContactEvent, query::Proximity},
};
use std::f32::consts::{FRAC_PI_2, PI};

static PLANET_RADIUS: f32 = 200.0;
static ATMOSPHERE_RADIUS: f32 = PLANET_RADIUS * 2.0;

#[derive(Debug, Default)]
struct Planet;

#[derive(Debug, Default)]
struct Atmosphere;

#[derive(Debug, Default)]
struct Player {
    is_grounded: bool,
}

#[derive(Debug, Default)]
struct Bullet {
    is_exploding: bool,
}

#[derive(Debug)]
struct Attractable(Option<Entity>);

#[derive(Debug, Default)]
struct Cursor {
    world_position: Vec2,
}

#[derive(Default)]
struct MouseState {
    cursor_moved_event_reader: EventReader<CursorMoved>,
}

fn setup(mut commands: Commands, mut configuration: ResMut<RapierConfiguration>) {
    configuration.gravity = Default::default();

    commands
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(1000.0, 100.0, 2000.0)),
            ..Default::default()
        })
        .spawn(Camera2dComponents {
            transform: Transform::from_scale(Vec3::one()),
            ..Camera2dComponents::default()
        });

    commands
        .spawn((Player::default(),))
        .with_bundle((
            Attractable(None),
            RigidBodyBuilder::new_dynamic().translation(0.0, 250.0),
            ColliderBuilder::cuboid(8.0, 23.0),
        ))
        .spawn((Cursor::default(),));
}

fn spawn_planets(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // TODO change to a non-zero planet, to make movement planet-independent
    for translation in vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(750.0, 500.0, 0.0)].into_iter() {
        commands
            .spawn(primitive(
                materials.add(Color::rgba(0.4, 0.7, 0.3, 0.2).into()),
                &mut meshes,
                ShapeType::Circle(ATMOSPHERE_RADIUS),
                TessellationMode::Fill(&FillOptions::default()),
                translation,
            ))
            .with_bundle((
                Atmosphere::default(),
                RigidBodyBuilder::new_static().translation(translation.x(), translation.y()),
                ColliderBuilder::ball(ATMOSPHERE_RADIUS).sensor(true),
            ))
            .spawn(primitive(
                materials.add(Color::rgb(0.4, 0.6, 0.3).into()),
                &mut meshes,
                ShapeType::Circle(PLANET_RADIUS),
                TessellationMode::Fill(&FillOptions::default()),
                translation,
            ))
            .with_bundle((
                Planet::default(),
                RigidBodyBuilder::new_static().translation(translation.x(), translation.y()),
                ColliderBuilder::ball(PLANET_RADIUS),
            ));
    }
}

fn get_planet_center(
    bodies: &ResMut<RigidBodySet>,
    atmosphere_query: &Query<(&Atmosphere, &RigidBodyHandleComponent)>,
    attractable: &Attractable,
) -> Option<Vector2<f32>> {
    attractable
        .0
        .and_then(|entity| {
            atmosphere_query
                .get_component::<RigidBodyHandleComponent>(entity)
                .ok()
        })
        .and_then(|component| bodies.get(component.handle()))
        .and_then(|body| Some(body.position.translation.vector))
}

fn gravitate(
    mut bodies: ResMut<RigidBodySet>,
    attractable_body_query: Query<(&Attractable, &RigidBodyHandleComponent)>,
    atmosphere_query: Query<(&Atmosphere, &RigidBodyHandleComponent)>,
) {
    for (attractable, body_handle) in attractable_body_query.iter() {
        let planet_center = match get_planet_center(&bodies, &atmosphere_query, attractable) {
            Some(vector) => vector,
            None => continue,
        };

        let mut body = bodies.get_mut(body_handle.handle()).unwrap();
        let diff = planet_center - body.position.translation.vector;
        let distance_squared = diff.magnitude_squared();
        let max_pull_distance_squared = (ATMOSPHERE_RADIUS).powf(2.0);

        if distance_squared > max_pull_distance_squared {
            continue;
        }
        let gravity = diff.normalize().scale(200_000.0);
        body.apply_force(gravity);
    }
}

fn graviturn(
    mut bodies: ResMut<RigidBodySet>,
    player_query: Query<(&Player, &Attractable, &RigidBodyHandleComponent)>,
    atmosphere_query: Query<(&Atmosphere, &RigidBodyHandleComponent)>,
) {
    for (_, attractable, body_handle_component) in player_query.iter() {
        let planet_center = match get_planet_center(&bodies, &atmosphere_query, attractable) {
            Some(vector) => vector,
            None => continue,
        };

        let mut body = bodies.get_mut(body_handle_component.handle()).unwrap();

        let translation_vector = body.position.translation.vector;
        let body_planet_distance = planet_center - translation_vector;

        let center = Vector2::new(0.0, 1.0);
        let target_angle = body_planet_distance.angle(&center)
            * if center.x < translation_vector.x {
                1.0
            } else {
                -1.0
            };
        body.position =
            Isometry2::from_parts(body.position.translation, UnitComplex::new(target_angle))
    }
}

fn move_player(
    mut bodies: ResMut<RigidBodySet>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    player_query: Query<(&Player, &Attractable, &RigidBodyHandleComponent)>,
    atmosphere_query: Query<(&Atmosphere, &RigidBodyHandleComponent)>,
) {
    for (player, attractable, body_handle_component) in player_query.iter() {
        let planet_center = match get_planet_center(&bodies, &atmosphere_query, attractable) {
            Some(vector) => vector,
            None => continue,
        };
        let mut body = bodies.get_mut(body_handle_component.handle()).unwrap();
        let diff = planet_center - body.position.translation.vector;

        let directions: [(KeyCode, (f32, f32)); 4] = [
            (KeyCode::A, (-1.0, 0.0)),
            (KeyCode::D, (1.0, 0.0)),
            (KeyCode::W, (0.0, 1.0)),
            (KeyCode::S, (0.0, -1.0)),
        ];
        let direction = directions
            .iter()
            .fold(Vector2::zeros(), |sum, (key_code, v)| {
                if keyboard_input.pressed(*key_code) {
                    sum + Vector2::new(v.0, v.1)
                } else {
                    sum
                }
            });

        let has_direction = direction.magnitude_squared() > 0.0;

        let clockwise = Vector2::new(diff.y, -diff.x).normalize();
        let is_clockwise = direction.angle(&clockwise) < FRAC_PI_2;
        let direction_factor = if is_clockwise { 1.0 } else { -1.0 };

        if player.is_grounded && keyboard_input.pressed(KeyCode::Space) {
            body.apply_impulse(
                (diff
                    + if has_direction {
                        clockwise * direction_factor
                    } else {
                        Vector2::default()
                    })
                .normalize()
                    * -30000.0,
            );
            continue;
        }

        if !has_direction {
            continue;
        }

        if !player.is_grounded {
            body.apply_impulse((clockwise * direction_factor * 2.0 + diff.normalize()) * 1000.0);
            continue;
        }

        let planet_angle = diff.y.atan2(diff.x);
        let new_planet_angle = PI + planet_angle + time.delta_seconds * 1.2 * direction_factor;

        let body_angle = body.position.rotation.angle();
        body.set_position(Isometry2::new(
            Vector2::new(new_planet_angle.cos(), new_planet_angle.sin())
                .scale(PLANET_RADIUS + 23.0),
            body_angle,
        ));
    }
}

fn aim(
    mut cursor: ResMut<Cursor>,
    mut state: Local<MouseState>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    query: Query<(&Camera, &Transform)>,
) {
    let cursor_position = match state.cursor_moved_event_reader.latest(&cursor_moved_events) {
        Some(event) => event.position,
        None => return,
    };

    let window = match windows.get_primary() {
        Some(w) => w,
        None => return,
    };
    for (_, transform) in query.iter() {
        let translation = Vec2::new(transform.translation.x(), transform.translation.y());
        let window_center = Vec2::new(window.width() as f32, window.height() as f32) * 0.5;
        cursor.world_position =
            translation + ((cursor_position - window_center) * transform.scale.x());
    }
}

fn shoot(
    mut commands: Commands,
    mut bodies: ResMut<RigidBodySet>,
    cursor: Res<Cursor>,
    mouse_button_input: Res<Input<MouseButton>>,
    _player: &Player,
    body_handle_component: &RigidBodyHandleComponent,
) {
    let body = bodies.get_mut(body_handle_component.handle()).unwrap();
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let body_vector = Vec2::from_slice_unaligned(body.position.translation.vector.as_slice());
        let direction = (cursor.world_position - body_vector).normalize();
        let start_at = body_vector + direction * 30.0;
        let vel = direction * 700.0;
        let entity = commands
            .spawn((Bullet::default(),))
            .current_entity()
            .unwrap();
        commands.with_bundle((
            Attractable(None),
            RigidBodyBuilder::new_dynamic()
                .translation(start_at.x(), start_at.y())
                .linvel(vel.x(), vel.y()),
            ColliderBuilder::cuboid(5.0, 5.0).user_data(entity.id().into()),
        ));
    }
}

fn physics_events(
    mut commands: Commands,
    events: Res<EventQueue>,
    body_handle_to_entity: Res<BodyHandleToEntity>,

    atmosphere_query: Query<(&Atmosphere, &RigidBodyHandleComponent)>,
    mut attractable_query: Query<&mut Attractable>,

    planet_query: Query<&Planet>,
    mut player_query: Query<&mut Player>,
    bullet_query: Query<&Bullet>,
) {
    while let Ok(proximity_event) = events.proximity_events.pop() {
        let entities: Vec<Entity> = [proximity_event.collider1, proximity_event.collider2]
            .iter()
            .filter_map(|handle| body_handle_to_entity.0.get(&handle))
            .map(|entity| *entity)
            .collect();

        if let Some(atmosphere_entity) = entities.iter().find(|entity| {
            atmosphere_query
                .get_component::<RigidBodyHandleComponent>(**entity)
                .is_ok()
        }) {
            for entity in &entities {
                let mut attractable =
                    match attractable_query.get_component_mut::<Attractable>(*entity) {
                        Ok(attractable) => attractable,
                        Err(..) => {
                            continue;
                        }
                    };
                match proximity_event.new_status {
                    Proximity::Intersecting => {
                        attractable.0 = Some(atmosphere_entity.clone());
                    }
                    Proximity::WithinMargin => {}
                    Proximity::Disjoint => {
                        if attractable.0.contains(atmosphere_entity) {
                            attractable.0 = None;
                        }
                    }
                };
            }
        }
    }

    while let Ok(contact_event) = events.contact_events.pop() {
        let (handles, is_started) = match contact_event {
            ContactEvent::Started(handle1, handle2) => ([handle1, handle2], true),
            ContactEvent::Stopped(handle1, handle2) => ([handle1, handle2], false),
        };
        let entities: Vec<Entity> = handles
            .iter()
            .filter_map(|handle| body_handle_to_entity.0.get(&handle))
            .map(|entity| *entity)
            .collect();

        for bullet_entity in entities
            .iter()
            .filter(|entity| bullet_query.get(**entity).is_ok())
        {
            commands.despawn(*bullet_entity);
        }

        if entities
            .iter()
            .any(|entity| planet_query.get(*entity).is_ok())
        {
            for entity in entities {
                if let Ok(mut player) = player_query.get_component_mut::<Player>(entity) {
                    player.is_grounded = is_started;
                }
            }
        }
    }
}

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            mode: WindowMode::Windowed,
            width: 1920,
            height: 1080,
            ..Default::default()
        })
        .add_resource(ClearColor(Color::rgb(
            0xF9 as f32 / 255.0,
            0xF9 as f32 / 255.0,
            0xFF as f32 / 255.0,
        )))
        .add_resource(Msaa::default())
        .init_resource::<Cursor>()
        //
        .add_stage_after(stage::POST_UPDATE, "HANDLE_CONTACT")
        .add_stage_after("HANDLE_CONTACT", "HANDLE_EXPLOSION")
        .add_stage_after("HANDLE_EXPLOSION", "HANDLE_RUNSTATE")
        .add_stage_after("HANDLE_RUNSTATE", "CLEANUP")
        //
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_plugin(RapierUtilsPlugin)
        //
        .add_startup_system(setup.system())
        .add_startup_system(spawn_planets.system())
        //
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_system_to_stage(stage::UPDATE, gravitate.system())
        .add_system_to_stage(stage::UPDATE, graviturn.system())
        .add_system_to_stage(stage::UPDATE, move_player.system())
        .add_system_to_stage(stage::UPDATE, aim.system())
        .add_system_to_stage(stage::UPDATE, shoot.system())
        .add_system_to_stage(stage::POST_UPDATE, physics_events.system())
        .run();
}
