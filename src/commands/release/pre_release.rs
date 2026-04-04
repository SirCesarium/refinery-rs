use crate::{commands::release::ReleaseAction, subcmd};

subcmd!(ReleaseAction, pre_release() {
    todo!("Implement Cargo.toml version upgrade & git push logic");
});
