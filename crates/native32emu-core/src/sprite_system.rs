// Sprite system: manages movie instances including position, visibility,
// frame advancement, cloning, and removal.

use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
                    if self.sprites.contains_key(name) {
                        frame_movie_names.insert(name.clone());
                    } else {
                        let renamed_instance = self
                            .sprites
                            .iter()
                            .find(|(_, movie)| {
                                !movie.cloned
                                    && movie.movie == obj.index as u32
                                    && movie.depth == obj.depth
                            })
                            .map(|(existing_name, _)| existing_name.clone());

                        if let Some(existing_name) = renamed_instance {
                            frame_movie_names.insert(existing_name);
                        } else {
                            // Create new movie instance.
                            let mut state =
                                MovieState::new(obj.index as u32, obj.x, obj.y, obj.depth);
                            state.next_frame = Some(0);
                            self.sprites.insert(name.clone(), state);
                            frame_movie_names.insert(name.clone());
                        }
                    }
                    // If already exists, preserve its state (do not reset).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_loader::{FrameObject, ObjectType};

    fn make_movie_object(name: &str, index: u16, x: i16, y: i16, depth: u16) -> FrameObject {
        FrameObject {
            obj_type: ObjectType::Movie,
            index,
            x,
            y,
            depth,
            name: Some(name.to_string()),
        }
    }

    #[test]
    fn test_new_system_is_empty() {
        let ss = SpriteSystem::new();
        assert_eq!(ss.sprites.len(), 0);
    }

    #[test]
    fn test_default_trait() {
        let ss = SpriteSystem::default();
        assert_eq!(ss.sprites.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut ss = SpriteSystem::new();
        let state = MovieState::new(1, 10, 20, 5);
        ss.insert("hero".to_string(), state);

        let movie = ss.get("hero").unwrap();
        assert_eq!(movie.movie, 1);
        assert_eq!(movie.x, 10);
        assert_eq!(movie.y, 20);
        assert_eq!(movie.depth, 5);
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let ss = SpriteSystem::new();
        assert!(ss.get("missing").is_none());
    }

    #[test]
    fn test_contains_key() {
        let mut ss = SpriteSystem::new();
        assert!(!ss.contains_key("a"));
        ss.insert("a".to_string(), MovieState::new(1, 0, 0, 0));
        assert!(ss.contains_key("a"));
    }

    #[test]
    fn test_remove() {
        let mut ss = SpriteSystem::new();
        ss.insert("a".to_string(), MovieState::new(1, 0, 0, 0));
        let removed = ss.remove("a");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().movie, 1);
        assert!(!ss.contains_key("a"));
    }

    #[test]
    fn test_remove_nonexistent_returns_none() {
        let mut ss = SpriteSystem::new();
        assert!(ss.remove("missing").is_none());
    }

    #[test]
    fn test_update_for_frame_adds_new_movies() {
        let mut ss = SpriteSystem::new();
        let objects = vec![make_movie_object("hero", 1, 10, 20, 0)];
        ss.update_for_frame(&objects);

        assert!(ss.contains_key("hero"));
        let movie = ss.get("hero").unwrap();
        assert_eq!(movie.movie, 1);
        assert_eq!(movie.x, 10);
        assert_eq!(movie.y, 20);
        assert!(movie.visible);
        assert!(movie.playing);
        assert!(!movie.cloned);
        assert_eq!(movie.next_frame, Some(0));
    }

    #[test]
    fn test_update_for_frame_preserves_existing_state() {
        let mut ss = SpriteSystem::new();
        let mut state = MovieState::new(1, 0, 0, 0);
        state.frame = 42;
        state.visible = false;
        ss.insert("hero".to_string(), state);

        // Same movie in new frame
        let objects = vec![make_movie_object("hero", 1, 10, 20, 0)];
        ss.update_for_frame(&objects);

        let movie = ss.get("hero").unwrap();
        assert_eq!(movie.frame, 42, "frame should be preserved");
        assert!(!movie.visible, "visibility should be preserved");
    }

    #[test]
    fn test_update_for_frame_preserves_renamed_instances_by_identity() {
        let mut ss = SpriteSystem::new();
        let objects = vec![make_movie_object("q0n0", 7, 10, 20, 3)];
        ss.update_for_frame(&objects);

        let mut renamed = ss.remove("q0n0").unwrap();
        renamed.y = -124;
        ss.insert("qn0".to_string(), renamed);

        ss.update_for_frame(&objects);

        assert!(ss.contains_key("qn0"));
        assert!(!ss.contains_key("q0n0"));
        assert_eq!(ss.get("qn0").unwrap().y, -124);
    }

    #[test]
    fn test_update_for_frame_removes_non_cloned_absent_movies() {
        let mut ss = SpriteSystem::new();
        ss.insert("old".to_string(), MovieState::new(1, 0, 0, 0));

        // Empty frame - "old" should be removed
        ss.update_for_frame(&[]);
        assert!(!ss.contains_key("old"));
    }

    #[test]
    fn test_update_for_frame_keeps_cloned_movies() {
        let mut ss = SpriteSystem::new();
        let mut state = MovieState::new(1, 0, 0, 0);
        state.cloned = true;
        ss.insert("clone".to_string(), state);

        ss.update_for_frame(&[]);
        assert!(ss.contains_key("clone"), "cloned movie should survive");
    }

    #[test]
    fn test_update_for_frame_ignores_non_movie_objects() {
        let mut ss = SpriteSystem::new();
        let objects = vec![FrameObject {
            obj_type: ObjectType::Image,
            index: 1,
            x: 0,
            y: 0,
            depth: 0,
            name: Some("not_a_movie".to_string()),
        }];
        ss.update_for_frame(&objects);
        assert!(!ss.contains_key("not_a_movie"));
    }

    #[test]
    fn test_update_for_frame_multiple_movies() {
        let mut ss = SpriteSystem::new();
        let objects = vec![
            make_movie_object("a", 1, 0, 0, 0),
            make_movie_object("b", 2, 10, 20, 1),
            make_movie_object("c", 3, 30, 40, 2),
        ];
        ss.update_for_frame(&objects);
        assert_eq!(ss.sprites.len(), 3);
    }

    #[test]
    fn test_movie_state_new_defaults() {
        let ms = MovieState::new(5, 100, 200, 10);
        assert_eq!(ms.movie, 5);
        assert_eq!(ms.x, 100);
        assert_eq!(ms.y, 200);
        assert_eq!(ms.depth, 10);
        assert_eq!(ms.frame, 0);
        assert!(ms.visible);
        assert!(ms.playing);
        assert!(!ms.cloned);
        assert!(ms.sound_channel.is_none());
        assert_eq!(ms.next_frame, Some(0));
    }

    #[test]
    fn test_iter_and_iter_mut() {
        let mut ss = SpriteSystem::new();
        ss.insert("a".to_string(), MovieState::new(1, 0, 0, 0));
        ss.insert("b".to_string(), MovieState::new(2, 0, 0, 0));

        let names: Vec<&String> = ss.iter().map(|(name, _)| name).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&"a".to_string()));
        assert!(names.contains(&&"b".to_string()));

        // Mutate via iter_mut
        for (_, movie) in ss.iter_mut() {
            movie.frame = 99;
        }
        assert_eq!(ss.get("a").unwrap().frame, 99);
        assert_eq!(ss.get("b").unwrap().frame, 99);
    }

    #[test]
    fn test_get_mut() {
        let mut ss = SpriteSystem::new();
        ss.insert("hero".to_string(), MovieState::new(1, 0, 0, 0));

        if let Some(movie) = ss.get_mut("hero") {
            movie.x = 42;
            movie.y = 84;
        }
        assert_eq!(ss.get("hero").unwrap().x, 42);
        assert_eq!(ss.get("hero").unwrap().y, 84);
    }

    #[test]
    fn test_update_removes_old_and_adds_new() {
        let mut ss = SpriteSystem::new();
        ss.insert("old".to_string(), MovieState::new(1, 0, 0, 0));

        let objects = vec![make_movie_object("new", 2, 0, 0, 0)];
        ss.update_for_frame(&objects);

        assert!(!ss.contains_key("old"));
        assert!(ss.contains_key("new"));
    }

    #[test]
    fn test_movie_without_name_is_skipped() {
        let mut ss = SpriteSystem::new();
        let objects = vec![FrameObject {
            obj_type: ObjectType::Movie,
            index: 1,
            x: 0,
            y: 0,
            depth: 0,
            name: None, // no name
        }];
        ss.update_for_frame(&objects);
        assert_eq!(ss.sprites.len(), 0);
    }
}
