use langfuse_ergonomic::observe;

#[observe(name = "a", name = "b")]
fn f() {}

fn main() {}
