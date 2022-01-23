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
        #[builder(each = "value2_single")]
        value2: Vec<u8>,
    }
}
