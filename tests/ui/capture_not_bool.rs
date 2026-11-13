use langfuse_ergonomic::observe;

#[observe(capture_input = "no")]
fn f() {}

fn main() {}
