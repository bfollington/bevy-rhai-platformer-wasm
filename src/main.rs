mod post_processing;

use avian2d::{prelude as avian, prelude::*};
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy_editor_pls::prelude::*;
use bevy_scriptum::runtimes::rhai::prelude::*;
use bevy_scriptum::{prelude::*, ScriptingRuntimeBuilder};
use bevy_tnua::{builtins::TnuaBuiltinCrouch, prelude::*};
use bevy_tnua_avian2d::*;
use post_processing::{PostProcessPlugin, PostProcessSettings};
use rhai::ImmutableString;

#[derive(Component)]
struct DesiredVelocity(Vec3);

#[derive(Component)]
struct JumpQueued(bool);

fn setup_rhai<'a>(runtime: ScriptingRuntimeBuilder<'a, RhaiRuntime>) {
    runtime
        .add_function(
            String::from("print_message"),
            |In((x,)): In<(ImmutableString,)>| {
                println!("called with string: '{}'", x);
            },
        )
        .add_function(
            String::from("read_input"),
            |In((key,)): In<(ImmutableString,)>, keyboard_input: Res<ButtonInput<KeyCode>>| {
                let pressed = match key.as_str() {
                    "A" => keyboard_input.pressed(KeyCode::KeyA),
                    "D" => keyboard_input.pressed(KeyCode::KeyD),
                    "S" => keyboard_input.pressed(KeyCode::KeyS),
                    "Space" => keyboard_input.just_pressed(KeyCode::Space),
                    _ => false,
                };

                pressed
            },
        )
        .add_function(
            String::from("set_desired_velocity"),
            |In((entity, x, y)): In<(Entity, f32, f32)>, world: &mut World| {
                if let Some(mut entity_ref) = world.get_entity_mut(entity) {
                    let desired_velocity = Vec3::new(x, y, 0.0);
                    entity_ref.insert(DesiredVelocity(desired_velocity));
                }
            },
        )
        .add_function(
            String::from("queue_jump"),
            |In((entity,)): In<(Entity,)>, world: &mut World| {
                if let Some(mut entity_ref) = world.get_entity_mut(entity) {
                    entity_ref.insert(JumpQueued(true));
                }
            },
        );
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(TnuaAvian2dPlugin::default())
        .add_plugins(TnuaControllerPlugin::default())
        .add_plugins(PostProcessPlugin)
        .add_plugins(EditorPlugin::default())
        .add_scripting::<RhaiRuntime>(setup_rhai)
        .add_systems(Startup, setup_camera_and_lights)
        .add_systems(Startup, setup_player)
        .add_systems(Startup, setup_level)
        .add_systems(Update, apply_platformer_controls)
        .add_systems(Update, reset_jump_queued.after(apply_platformer_controls))
        .add_systems(Update, call_rhai_on_update_from_rust)
        .run();
}

fn reset_jump_queued(mut query: Query<&mut JumpQueued>) {
    for mut jump_queued in query.iter_mut() {
        jump_queued.0 = false;
    }
}

fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 14.0, 30.0)
                .with_scale((0.05 * Vec2::ONE).extend(1.0))
                .looking_at(Vec3::new(0.0, 14.0, 0.0), Vec3::Y),
            ..Default::default()
        },
        PostProcessSettings {
            pixel_size: 512.,     // Smaller value for less pixelation
            edge_threshold: 0.5,  // Higher value for less pronounced edges
            color_depth: 16.0,    // Higher value for more colors
            effect_strength: 1.0, // Adjust this to blend with the original image
        },
    ));
}

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        avian::RigidBody::Dynamic,
        avian::Collider::capsule(1.0, 2.0),
        TnuaControllerBundle::default(),
        CharacterMotionConfig {
            speed: 30.0,
            acceleration: 50.0,
            jump_height: 12.0,
            float_height: 2.1,
            crouch_float_offset: -0.9,
        },
        TnuaAvian2dSensorShape(avian::Collider::rectangle(1.0, 0.0)),
        SpriteBundle {
            sprite: Sprite {
                color: Color::Srgba(Srgba {
                    red: 0.5,
                    green: 0.5,
                    blue: 1.0,
                    alpha: 1.0,
                }),
                custom_size: Some(Vec2::new(2.0, 4.0)),
                ..default()
            },
            ..default()
        },
        Script::<RhaiScript>::new(asset_server.load("scripts/game_logic.rhai")),
        DesiredVelocity(Vec3::ZERO),
        JumpQueued(false),
        GravityScale(4.0),
        LockedAxes::ROTATION_LOCKED,
    ));
}

fn setup_level(mut commands: Commands) {
    // Ground platform
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::Srgba(Srgba {
                    red: 0.3,
                    green: 0.3,
                    blue: 0.3,
                    alpha: 1.0,
                }),
                custom_size: Some(Vec2::new(20.0, 5.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -2.0, 0.0),
            ..default()
        },
        avian::RigidBody::Static,
        avian::Collider::rectangle(20.0, 5.0),
    ));

    // Floating platform
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::Srgba(Srgba {
                    red: 0.5,
                    green: 0.5,
                    blue: 0.5,
                    alpha: 1.0,
                }),
                custom_size: Some(Vec2::new(8.0, 5.)),
                ..default()
            },
            transform: Transform::from_xyz(10.0, 5.0, 0.0),
            ..default()
        },
        avian::RigidBody::Static,
        avian::Collider::rectangle(8.0, 5.),
    ));

    // Sloped platform
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::Srgba(Srgba {
                    red: 0.4,
                    green: 0.4,
                    blue: 0.4,
                    alpha: 1.0,
                }),
                custom_size: Some(Vec2::new(10.0, 5.)),
                ..default()
            },
            transform: Transform::from_xyz(-10.0, 2.0, 0.0)
                .with_rotation(Quat::from_rotation_z(0.3)),
            ..default()
        },
        avian::RigidBody::Static,
        avian::Collider::rectangle(10.0, 5.),
    ));

    // Small floating platform
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::Srgba(Srgba {
                    red: 0.6,
                    green: 0.6,
                    blue: 0.6,
                    alpha: 1.0,
                }),
                custom_size: Some(Vec2::new(4.0, 5.)),
                ..default()
            },
            transform: Transform::from_xyz(-5.0, 8.0, 0.0),
            ..default()
        },
        avian::RigidBody::Static,
        avian::Collider::rectangle(4.0, 5.),
    ));
}

#[derive(Component)]
struct CharacterMotionConfig {
    speed: f32,
    acceleration: f32,
    jump_height: f32,
    float_height: f32,
    crouch_float_offset: f32,
}

fn call_rhai_on_update_from_rust(
    mut scripted_entities: Query<(Entity, &mut RhaiScriptData)>,
    scripting_runtime: ResMut<RhaiRuntime>,
) {
    for (entity, mut script_data) in &mut scripted_entities {
        scripting_runtime
            .call_fn("update", &mut script_data, entity, ())
            .unwrap();
    }
}

fn apply_platformer_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &mut TnuaController,
        &CharacterMotionConfig,
        &DesiredVelocity,
        &JumpQueued,
    )>,
    time: Res<Time>,
) {
    for (mut controller, config, desired_velocity, jump_queued) in query.iter_mut() {
        let desired_velocity = desired_velocity.0;

        controller.basis(TnuaBuiltinWalk {
            // Move in the direction the player entered, at a speed of 10.0:
            desired_velocity: desired_velocity * config.speed,
            acceleration: config.acceleration,
            air_acceleration: config.acceleration / 2.0,
            free_fall_extra_gravity: 2.0,

            // Turn the character in the movement direction:
            desired_forward: desired_velocity,

            // Must be larger than the height of the entity's center from the bottom of its
            // collider, or else the character will not float and Tnua will not work properly:
            float_height: config.float_height,

            // TnuaBuiltinWalk has many other fields that can be configured:
            ..Default::default()
        });

        if keyboard_input.pressed(KeyCode::Space) {
            controller.action(TnuaBuiltinJump {
                height: config.jump_height,
                ..default()
            });
        }

        if keyboard_input.pressed(KeyCode::KeyS) {
            controller.action(TnuaBuiltinCrouch {
                float_offset: config.crouch_float_offset,
                ..default()
            });
        }
    }
}
