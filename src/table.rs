use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Table {
    path: PathBuf,
}

impl Table {
    pub fn load(path: PathBuf) -> Self {
        Self {
            path: path
        }
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}