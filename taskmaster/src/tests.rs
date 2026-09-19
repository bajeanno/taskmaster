use std::{fs, path::PathBuf};

pub struct TestDir(PathBuf);

impl TestDir {
    pub fn new(name: &str) -> Self {
        let path = PathBuf::from("target/tests/").join(name);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn sub_dir(mut self, name: &str) -> Self {
        self.0 = self.0.join(name);
        fs::create_dir_all(self.0.clone()).unwrap();
        self
    }

    pub fn join(&self, name: &str) -> String {
        let path = self.0.join(name);
        path.to_str().unwrap().to_string()
    }

    pub fn write(&self, name: &str, content: &str) -> String {
        let path = self.join(name);
        fs::write(&path, content).unwrap();
        path
    }
}
