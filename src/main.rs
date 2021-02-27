use std::collections::HashSet;

use bevy::prelude::*;
use bevy::render::camera::OrthographicProjection;
use bevy::render::pass::ClearColor;
use bevy_rapier2d::physics::{
    JointBuilderComponent, RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent,
};
use bevy_rapier2d::rapier::dynamics::{BallJoint, RigidBody, RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use bevy_rapier2d::rapier::na::Vector2;
use nalgebra::Point2;
use rand::{random, Rng};

// fn vain() {
//     let mut app = App::build();
//     app.add_resource(Msaa { samples: 4 })
//         .add_plugins(DefaultPlugins);
//     #[cfg(target_arch = "wasm32")]
//     app.add_plugin(bevy_webgl2::WebGL2Plugin);
//     app.add_startup_system(setup.system()).run();
// }

fn main() {
    // Set up Bevy
    let mut app = App::build();
    app.init_resource::<Game>()
        .add_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_game.system())
        .add_startup_system(setup_board.system())
        .add_startup_system(setup_initial_tetromino.system())
        .add_system(tetromino_movement.system())
        .add_system(tetromino_sleep_detection.system())
        .add_plugin(RapierPhysicsPlugin);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.run();
}

//
// Note on coordinate systems used
// The game uses different coordinate systems.
// What they have in common, is that the Y axis always points _upwards_.
//
// 1. Tetromino coordinate system (discrete, IVector)
//    The "middle block" in the tetromino is (0, 0)

// 2. Board coordinate system (discrete, IVector)
//    The left-most, bottom block position is the origin,
//    so we can refer to row 0, 1, 2, column 0, 1, 2 etc
//
// 3. Physics coordinate system
//    The physics coordinate system is expressed in block size units:
//    A tetris block has a size of of (1.0, 1.0).
//    The origin of this coordinate system is the _center of the board_.
//
//    A cuboid collider has its center at its rigid body's position.
//    Therefore the position of a block lying flat on the board floor (in row 0)
//    is floor_y + 0.5.
//
// 4. Screen coordinate system
//    Pixels on the screen!
//    This is the Physics coordinate system scaled up by BLOCK_PX_SIZE.
//    So the center of the board is also the center of the screen.
//
// It is not recommended to put large numbers into the physics engine,
// because of floating point precision loss.
// Therefore the physics coordinate system is kept at a much smaller scale than
// screen coordinates.
//

const BLOCK_PX_SIZE: f32 = 30.0;

// In terms of block size:
const FLOOR_BLOCK_HEIGHT: f32 = 2.0;

const BLOCK_MASS: f32 = 1.0;
const BLOCK_LINEAR_DAMPING: f32 = 1.0;

const MOVEMENT_FORCE: f32 = 20.0;
const TORQUE: f32 = 20.0;

const N_LANES: usize = 10;
const N_ROWS: usize = 20;

/// Type for our discrete coordinate systems
/// (column, row) or (x, y)
type IVector = (i32, i32);

/// This struct is used as a Bevy resource: Res<Game>
struct Game {
    n_lanes: usize,
    n_rows: usize,
    block_color: Option<Handle<ColorMaterial>>,
    current_tetromino_blocks: HashSet<Entity>,
    current_tetromino_joints: Vec<Entity>,
    camera: Option<Entity>,
}

impl Game {
    ///
    /// The y position of the floor, in physics coordinates
    ///
    fn floor_y(&self) -> f32 {
        -(self.n_rows as f32) * 0.5
    }

    ///
    /// The x position of the left edge of the board, in physicss coordinates
    ///
    fn left_edge_x(&self) -> f32 {
        -(self.n_lanes as f32) * 0.5
    }

    ///
    /// Translate the board coordinate to the center of the board, topmost row,
    /// where tetrominos should spawn!
    ///
    fn translate_to_board_center_top(&self, (col, row): IVector) -> IVector {
        let x = col + self.n_lanes as i32 / 2;
        let y = row + self.n_rows as i32;
        (x, y)
    }

    ///
    /// Translate from board coordinates to physics coordinates.
    ///
    fn board_to_physics(&self, (col, row): IVector) -> (f32, f32) {
        let x = self.left_edge_x() + col as f32 + 0.5;
        let y = self.floor_y() + row as f32 + 0.5;

        (x, y)
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            n_lanes: N_LANES,
            n_rows: N_ROWS,
            block_color: None,
            current_tetromino_blocks: HashSet::new(),
            current_tetromino_joints: vec![],
            camera: None,
        }
    }
}

fn byte_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn setup_game(
    commands: &mut Commands,
    mut game: ResMut<Game>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    rapier_config.scale = BLOCK_PX_SIZE;

    game.block_color = Some(materials.add(random_color().into()));
    game.camera = commands.spawn(Camera2dBundle::default()).current_entity();
}

/// Represent Tetris' different tetromino kinds
#[derive(Clone, Copy, Debug)]
enum TetrominoKind {
    I,
    O,
    T,
    J,
    L,
    S,
    Z,
}

impl TetrominoKind {
    fn random() -> Self {
        match rand::thread_rng().gen_range(0..=6) {
            0 => Self::I,
            1 => Self::O,
            2 => Self::T,
            3 => Self::J,
            4 => Self::L,
            5 => Self::S,
            _ => Self::Z,
        }
    }

    fn layout(&self) -> TetrominoLayout {
        match self {
            Self::I => TetrominoLayout {
                coords: [(1, 1), (1, 0), (1, -1), (1, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::O => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (0, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3), (1, 0)],
            },
            Self::T => TetrominoLayout {
                coords: [(0, 0), (1, 0), (2, 0), (1, -1)],
                joints: vec![(0, 1), (1, 2), (1, 3)],
            },
            Self::J => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (0, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::L => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (2, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::S => TetrominoLayout {
                coords: [(0, -1), (1, -1), (1, 0), (2, 0)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::Z => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (2, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
        }
    }
}

/// The layout of one tetromino
struct TetrominoLayout {
    /// All tetrominos consist of 4 blocks, so we use a fixed-size array.
    /// This is expressed in the tetromino coordinate system
    coords: [IVector; 4],
    /// OTOH, The number of _joints_ is variable..
    joints: Vec<(usize, usize)>,
}

struct Block;

// startup system
fn setup_board(
    commands: &mut Commands,
    game: Res<Game>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let floor_y = game.floor_y();

    // Add a "floor" - something blocks collide with when they hit the bottom of the board.
    // The floor is a *static* rigid body. It has infinite mass, and should
    // not be influenced by any forces.
    commands
        .spawn(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
            sprite: Sprite::new(Vec2::new(
                game.n_lanes as f32 * BLOCK_PX_SIZE,
                FLOOR_BLOCK_HEIGHT * BLOCK_PX_SIZE,
            )),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_static().translation(0.0, floor_y - (FLOOR_BLOCK_HEIGHT * 0.5)))
        .with(ColliderBuilder::cuboid(
            game.n_lanes as f32 * 0.5,
            FLOOR_BLOCK_HEIGHT * 0.5,
        ));
}

// startup system
fn setup_initial_tetromino(commands: &mut Commands, mut game: ResMut<Game>) {
    spawn_tetromino(commands, &mut game);
}

fn spawn_tetromino(commands: &mut Commands, game: &mut Game) {
    let kind = TetrominoKind::random();
    let TetrominoLayout { coords, joints } = kind.layout();

    let block_entities: Vec<Entity> = coords
        .iter()
        .map(|_| spawn_block(commands, game, kind, coords[0]))
        .collect();

    let joint_entities: Vec<Entity> = joints
        .iter()
        .map(|(i, j)| {
            let x_dir = coords[*j].0 as f32 - coords[*i].0 as f32;
            let y_dir = coords[*j].1 as f32 - coords[*i].1 as f32;

            let anchor_1 = Point2::new(x_dir * 0.5, y_dir * 0.5);
            let anchor_2 = Point2::new(x_dir * -0.5, y_dir * -0.5);

            commands
                .spawn((JointBuilderComponent::new(
                    BallJoint::new(anchor_1, anchor_2),
                    block_entities[*i],
                    block_entities[*j],
                ),))
                .current_entity()
                .unwrap()
        })
        .collect();

    game.current_tetromino_blocks = block_entities.into_iter().collect();
    game.current_tetromino_joints = joint_entities.into_iter().collect();
}

fn spawn_block(
    commands: &mut Commands,
    game: &Game,
    kind: TetrominoKind,
    tetromino_coord: IVector,
) -> Entity {
    let (x, y) = game.board_to_physics(game.translate_to_board_center_top(tetromino_coord));

    println!("block physics coords: {}, {}", x, y);

    let rigid_body = RigidBodyBuilder::new_dynamic()
        .translation(x, y)
        .mass(BLOCK_MASS)
        .linear_damping(BLOCK_LINEAR_DAMPING);
    let collider = ColliderBuilder::cuboid(0.5, 0.5).density(1.0);

    commands
        .spawn(SpriteBundle {
            material: game.block_color.clone().unwrap(),
            sprite: Sprite::new(Vec2::new(BLOCK_PX_SIZE, BLOCK_PX_SIZE)),
            ..Default::default()
        })
        .with(rigid_body)
        .with(collider)
        .with(Block)
        .current_entity()
        .unwrap()
}

// system
fn tetromino_movement(
    input: Res<Input<KeyCode>>,
    game: Res<Game>,
    rigid_body_query: Query<&RigidBodyHandleComponent>,
    mut rigid_bodies: ResMut<RigidBodySet>,
) {
    let movement = input.pressed(KeyCode::Right) as i8 - input.pressed(KeyCode::Left) as i8;

    for entity in game.current_tetromino_blocks.iter() {
        if let Ok(handle_component) = rigid_body_query.get(*entity) {
            if let Some(rigid_body) = rigid_bodies.get_mut(handle_component.handle()) {
                if movement != 0 {
                    rigid_body
                        .apply_force(Vector2::new(movement as f32 * MOVEMENT_FORCE, 0.0), true)
                }
            }
        }
    }
}

// system
fn tetromino_sleep_detection(
    commands: &mut Commands,
    mut game: ResMut<Game>,
    block_query: Query<(Entity, &RigidBodyHandleComponent)>,
    rigid_bodies: ResMut<RigidBodySet>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let all_blocks_sleeping = game.current_tetromino_blocks.iter().all(|block_entity| {
        block_query
            .get(*block_entity)
            .ok()
            .and_then(|(_, rigid_body_component)| rigid_bodies.get(rigid_body_component.handle()))
            .map(RigidBody::is_sleeping)
            .unwrap_or(false)
    });

    if all_blocks_sleeping {
        for joint in &game.current_tetromino_joints {
            commands.despawn(*joint);
        }

        game.block_color = Some(materials.add(random_color().into()));
        spawn_tetromino(commands, &mut game);
    }
}

fn random_color() -> Color {
    let r = rand::thread_rng().gen_range(0..255);
    let g = rand::thread_rng().gen_range(0..255);
    let b = rand::thread_rng().gen_range(0..255);
    byte_rgb(r, g, b)
}
