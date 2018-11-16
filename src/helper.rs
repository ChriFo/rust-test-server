pub use rand::random;
use rand::{self, distributions::Alphanumeric, Rng};
use std::{
    fs::File,
    io::{Read, Result},
};

/// Generates a random string with given size.
pub fn random_string(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .collect::<String>()
}

/// Reads file content into string result.
pub fn read_file(file: &str) -> Result<String> {
    let mut file = File::open(file)?;
    let mut content = String::new();
    let _ = file.read_to_string(&mut content);
    Ok(content)
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn random_string_has_given_size() {
        let size: u8 = random();
        let string = random_string(size as usize);

        assert_eq!(string.len(), size as usize);
    }

    #[test]
    fn random_strings_are_different() {
        let size: u8 = std::cmp::max(1, random());
        let first_string = random_string(size as usize);
        let second_string = random_string(size as usize);

        assert_that(&first_string).is_not_equal_to(&second_string);
    }

    #[test]
    fn read_file_returns_error_if_file_not_exists() {
        let not_exist = read_file("a");

        assert!(not_exist.is_err());
    }

    #[test]
    fn read_file_returns_content() {
        let content = read_file("tests/read_file_test");

        assert!(content.is_ok());
        assert_eq!(&content.unwrap(), "a1b2c3\n");
    }
}
