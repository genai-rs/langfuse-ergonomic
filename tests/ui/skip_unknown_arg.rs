use langfuse_ergonomic::observe;

#[observe(skip(password))]
fn login(user: &str) -> usize {
    user.len()
}

fn main() {}
