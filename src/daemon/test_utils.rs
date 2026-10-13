use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TestDir {
    root: PathBuf,
    current: PathBuf,
}

impl TestDir {
    pub fn new(name: &str) -> Self {
        let id = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            PathBuf::from("target/tests/").join(format!("{name}-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self {
            current: root.clone(),
            root,
        }
    }

    pub fn sub_dir(mut self, name: &str) -> Self {
        self.current = self.current.join(name);
        fs::create_dir_all(&self.current).unwrap();
        self
    }

    pub fn join(&self, name: &str) -> String {
        let path = self.current.join(name);
        path.to_str().unwrap().to_string()
    }

    pub fn write(&self, name: &str, content: &str) -> String {
        let path = self.join(name);
        fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
