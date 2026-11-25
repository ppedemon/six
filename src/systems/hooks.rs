use crate::{active_session, components::EditorCtx, systems::insert::post_insert};

pub fn on_insert_start(ctx: &mut EditorCtx) {
    let (session, _) = active_session!(mut ctx);
    session.insert_log.init();
}

pub fn on_insert_end(ctx: &mut EditorCtx) {
    post_insert(ctx);
}

pub fn on_load() {
  
}
