pub fn get_bit(num: usize, i: u8) -> u8 {
    ((num >> i) & 1) as u8
}

pub fn set_bit(num: usize, idx: u8) -> u8 {
    (num | (1 << idx)) as u8
}

pub fn unset_bit(num: usize, idx: u8) -> u8 {
    (num & !(1 << idx)) as u8
}
