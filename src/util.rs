pub fn is_succeeded(result: i32) -> bool {
    return result >= 0;
}

pub fn is_failed(result: i32) -> bool {
    return result < 0;
}
