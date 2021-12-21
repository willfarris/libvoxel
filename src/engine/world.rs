use std::collections::{HashMap, LinkedList};

use cgmath::{Matrix4, Vector3, Vector2};
use crate::{c_str, engine::{block::{self, BLOCKS, MeshType}}, graphics::{meshgen, shader::Shader, vertex}};
use crate::graphics::mesh::{Mesh, Texture};

use noise::{Perlin, NoiseFn, Seedable};

#[cfg(target_os = "android")]
extern crate android_log;

pub const CHUNK_SIZE: usize = 16;

#[derive(Debug)]
pub struct Chunk {
    blocks: [[[usize; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    block_mesh: Option<Mesh>,
    model_matrix: Matrix4<f32>,
}

impl Chunk {
    pub fn from_blocks(blocks: [[[usize; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE], position: Vector3<isize>) -> Self {
        Self {
            blocks,
            block_mesh: None,
            model_matrix: Matrix4::from_translation(Vector3::new(position.x as f32, position.y as f32, position.z as f32)),
        }
    }

    pub fn block_at_chunk_pos(&self, chunk_index: &Vector3<usize>) -> usize {
        self.blocks[chunk_index.x][chunk_index.y][chunk_index.z]
    }
}

pub struct World {
    seed: u32,
    pub chunks: HashMap<Vector3<isize>, Chunk>,
    pub generation_queue: HashMap<Vector3<isize>, LinkedList<(Vector3<usize>, usize)>>,
    noise_offset: Vector2<f64>,
    noise_scale: f64,
    perlin: Perlin,
    texture: Texture,
    pub(crate) world_shader: Shader,
}

impl World {
    pub fn new(texture: Texture, world_shader: Shader, seed: u32) -> Self {
        let noise_scale = 0.02;
        let noise_offset = Vector2::new(
            1_000_000.0 * rand::random::<f64>() + 3_141_592.0,
            1_000_000.0 * rand::random::<f64>() + 3_141_592.0,
        );
        let perlin = Perlin::new();
        perlin.set_seed(seed);

        let mut world = Self {
            seed,
            chunks: HashMap::new(),
            generation_queue: HashMap::new(),
            noise_offset,
            noise_scale,
            perlin,
            texture,
            world_shader,
        };
        
        let chunk_radius: isize = 3;
        for chunk_x in -chunk_radius..chunk_radius {
            for chunk_y in 0..chunk_radius {
                for chunk_z in -chunk_radius..chunk_radius {
                    let chunk_index = Vector3::new(chunk_x, chunk_y, chunk_z);
                    let chunk_data: [[[usize; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE] = [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

                    let mut cur_chunk = Chunk::from_blocks(chunk_data, 16 * chunk_index);
                    
                    world.gen_terrain(&chunk_index, &mut cur_chunk);
                    world.gen_caves(&chunk_index, &mut cur_chunk);
                    world.chunks.insert(chunk_index, cur_chunk);
                }
            }
        }

        let chunks = &mut world.chunks;
        world.generation_queue.retain( |key, blocks_queue| {
            if let Some(chunk) = chunks.get_mut(key) {
                for (index, block_id) in blocks_queue {
                    chunk.blocks[index.x][index.y][index.z] = *block_id;
                }
                return false;
            }
            true
        });

        let mut positions = Vec::new();
        for (position, _chunk) in &world.chunks {
            positions.push(position.clone());
        }
        for position in positions {
            world.gen_chunk_mesh(&position);
        }

        world
    }

    fn gen_terrain(&mut self, chunk_index: &Vector3<isize>, chunk: &mut Chunk) {
        //let noise_scale = 0.02;

        //println!("Generating terrain...");

        for block_x in 0..CHUNK_SIZE {
            for block_y in 0..CHUNK_SIZE {
                for block_z in 0..CHUNK_SIZE {
                    let global_x = block_x as isize + (chunk_index.x * CHUNK_SIZE as isize);
                    let global_y = block_y as isize + (chunk_index.y * CHUNK_SIZE as isize);
                    let global_z = block_z as isize + (chunk_index.z * CHUNK_SIZE as isize);
                    let surface_y = self.surface_noise(global_x as f64, global_z as f64);
                    if (global_y as f64) < surface_y {
                        if global_y == surface_y.floor() as isize {
                            chunk.blocks[block_x][block_y][block_z] = 2;
                            self.place_ground_foliage(global_x, global_y + 1, global_z);
                        } else if (global_y as f64) < (7.0 * surface_y/8.0).floor() {
                            match rand::random::<usize>()%100 {
                                0 => chunk.blocks[block_x][block_y][block_z] = 14,
                                1..=3 => chunk.blocks[block_x][block_y][block_z] = 15,
                                _ => chunk.blocks[block_x][block_y][block_z] = 1,
                            }                            
                        } else {
                            chunk.blocks[block_x][block_y][block_z] = 3;
                        }
                    }
                }
            }
        }

        for (position, chunk) in &mut self.chunks {
            if let Some(queue) = self.generation_queue.get(position) {
                for (block_pos, block_id) in queue {
                    chunk.blocks[block_pos.x][block_pos.y][block_pos.z] = *block_id;
                }
            }
        }
    }

    fn place_ground_foliage(&mut self, x: isize, y: isize, z: isize) {
        match rand::random::<usize>()%100 {
            50..=99 => {
                let mut block_id = rand::random::<usize>()%10;
                if block_id <= 6 { block_id = 12 } else if block_id <= 7 {block_id = 13} else if block_id <= 8 {block_id = 7} else {block_id = 10};

                let (position, block_index) = World::chunk_and_block_index(&Vector3::new(x, y, z));
                if let Some(chunk) = self.chunks.get_mut(&position) {
                    chunk.blocks[block_index.x][block_index.y][block_index.z] = block_id;
                } else {
                    self.append_queued_block(block_id, &position, &block_index);
                }
            }
            40 => {
                self.place_tree(Vector3::new(x, y, z))
            }
            _ => {

            }
        }
    }

    fn append_queued_block(&mut self, block_id: usize, chunk_index: &Vector3<isize>, block_index: &Vector3<usize>) {
        if let Some(list) = self.generation_queue.get_mut(chunk_index) {
            list.push_back((*block_index, block_id));
        } else {
            self.generation_queue.insert(*chunk_index, LinkedList::new());
            if let Some(list) = self.generation_queue.get_mut(chunk_index) {
                list.push_back((*block_index, block_id));
            }
        }
    }

    fn gen_caves(&mut self, chunk_index: &Vector3<isize>, chunk: &mut Chunk) {
        let noise_scale = 0.1;
        let cutoff = 0.6;

        //println!("Digging caves...");

        for block_x in 0..CHUNK_SIZE {
            for block_y in 0..CHUNK_SIZE {
                for block_z in 0..CHUNK_SIZE {
                    let global_x = (block_x as isize + (chunk_index.x * CHUNK_SIZE as isize)) as f64;
                    let global_y = (block_y as isize + (chunk_index.y * CHUNK_SIZE as isize)) as f64;
                    let global_z = (block_z as isize + (chunk_index.z * CHUNK_SIZE as isize)) as f64;
                    let noise = self.perlin.get([noise_scale * global_x, noise_scale * global_y, noise_scale * global_z]);
                    if noise > cutoff {
                        chunk.blocks[block_x][block_y][block_z] = 0;
                    }
                }
            }
        }
    }

    pub fn place_tree(&mut self, world_pos: Vector3<isize>) {

        

        for y in 0..5 {
            //chunk.blocks[block_index.x][block_index.y+y][block_index.z] = 9;
            let (chunk_index, block_index) = World::chunk_and_block_index(&(world_pos + Vector3::new(0, y, 0)));
            if let Some(chunk) = self.chunks.get_mut(&chunk_index) {
                chunk.blocks[block_index.x][block_index.y][block_index.z] = 9;
            } else {
                self.append_queued_block(9, &chunk_index, &block_index);
            }
        }

        for x in -1..=1 {
            for z in -1..=1 {
                for y in 3..=5 {
                    if (x == -1 && z == -1 && y == 5) || (x == 1 && z == 1 && y == 5) || (x == -1 && z == 1 && y == 5) || (x == 1 && z == -1 && y == 5) || (x == 0 && z == 0 && y == 3) {
                        continue;
                    }
                    let (chunk_index, block_index) = World::chunk_and_block_index(&(world_pos + Vector3::new(x, y, z)));
                    if let Some(chunk) = self.chunks.get_mut(&chunk_index) {
                        chunk.blocks[block_index.x][block_index.y][block_index.z] = 11;
                    } else {
                        self.append_queued_block(11, &chunk_index, &block_index);
                    }
                    //chunk.blocks[x][y][z] = 11;
                }
            }
        }

        //

        /*if block_index.x == 0 || block_index.x == CHUNK_SIZE-1 || block_index.z == 0 || block_index.z == CHUNK_SIZE-1 || block_index.y > 4 {
            return;
        }
        
        chunk.blocks[block_index.x][block_index.y][block_index.z] = 3;
        for x in block_index.x-1..=block_index.x+1 {
            for z in block_index.z-1..=block_index.z+1 {
                for y in block_index.y+3..block_index.y+6 {
                    chunk.blocks[x][y][z] = 11;
                }
            }
        }
        chunk.blocks[block_index.x-1][block_index.y+5][block_index.z-1] = 0;
        chunk.blocks[block_index.x+1][block_index.y+5][block_index.z+1] = 0;
        chunk.blocks[block_index.x+1][block_index.y+5][block_index.z-1] = 0;
        chunk.blocks[block_index.x-1][block_index.y+5][block_index.z+1] = 0;
        for y in 1..5 {
            chunk.blocks[block_index.x][block_index.y+y][block_index.z] = 9;
        }*/
    }

    pub fn chunk_from_block_array(&mut self, chunk_index: Vector3<isize>, blocks: [[[usize; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]) {
        let new_chunk = Chunk::from_blocks(blocks, 16 * chunk_index);
        self.chunks.insert(chunk_index, new_chunk);
        self.gen_chunk_mesh(&chunk_index);
    }

    fn surface_noise(&self, global_x: f64, global_z: f64) -> f64 {
        5.0 * self.perlin.get([self.noise_scale * global_x + self.noise_offset.x, self.noise_scale * global_z + self.noise_offset.y])
                            //+ (50.0 * self.perlin.get([0.1 * noise_scale * self.noise_offset.x - 100.0, self.noise_offset.y - 44310.0]) + 3.0)
                            + 10.1
    }

    pub fn render_world(&self, _player_position: Vector3<f32>, _player_direction: Vector3<f32>) {
        unsafe {
            for (_position, chunk) in &self.chunks {
                if let Some(m) = &chunk.block_mesh {
                    self.world_shader.set_mat4(c_str!("model_matrix"), &chunk.model_matrix);
                    m.draw(&self.world_shader);
                }
            }
        }
    }

    fn chunk_and_block_index(world_pos: &Vector3<isize>) -> (Vector3<isize>, Vector3<usize>) {
        let chunk_index = Vector3 {
            x: (world_pos.x as f32 / CHUNK_SIZE as f32).floor() as isize,
            y: (world_pos.y as f32 / CHUNK_SIZE as f32).floor() as isize,
            z: (world_pos.z as f32 / CHUNK_SIZE as f32).floor() as isize,
        };
        let block_index = Vector3 {
            x: (world_pos.x.rem_euclid(CHUNK_SIZE as isize)) as usize,
            y: (world_pos.y.rem_euclid(CHUNK_SIZE as isize)) as usize,
            z: (world_pos.z.rem_euclid(CHUNK_SIZE as isize)) as usize,
        };
        (chunk_index, block_index)
    }

    pub fn destroy_at_global_pos(&mut self, world_pos: Vector3<isize>) {
        let (chunk_index, block_index) = World::chunk_and_block_index(&world_pos);
        if let Some(chunk) = self.chunks.get_mut(&chunk_index) {
            chunk.blocks[block_index.x][block_index.y][block_index.z] = 0;
            self.gen_chunk_mesh(&chunk_index);
            
            
            if block_index.x == 0 {
                let adjacent_chunk_index = chunk_index - Vector3::new(1, 0, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            } else if block_index.x == CHUNK_SIZE-1 {
                let adjacent_chunk_index = chunk_index + Vector3::new(1, 0, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            }

            if block_index.y == 0 {
                let adjacent_chunk_index = chunk_index - Vector3::new(0, 1, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            } else if block_index.y == CHUNK_SIZE-1 {
                let adjacent_chunk_index = chunk_index + Vector3::new(0, 1, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            }

            if block_index.z == 0 {
                let adjacent_chunk_index = chunk_index - Vector3::new(0, 0, 1);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            } else if block_index.z == CHUNK_SIZE-1 {
                let adjacent_chunk_index = chunk_index + Vector3::new(0, 0, 1);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            }
            
        }
    }

    pub fn place_at_global_pos(&mut self, world_pos: Vector3<isize>, block_id: usize) {
        let (chunk_index, block_index) = World::chunk_and_block_index(&world_pos);
        if let Some(chunk) = self.chunks.get_mut(&chunk_index) {
            //chunk.destroy_at_chunk_pos(block_index);
            chunk.blocks[block_index.x][block_index.y][block_index.z] = block_id;
            self.gen_chunk_mesh(&chunk_index);
            if block_index.x == 0 {
                let adjacent_chunk_index = chunk_index - Vector3::new(1, 0, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            } else if block_index.x == CHUNK_SIZE-1 {
                let adjacent_chunk_index = chunk_index + Vector3::new(1, 0, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            }

            if block_index.y == 0 {
                let adjacent_chunk_index = chunk_index - Vector3::new(0, 1, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            } else if block_index.y == CHUNK_SIZE-1 {
                let adjacent_chunk_index = chunk_index + Vector3::new(0, 1, 0);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            }

            if block_index.z == 0 {
                let adjacent_chunk_index = chunk_index - Vector3::new(0, 0, 1);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            } else if block_index.z == CHUNK_SIZE-1 {
                let adjacent_chunk_index = chunk_index + Vector3::new(0, 0, 1);
                if let Some(_) = self.chunks.get(&adjacent_chunk_index) {
                    self.gen_chunk_mesh(&adjacent_chunk_index);
                }
            }
        }
    }

    pub fn block_at_global_pos(&self, world_pos: Vector3<isize>) -> usize {
        let (chunk_index, block_index) = World::chunk_and_block_index(&world_pos);
        if let Some(chunk) = self.chunks.get(&chunk_index) {
            chunk.block_at_chunk_pos(&block_index)
        } else {
            0
        }
    }

    pub fn collision_at_world_pos(&self, world_pos: Vector3<isize>) -> bool {
        0 != self.block_at_global_pos(world_pos)
    }

    pub fn gen_chunk_mesh(&mut self, chunk_index: &Vector3<isize>) {
        let mut block_vertices = Vec::new();

        if let Some(current_chunk) = self.chunks.get(chunk_index) {
            for x in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let i = current_chunk.blocks[x][y][z];
                        if i == 0 {
                            continue;
                        }
                        let cur = &crate::engine::block::BLOCKS[i];
                        let tex_coords:[(f32, f32);  6] = if let Some(texture_type) = &cur.texture_map {
                            let mut coords = [(0.0f32, 0.0f32); 6];
                            match texture_type {
                                crate::engine::block::TextureType::Single(x, y) => {
                                    for i in 0..6 {
                                        coords[i] = (*x, *y)
                                    }
                                },
                                crate::engine::block::TextureType::TopAndSide((x_top, y_top), (x_side, y_side)) => {
                                    coords[0] = (*x_side, *y_side);
                                    coords[1] = (*x_side, *y_side);
                                    coords[2] = (*x_top, *y_top);
                                    coords[3] = (*x_side, *y_side);
                                    coords[4] = (*x_side, *y_side);
                                    coords[5] = (*x_side, *y_side);
                                },
                                crate::engine::block::TextureType::TopSideBottom((x_top, y_top), (x_side, y_side), (x_bottom, y_bottom)) => {
                                    coords[0] = (*x_side, *y_side);
                                    coords[1] = (*x_side, *y_side);
                                    coords[2] = (*x_top, *y_top);
                                    coords[3] = (*x_bottom, *y_bottom);
                                    coords[4] = (*x_side, *y_side);
                                    coords[5] = (*x_side, *y_side);
                                },
                            }
                            coords
                        } else {
                            [(0.0, 0.0); 6]
                        };

                        let position = [x as f32, y as f32, z as f32];
                        let vertex_type = cur.block_type as i32;
                        match cur.mesh_type {
                            MeshType::Block => {
                                let x_right_adjacent = if x < 15 {
                                    Some(BLOCKS[current_chunk.block_at_chunk_pos(&Vector3::new(x+1, y, z))])
                                } else if let Some(chunk) = self.chunks.get(&(*chunk_index + Vector3::new(1isize, 0, 0))) {
                                    Some(BLOCKS[chunk.block_at_chunk_pos(&Vector3::new(0, y, z))])
                                } else {
                                    None
                                };
                                if let Some(adjacent_block) = x_right_adjacent {
                                    if adjacent_block.transparent {
                                        meshgen::push_face(&position, 0, &mut block_vertices, &tex_coords[0], vertex_type);
                                    }
                                }

                                let x_left_adjacent = if x > 0 {
                                    Some(BLOCKS[current_chunk.block_at_chunk_pos(&Vector3::new(x-1, y, z))])
                                } else if let Some(chunk) = self.chunks.get(&(*chunk_index + Vector3::new(-1isize, 0, 0))) {
                                    Some(BLOCKS[chunk.block_at_chunk_pos(&Vector3::new(CHUNK_SIZE-1, y, z))])
                                } else {
                                    None
                                };
                                if let Some(adjacent_block) = x_left_adjacent {
                                    if adjacent_block.transparent {
                                        meshgen::push_face(&position, 1, &mut block_vertices, &tex_coords[1], vertex_type);
                                    }
                                }

        
                                let y_top_adjacent = if y < 15 {
                                    Some(BLOCKS[current_chunk.block_at_chunk_pos(&Vector3::new(x, y+1, z))])
                                } else if let Some(chunk) = self.chunks.get(&(*chunk_index + Vector3::new(0, 1isize, 0))) {
                                    Some(BLOCKS[chunk.block_at_chunk_pos(&Vector3::new(x,0, z))])
                                } else {
                                    None
                                };
                                if let Some(adjacent_block) = y_top_adjacent {
                                    if adjacent_block.transparent {
                                        meshgen::push_face(&position, 2, &mut block_vertices, &tex_coords[2], vertex_type);
                                    }
                                }
        
                                let y_bottom_adjacent = if y > 0 {
                                    Some(BLOCKS[current_chunk.block_at_chunk_pos(&Vector3::new(x, y-1, z))])
                                } else if let Some(chunk) = self.chunks.get(&(*chunk_index + Vector3::new(0, -1isize, 0))) {
                                    Some(BLOCKS[chunk.block_at_chunk_pos(&Vector3::new(x,CHUNK_SIZE-1, z))])
                                } else {
                                    None
                                };
                                if let Some(adjacent_block) = y_bottom_adjacent {
                                    if adjacent_block.transparent {
                                        meshgen::push_face(&position, 3, &mut block_vertices, &tex_coords[3], vertex_type);
                                    }
                                }

                                let z_back_adjacent = if z < 15 {
                                    Some(BLOCKS[current_chunk.block_at_chunk_pos(&Vector3::new(x, y, z+1))])
                                } else if let Some(chunk) = self.chunks.get(&(*chunk_index + Vector3::new(0, 0, 1isize))) {
                                    Some(BLOCKS[chunk.block_at_chunk_pos(&Vector3::new(x, y, 0))])
                                } else {
                                    None
                                };
                                if let Some(adjacent_block) = z_back_adjacent {
                                    if adjacent_block.transparent {
                                        meshgen::push_face(&position, 4, &mut block_vertices, &tex_coords[4], vertex_type);
                                    }
                                }


                                let z_front_adjacent = if z > 0 {
                                    Some(BLOCKS[current_chunk.block_at_chunk_pos(&Vector3::new(x, y, z-1))])
                                } else if let Some(chunk) = self.chunks.get(&(*chunk_index + Vector3::new(0, 0, -1isize))) {
                                    Some(BLOCKS[chunk.block_at_chunk_pos(&Vector3::new(x, y, CHUNK_SIZE-1))])
                                } else {
                                    None
                                };
                                if let Some(adjacent_block) = z_front_adjacent {
                                    if adjacent_block.transparent {
                                        meshgen::push_face(&position, 5, &mut block_vertices, &tex_coords[5], vertex_type);
                                    }
                                }
                            }
                            MeshType::CrossedPlanes => {
                                meshgen::push_face(&position, 6, &mut block_vertices, &tex_coords[0], vertex_type);
                                meshgen::push_face(&position, 7, &mut block_vertices, &tex_coords[0], vertex_type);
                                meshgen::push_face(&position, 8, &mut block_vertices, &tex_coords[0], vertex_type);
                                meshgen::push_face(&position, 9, &mut block_vertices, &tex_coords[0], vertex_type);
                            }
                        }
                        
                    }
                }
            }
            //let mesh = Mesh::new(block_vertices, &self.texture, &self.world_shader);
            //current_chunk.block_mesh = Some(mesh); 
        } else {
            return;
        }

        if !block_vertices.is_empty() {
            if let Some(chunk) = self.chunks.get_mut(chunk_index) {
                let block_mesh = Mesh::new(block_vertices, &self.texture, &self.world_shader);
                chunk.block_mesh = Some(block_mesh);
            }
        }
    }
}
