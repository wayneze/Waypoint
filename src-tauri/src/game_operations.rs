//! One filesystem operation owns a game and its shared installation directories.
use std::{collections::HashSet, path::Path, sync::{Mutex, OnceLock}};
use crate::records::GameRecord;

static OPERATIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
pub struct GameOperation(Vec<String>);
fn conflicts(a: &str, b: &str) -> bool {
    a == b || (a.starts_with("path:") && b.starts_with("path:") && (a.starts_with(&format!("{b}\\")) || b.starts_with(&format!("{a}\\"))))
}
impl GameOperation {
    pub fn start(record: &GameRecord) -> Result<Self, String> {
        let mut keys = vec![format!("record:{}", record.id)];
        keys.extend(record.library.iter().flat_map(|l| &l.installations).map(|i| format!("path:{}", crate::game_saves::normalized(Path::new(&i.directory)))));
        keys.extend(record.library.iter().flat_map(|l| &l.save_sources).map(|s| format!("path:{}", crate::game_saves::normalized(Path::new(&s.path)))));
        if let Some(path) = &record.save_path { keys.push(format!("path:{}", crate::game_saves::normalized(Path::new(path)))); }
        keys.sort(); keys.dedup();
        let mut active = OPERATIONS.get_or_init(Default::default).lock().map_err(|e| e.to_string())?;
        if keys.iter().any(|k| active.iter().any(|a| conflicts(a,k))) { return Err("该游戏或共用目录正在进行启动、存档或版本操作，请稍后再试".into()); }
        active.extend(keys.iter().cloned());
        Ok(Self(keys))
    }
    pub fn include_paths<'a>(&mut self, paths: impl Iterator<Item=&'a Path>) -> Result<(),String> {
        let keys: Vec<_> = paths.map(|p| format!("path:{}",crate::game_saves::normalized(p))).collect();
        let mut active = OPERATIONS.get_or_init(Default::default).lock().map_err(|e| e.to_string())?;
        if keys.iter().any(|k| active.iter().any(|a| !self.0.contains(a) && conflicts(a,k))) { return Err("存档目录正由另一项操作使用，请稍后重试".into()); }
        for key in keys { if !self.0.contains(&key) { active.insert(key.clone()); self.0.push(key); } }
        Ok(())
    }
}
impl Drop for GameOperation {
    fn drop(&mut self) {
        if let Ok(mut active) = OPERATIONS.get_or_init(Default::default).lock() { for key in &self.0 { active.remove(key); } }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_or_nested_save_directories_have_one_owner_until_the_operation_drops() {
        let game = |id: &str| serde_json::from_value::<GameRecord>(serde_json::json!({
            "id":id,"title":id,"category":"galgame","status":"backlog","progress":"","rating":0
        })).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let mut first=GameOperation::start(&game("operation-a")).unwrap();
        first.include_paths(std::iter::once(temp.path())).unwrap();
        let mut second=GameOperation::start(&game("operation-b")).unwrap();
        assert!(second.include_paths(std::iter::once(temp.path().join("child").as_path())).is_err());
        drop(first);
        second.include_paths(std::iter::once(temp.path())).unwrap();
    }
}
