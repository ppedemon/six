use crate::{
    active_session,
    cmd::{Cmd, InsertPoint},
    components::{BufferView, Config, EditorCtx, ExMode, Focus, Level, Mode, MutBuffer, TextStyle},
    ex::{Address, Delimiter, ExRange},
    systems::{
        hooks,
        insert::{clear_ex, insert_char},
        nav::utils,
    },
};

pub fn enter_insert(ctx: &mut EditorCtx, insert_point: InsertPoint) {
    clear_ex(ctx);

    ctx.editor.focus = Focus::Session;
    ctx.editor.char_at_cursor = None;

    ctx.status.clear_cmd();
    ctx.status
        .set_styled_msg(Level::Info, TextStyle::Bold, "-- INSERT --");

    let (session, buf_view) = active_session!(mut ctx);
    session.mode = Mode::Insert;
    utils::apply_insert_point(ctx, insert_point);

    hooks::on_insert_start(ctx);
}

pub fn enter_normal(ctx: &mut EditorCtx) {
    ctx.editor.focus = Focus::Session;
    ctx.editor.char_at_cursor = None;

    ctx.status.clear_cmd();
    ctx.status.clear_msg();

    let (session, _) = active_session!(mut ctx);
    let old_mode = session.mode;
    session.mode = Mode::Normal;

    if old_mode == Mode::Insert {
        hooks::on_insert_end(ctx);
    }
}

pub fn enter_ex(ctx: &mut EditorCtx, ex_mode: ExMode, cmd: Cmd) {
    clear_ex(ctx);
    prepare_ex(ctx, ex_mode, cmd);

    ctx.editor.focus = Focus::Ex;
    ctx.editor.char_at_cursor = None;
    utils::apply_insert_point(ctx, InsertPoint::Last);
}

fn prepare_ex(ctx: &mut EditorCtx, ex_mode: ExMode, cmd: Cmd) {
    let ex_session = &mut ctx.ex_session;
    ex_session.mode = ex_mode;

    let buf_view = &mut ctx.ex_buffer_view;
    let buffer = &mut ex_session.edit();
    let c = ex_mode.to_char();

    match ex_mode {
        ExMode::Colon => {
            let ex_range = reps_to_range(cmd.reps);
            insert_char(&ctx.config, buf_view, buffer, c);
            insert_str(&ex_range.to_string(), &ctx.config, buffer, buf_view);
        }
        _ => match ctx.last_search.last_pat_search() {
            None => {
                insert_char(&ctx.config, buf_view, buffer, c);
            }
            Some(p) => {
                insert_str(&p.to_string(), &ctx.config, buffer, buf_view);
            }
        },
    }
}

fn reps_to_range(reps: Option<usize>) -> ExRange {
    reps.filter(|reps| *reps > 1)
        .map(|reps| {
            ExRange::pair(
                Address::current(),
                Delimiter::Comma,
                Address::current_plus(reps - 1),
            )
        })
        .unwrap_or(ExRange::Implicit)
}

fn insert_str(s: &str, config: &Config, buffer: &mut impl MutBuffer, buf_view: &mut BufferView) {
    s.to_string().chars().for_each(|c| {
        let _ = insert_char(config, buf_view, buffer, c);
    });
}
