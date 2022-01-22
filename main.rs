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
    struct MyStruct {
        value: u32,
    }
}
