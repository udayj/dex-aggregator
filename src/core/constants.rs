use num_bigint::BigUint;
use std::str::FromStr;

pub const GET_RESERVES_SELECTOR: &str =
    "0x3cb0e1486e633fbe3e2fafe8aedf12b70ca1860e7467ddb75a17858cde39312";
pub const SCALE: f64 = 1000000_f64;

#[allow(non_snake_case)]
pub fn INFINITE() -> BigUint {
    BigUint::from_str("1000000000000000000000000000").unwrap()
}
