use langfuse_ergonomic::observe;

#[observe(as_type = "llm")]
fn f() {}

fn main() {}
