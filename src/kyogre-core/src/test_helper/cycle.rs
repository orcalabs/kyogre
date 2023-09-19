#[derive(Debug, Clone, Copy)]
pub struct Cycle(u8);

impl Cycle {
    pub fn val(&self) -> u8 {
        self.0
    }
    pub(super) fn new() -> Cycle {
        Cycle(1)
    }
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl PartialEq<u8> for Cycle {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}
