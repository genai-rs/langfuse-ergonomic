use langfuse_ergonomic::observe;

#[observe]
const fn f() -> u8 {
    1
}

fn main() {}
