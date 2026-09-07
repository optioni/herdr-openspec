pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
pub fn sub(a: i32, b: i32) -> i32 {
    a - b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_sub() {
        assert_eq!(sub(5, 3), 2);
    }
}
