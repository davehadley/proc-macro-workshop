use derive_builder::Builder;

// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run

fn main() {
    #[derive(Builder)]
    pub struct MyStruct {
        value1: u32,
        value2: u64,
        value3: Option<f64>,
    }
}
