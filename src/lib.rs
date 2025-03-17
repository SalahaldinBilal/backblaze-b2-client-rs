pub mod client;
pub mod definitions;
pub mod error;
pub mod simple_client;
pub mod tasks;
pub mod throttle;
pub mod util;

pub use reqwest;
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
