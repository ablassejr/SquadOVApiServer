pub fn integer_log2(input: i32) -> i32 {
    let mut tmp = input >> 1;
    let mut res = 0;
    while tmp > 0 {
        res += 1;
        tmp >>= 1;
    }
    res
}