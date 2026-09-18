use std::{fs::create_dir_all, path::PathBuf};

fn repo_target_dir() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tests");
    create_dir_all(path.clone()).expect("failed to create test directory in target/tests");
    path
}

pub fn test_artifacts_dir(sub: &str) -> PathBuf {
    let path = PathBuf::from(repo_target_dir()).join(sub);
    create_dir_all(path.clone())
        .expect(format!("failed to create test sub directory in target/tests/{sub}").as_str());
    path
}
