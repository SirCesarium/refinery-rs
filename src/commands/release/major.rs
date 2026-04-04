use crate::{commands::release::ReleaseAction, subcmd};

subcmd!(ReleaseAction, major() {
    todo!("Implement Cargo.toml version upgrade & git push logic");
});
