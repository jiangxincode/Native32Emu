// Sprite system: manages movie instances including position, visibility,
// frame advancement, cloning, and removal.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MovieState {
    pub movie: u32,
    pub x: i16,
    pub y: i16,
    pub depth: u16,
    pub frame: usize,
    pub visible: bool,
    pub playing: bool,
    pub cloned: bool,
    pub sound_channel: Option<usize>,
    pub next_frame: Option<isize>,
}

impl MovieState {
    pub fn new(movie: u32, x: i16, y: i16, depth: u16) -> Self {
        Self {
            movie,
            x,
            y,
            depth,
            frame: 0,
            visible: true,
            playing: true,
            cloned: false,
            sound_channel: None,
            next_frame: Some(0),
        }
    }
}

pub type SpriteMap = HashMap<String, MovieState>;

pub struct SpriteSystem {
    pub sprites: SpriteMap,
}

impl Default for SpriteSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteSystem {
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
        }
    }

    /// Update movie instances for a newly loaded frame.
    /// - Add new named movies that appear in the frame
    /// - Remove non-cloned movies that are no longer in the frame
    pub fn update_for_frame(&mut self, frame_objects: &[crate::file_loader::FrameObject]) {
        let mut frame_movie_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for obj in frame_objects {
            if obj.obj_type == crate::file_loader::ObjectType::Movie {
                if let Some(ref name) = obj.name {
                    frame_movie_names.insert(name.clone());
                    if !self.sprites.contains_key(name) {
                        // Create new movie instance
                        let mut state = MovieState::new(obj.index as u32, obj.x, obj.y, obj.depth);
                        state.next_frame = Some(0);
                        self.sprites.insert(name.clone(), state);
                    }
                    // If already exists, preserve its state (don't reset)
                }
            }
        }

        // Remove non-cloned movies not in the new frame
        let to_remove: Vec<String> = self
            .sprites
            .iter()
            .filter(|(name, movie)| !movie.cloned && !frame_movie_names.contains(*name))
            .map(|(name, _)| name.clone())
            .collect();

        for name in to_remove {
            self.sprites.remove(&name);
        }
    }

    /// Advance movie frames. Movies run at half the main framerate.
    pub fn tick(&mut self, tick_count: u64) {
        for (_, movie) in self.sprites.iter_mut() {
            if !movie.playing || movie.next_frame.is_some() {
                continue;
            }
            if movie.sound_channel.is_some() {
                continue;
            }
            // Half framerate: advance every other tick
            if tick_count.is_multiple_of(2) {
                movie.next_frame = Some(movie.frame as isize + 1);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &MovieState)> {
        self.sprites.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut MovieState)> {
        self.sprites.iter_mut()
    }

    pub fn get(&self, name: &str) -> Option<&MovieState> {
        self.sprites.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut MovieState> {
        self.sprites.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<MovieState> {
        self.sprites.remove(name)
    }

    pub fn insert(&mut self, name: String, state: MovieState) {
        self.sprites.insert(name, state);
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.sprites.contains_key(name)
    }
}
