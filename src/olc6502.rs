use super::bus;

pub struct Olc6502 {

}

impl Olc6502 {
    fn connect_bus(n: &bus::Bus) {
        bus = n;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
