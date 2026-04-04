use crate::{commands::release::ReleaseAction, subcmd};

subcmd!(ReleaseAction, minor() {
    todo!("Implement Cargo.toml version upgrade & git push logic");
});
