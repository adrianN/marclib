#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TriStateBool {
    True,
    False,
    Null,
}
impl std::ops::Not for TriStateBool {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            TriStateBool::True => TriStateBool::False,
            TriStateBool::False => TriStateBool::True,
            TriStateBool::Null => TriStateBool::Null,
        }
    }
}
pub fn parse_usize5(slice: &[u8]) -> usize {
    unsafe {
        let n0 = *(slice.get_unchecked(0)) as usize;
        let n1 = *(slice.get_unchecked(1)) as usize;
        let n2 = *(slice.get_unchecked(2)) as usize;
        let n3 = *(slice.get_unchecked(3)) as usize;
        let n4 = *(slice.get_unchecked(4)) as usize;
        let zero = b'0' as usize;
        n0 * 10000 + n1 * 1000 + n2 * 100 + n3 * 10 + n4
            - (10000 * zero + 1000 * zero + 100 * zero + 10 * zero + zero)
    }
}

pub fn parse_usize4(slice: &[u8]) -> usize {
    unsafe {
        let n0 = *(slice.get_unchecked(0)) as usize;
        let n1 = *(slice.get_unchecked(1)) as usize;
        let n2 = *(slice.get_unchecked(2)) as usize;
        let n3 = *(slice.get_unchecked(3)) as usize;
        let zero = b'0' as usize;
        n0 * 1000 + n1 * 100 + n2 * 10 + n3 - (1000 * zero + 100 * zero + 10 * zero + 1 * zero)
    }
}

pub fn parse_usize3(slice: &[u8]) -> usize {
    unsafe {
        let n0 = *(slice.get_unchecked(0)) as usize;
        let n1 = *(slice.get_unchecked(1)) as usize;
        let n2 = *(slice.get_unchecked(2)) as usize;
        let zero = b'0' as usize;
        n0 * 100 + n1 * 10 + n2 - (100 * zero + 10 * zero + 1 * zero)
    }
}

pub fn parse_usize(slice: &[u8]) -> usize {
    assert!(slice.len() < 6);
    let mut n: usize = 0;
    for i in slice {
        n *= 10;
        n += (i - b'0') as usize;
    }
    n
}

pub fn write_usize(n: usize, len: usize, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    assert!(n < 100000);
    let mut n_len: usize = 0;
    let mut m = n;
    let mut buf: [u8; 5] = [b'0', b'0', b'0', b'0', b'0'];
    while m > 0 {
        buf[buf.len() - n_len - 1] = b'0' + (m % 10) as u8;
        m /= 10;
        n_len += 1;
    }
    writer.write_all(&buf[5 - len..])
}
