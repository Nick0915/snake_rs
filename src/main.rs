use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use rand::prelude::random;
use bevy::core::FixedTimestep;

use std::fmt;

// game vars
const WIDTH: u8 = 15;
const HEIGHT: u8 = 15;

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
struct Position {
    x: i8,
    y: i8,
}
impl fmt::Debug for Position {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Position")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

struct Size {
    width: f32,
    height: f32,
}
impl Size {
    // creates and returns a new Size w/ both values as x
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    UP, DOWN, LEFT, RIGHT
}
impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::LEFT => Self::RIGHT,
            Self::RIGHT => Self::LEFT,
            Self::UP => Self::DOWN,
            Self::DOWN => Self::UP,
        }
    }
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SnakeState {
    Input,
    Movement,
    Eating,
    Growth,
}

// custom components as structs
struct Food;
struct SnakeHead {
    direction: Direction
}
struct SnakeSegment;
struct QueuedDirection {
    direction: Direction
}

#[derive(Default)]
struct SnakeSegments(Vec<Entity>); // list of snake parts

#[derive(Default)]
struct LastTailPosition(Option<Position>);

#[derive(Default, PartialEq, Eq)]
struct OccupiedPositions(Vec<Position>); // [tail, ..., head]

// events
struct SpawnFoodEvent;
struct GrowthEvent;
struct GameOverEvent;

// just a concrete list of all materials we need
struct Materials {
    head_material: Handle<ColorMaterial>,
    tail_material: Handle<ColorMaterial>,
    food_material: Handle<ColorMaterial>,
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            width: 500.,
            height: 500.,
            ..Default::default()
        })
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.25)))
        .insert_resource(OccupiedPositions(Vec::new()))
        .add_startup_system(setup.system())
        .add_startup_stage("game_setup", SystemStage::single(spawn_snake.system()))
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .add_event::<SpawnFoodEvent>()
        .add_system(spawn_food.system().before(SnakeState::Input))
        // ! TEMPORARY
        // .add_system(
        //     spawn_food
        //         .system()
        //         .with_run_criteria(FixedTimestep::step(0.5))
        // )
        .add_system(
            snake_input
                .system()
                .label(SnakeState::Input)
                .before(SnakeState::Movement) // ensures ::Input happens before ::Movement
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.200))
                .with_system(snake_movement.system().label(SnakeState::Movement))
                .with_system(
                    eat_food
                        .system()
                        .label(SnakeState::Eating)
                        .after(SnakeState::Movement)
                )
                .with_system(
                    snake_growth
                        .system()
                        .label(SnakeState::Growth)
                        .after(SnakeState::Eating)
                )
        )
        .add_system(game_over.system().after(SnakeState::Movement))
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation.system())
                .with_system(size_scaling.system())
        )
        .add_plugins(DefaultPlugins)
        .run();
}

// system ran at startup
fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    // spawn new component: an orthographic 2d camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // create a new material
    commands.insert_resource(Materials {
        head_material: materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
        tail_material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
        food_material: materials.add(Color::rgb(0.7, 0., 0.).into()),
    });
}

// system to spawn a snake head
fn spawn_snake(
    mut commands: Commands,
    materials: Res<Materials>,
    mut segments: ResMut<SnakeSegments>,
    mut occupied: ResMut<OccupiedPositions>,
    mut food_writer: EventWriter<SpawnFoodEvent>
) {
    segments.0 = vec![
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.head_material.clone(),
                sprite: Sprite::new(Vec2::new(10.0, 10.0)),
                ..Default::default()
            })
            .insert(SnakeHead {
                direction: Direction::UP
            }) // add SnakeHead component
            .insert(Position { x: 0, y : 1}) // add Position component
            .insert(Size::square(0.8)) // add Size component of { .8, .8 }
            .insert(QueuedDirection{ direction: Direction::UP })
            .id(),
        spawn_segment(
            commands, materials, Position{ x: 0, y: 0 }
        )
    ];

    occupied.0.push(Position{ x: 0, y: 1 });
    occupied.0.push(Position{ x: 0, y: 0 });

    food_writer.send(SpawnFoodEvent);
}

// system to spawn food
fn spawn_food(
    mut commands: Commands,
    materials: Res<Materials>,
    occupied: Res<OccupiedPositions>,
    mut food_reader: EventReader<SpawnFoodEvent>,
    mut game_over_writer: EventWriter<GameOverEvent>
) {
    if food_reader.iter().next().is_some() {
        // println!("{:?}", occupied.0);
        let mut spawn_found = false;
        // only for debugging
        // let mut tried_positions = Vec::new();

        let mut rand_x = (random::<f32>() * WIDTH as f32) as i8;
        let mut rand_y = (random::<f32>() * HEIGHT as f32) as i8;
        // calculate new positions until unoccupied one is found
        for _ in 0..(WIDTH as i16 * HEIGHT as i16 + 1) {
            if occupied.0.iter().any(|&i| i == Position{ x: rand_x, y: rand_y }) {
                // if position doesn't work, add it to attempts
                // tried_positions.push(Position{ x: rand_x, y: rand_y });

                let mut new_rand_x = (random::<f32>() * WIDTH as f32) as i8;
                let mut new_rand_y = (random::<f32>() * HEIGHT as f32) as i8;
                while new_rand_x == rand_x && new_rand_y == rand_y {
                    new_rand_x = (random::<f32>() * WIDTH as f32) as i8;
                    new_rand_y = (random::<f32>() * HEIGHT as f32) as i8;
                }

                rand_x = new_rand_x;
                rand_y = new_rand_y;
            // if position hasn't been attempted
            } else {
                spawn_found = true;
                // println!("spot found: {:?}", Position{ x: rand_x, y: rand_y });
                break;
            }
        }

        // if food can't spawn anywhere, game over
        if !spawn_found {
            println!("!YOU WIN!");
            // println!("tried positions: {:?}", tried_positions);
            game_over_writer.send(GameOverEvent);
            return;
        }

        commands.spawn_bundle(SpriteBundle {
            material: materials.food_material.clone(),
            sprite: Sprite::new(Vec2::new(10., 10.)),
            ..Default::default()
        })
        .insert(Food)
        .insert(Position {
            x: rand_x,
            y: rand_y,
        })
        .insert(Size::square(0.65));
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut sprite) in q.iter_mut() {
        sprite.size = Vec2::new(
            sprite_size.width / (WIDTH as f32) * (window.width() as f32),
            sprite_size.height / (HEIGHT as f32) * (window.height() as f32),
        );
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, HEIGHT as f32),
            0.,
        );
    }
}

fn snake_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut heads: Query<(&SnakeHead, &mut QueuedDirection)>
) {
    if let Some((head, mut queued)) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::Left) {
            Direction::LEFT
        } else if keyboard_input.pressed(KeyCode::Down) {
            Direction::DOWN
        } else if keyboard_input.pressed(KeyCode::Up) {
            Direction::UP
        } else if keyboard_input.pressed(KeyCode::Right) {
            Direction::RIGHT
        } else {
            queued.direction // defaults to previously queued input
        };

        if dir != head.direction.opposite() {
            queued.direction = dir;
        }
    }
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut heads: Query<(Entity, &mut SnakeHead, &QueuedDirection)>,
    mut positions: Query<&mut Position>,
    mut occupied: ResMut<OccupiedPositions>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_writer: EventWriter<GameOverEvent>
) {
    // println!("{:?}", occupied.0);
    if let Some((head_entity, mut head, queued)) = heads.iter_mut().next() {
        head.direction = queued.direction;
        let segment_positions = segments.0.iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = positions.get_mut(head_entity).unwrap();
        match &head.direction {
            Direction::LEFT => {
                head_pos.x -= 1;
            }
            Direction::RIGHT => {
                head_pos.x += 1;
            }
            Direction::UP => {
                head_pos.y += 1;
            }
            Direction::DOWN => {
                head_pos.y -= 1;
            }
        };
        if head_pos.x < 0
            || head_pos.y < 0
            || head_pos.x as u8 >= WIDTH
            || head_pos.y as u8 >= HEIGHT
        {
            game_over_writer.send(GameOverEvent);
        }
        occupied.0.push(Position{ x: head_pos.x, y: head_pos.y }); // add new head
        segment_positions
        .iter()
        .zip(segments.0.iter().skip(1))
        .for_each(|(pos, segment)| {
            *positions.get_mut(*segment).unwrap() = *pos;
        });
        last_tail_position.0 = Some(*segment_positions.last().unwrap());

        // check collision after movement
        let head_pos_2 = positions.get_mut(head_entity).unwrap();

        if segment_positions.contains(&head_pos_2) {
            game_over_writer.send(GameOverEvent);
        }

        occupied.0.remove(0); // remove old tail
    }
}

fn eat_food(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    mut food_writer: EventWriter<SpawnFoodEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
                food_writer.send(SpawnFoodEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
    mut occupied: ResMut<OccupiedPositions>,
    materials: Res<Materials>
) {
    if growth_reader.iter().next().is_some() {
        segments.0.push(spawn_segment(
            commands,
            materials,
            last_tail_position.0.unwrap()
        ));
        occupied.0.insert(0, last_tail_position.0.unwrap());
    }
}

fn spawn_segment(
    mut commands: Commands,
    materials: Res<Materials>,
    position: Position,
) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.tail_material.clone(),
            ..Default::default()
        })
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(0.65))
        .id()
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    food_writer: EventWriter<SpawnFoodEvent>,
    materials: Res<Materials>,
    segments_res: ResMut<SnakeSegments>,
    mut occupied: ResMut<OccupiedPositions>,
    entities: Query<Entity, With<Size>>
) {
    if reader.iter().next().is_some() {
        for ent in entities.iter() {
            commands.entity(ent).despawn();
        }
        occupied.0.truncate(0);
        spawn_snake(commands, materials, segments_res, occupied, food_writer);
    }
}