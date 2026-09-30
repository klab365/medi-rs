use medi_rs::medi_module;

struct Database;

medi_module! {
    manifest storage;
    resources { Database; }
}

fn main() {}
