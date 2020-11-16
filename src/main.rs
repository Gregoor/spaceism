use bevy::{prelude::*, render::pass::ClearColor};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::na::{Isometry2, UnitComplex};
use bevy_rapier2d::{
    na::Vector2,
    physics::{RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent},
    render::RapierRenderPlugin,
};
use rapier2d::{
    dynamics::{RigidBodyBuilder, RigidBodySet},
    geometry::ColliderBuilder,
    pipeline::PhysicsPipeline,
};
use std::ops::Neg;

static PLANET_RADIUS: f32 = 160.0;
static ATMO_RADIUS: f32 = PLANET_RADIUS * 2.0;

static GROUND_SPEED: f32 = 80.0;

struct Player;

struct Planet;

struct Bullet;

fn enable_physics_profiling(mut pipeline: ResMut<PhysicsPipeline>) {
    pipeline.counters.enable();
}

fn setup(
    mut commands: Commands,
    mut configuration: ResMut<RapierConfiguration>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    configuration.gravity = Default::default();

    commands
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(1000.0, 100.0, 2000.0)),
            ..Default::default()
        })
        .spawn(Camera2dComponents {
            transform: Transform::from_scale(Vec3::one()),
            ..Camera2dComponents::default()
        })
        .spawn(primitive(
            materials.add(Color::rgba(0.4, 0.7, 0.3, 0.2).into()),
            &mut meshes,
            ShapeType::Circle(ATMO_RADIUS),
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero(),
        ))
        .spawn(primitive(
            materials.add(Color::rgb(0.4, 0.6, 0.3).into()),
            &mut meshes,
            ShapeType::Circle(PLANET_RADIUS),
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero(),
        ))
        .with_bundle((
            Planet {},
            RigidBodyBuilder::new_static(),
            ColliderBuilder::ball(PLANET_RADIUS),
        ))
        .spawn((Player {},))
        .with_bundle((
            RigidBodyBuilder::new_dynamic().translation(0.0, 250.0),
            ColliderBuilder::cuboid(8.0, 23.0).restitution(-1.0),
        ));
}

fn gravitate(mut bodies: ResMut<RigidBodySet>, body_handle_component: &RigidBodyHandleComponent) {
    let mut body = bodies.get_mut(body_handle_component.handle()).unwrap();

    if body.is_static() {
        return;
    }

    let diff = Vector2::default() - body.position.translation.vector;
    let distance_squared = diff.magnitude_squared();
    let max_pull_distance_squared = (ATMO_RADIUS).powf(2.0);

    if distance_squared > max_pull_distance_squared {
        return;
    }
    let gravity = diff.normalize().scale(200_000.0);
    body.apply_force(gravity);
}

fn graviturn(
    mut bodies: ResMut<RigidBodySet>,
    _player: &Player,
    body_handle_component: &RigidBodyHandleComponent,
) {
    let mut body = bodies.get_mut(body_handle_component.handle()).unwrap();

    let translation_vector = body.position.translation.vector;
    let body_planet_distance = Vector2::default() - translation_vector;

    let center = Vector2::new(0.0, 1.0);
    let target_angle = body_planet_distance.angle(&center)
        * if center.x < translation_vector.x {
            1.0
        } else {
            -1.0
        };
    body.position = Isometry2::from_parts(body.position.translation, UnitComplex::new(target_angle))
}

fn handle_move(
    mut bodies: ResMut<RigidBodySet>,
    keyboard_input: Res<Input<KeyCode>>,
    _player: &Player,
    body_handle_component: &RigidBodyHandleComponent,
) {
    let mut body = bodies.get_mut(body_handle_component.handle()).unwrap();
    let direction: Vector2<f32> = (if keyboard_input.pressed(KeyCode::Left) {
        [-1.0, 0.0]
    } else if keyboard_input.pressed(KeyCode::Right) {
        [1.0, 0.0]
    } else if keyboard_input.pressed(KeyCode::Up) {
        [0.0, 1.0]
    } else if keyboard_input.pressed(KeyCode::Down) {
        [0.0, -1.0]
    } else {
        body.linvel.neg().into()
    })
    .into();
    let diff = Vector2::default() - body.position.translation.vector;
    if diff.magnitude_squared() > 33650.0 {
        // AKA HARD CODED IS GROUNDED CHECK
        return;
    }
    let clockwise = Vector2::new(diff.y, -diff.x).normalize();
    let is_clockwise = direction.angle(&clockwise) < std::f32::consts::PI / 2.0;

    let change = (if is_clockwise {
        clockwise
    } else {
        clockwise.neg()
    })
    .scale(GROUND_SPEED.powf(2.0))
        - body.linvel;
    body.apply_impulse(change);
}

fn handle_shooting(
    mut commands: Commands,
    mut bodies: ResMut<RigidBodySet>,
    mouse_button_input: Res<Input<MouseButton>>,
    _player: &Player,
    body_handle_component: &RigidBodyHandleComponent,
) {
    let body = bodies.get_mut(body_handle_component.handle()).unwrap();
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let v: Vector2<_> = body.position.translation.vector;
        commands.spawn((Bullet {},)).with_bundle((
            RigidBodyBuilder::new_dynamic()
                .translation(v.x, v.y)
                .linvel(60.0, 0.0),
            ColliderBuilder::cuboid(5.0, 5.0),
        ));
    }
}

fn main() {
    App::build()
        .add_resource(ClearColor(Color::rgb(
            0xF9 as f32 / 255.0,
            0xF9 as f32 / 255.0,
            0xFF as f32 / 255.0,
        )))
        .add_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_startup_system(setup.system())
        .add_startup_system(enable_physics_profiling.system())
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_system_to_stage(stage::UPDATE, gravitate.system())
        .add_system_to_stage(stage::UPDATE, graviturn.system())
        .add_system_to_stage(stage::UPDATE, handle_move.system())
        .add_system_to_stage(stage::UPDATE, handle_shooting.system())
        .run();
}
