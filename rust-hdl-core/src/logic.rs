use crate::ast::Verilog;

pub trait Logic {
    fn update(&mut self);
    fn connect(&mut self) {}
    fn hdl(&self) -> Verilog {
        Verilog::Empty
    }
}

pub fn logic_connect_fn<L: Logic>(x: &mut L) {
    x.connect();
}

impl<L: Logic, const P: usize> Logic for [L; P] {
    fn update(&mut self) {}
}
