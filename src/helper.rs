use rand::{self, distributions::Alphanumeric, Rng};
use std::{fs::File, io::Read};

pub fn random_string(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .collect::<String>()
}

pub fn read_file(file: &str) -> String {
    let mut file = File::open(file).unwrap();
    let mut content = String::new();
    let _ = file.read_to_string(&mut content);
    content
}
