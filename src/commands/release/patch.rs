use crate::{commands::release::ReleaseAction, subcmd};

subcmd!(ReleaseAction, patch() {
    todo!("Implement Cargo.toml version upgrade & git push logic");
});
