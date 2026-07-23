use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Position, Rect, Size},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget},
};
use ropey::Rope;
use unicode_width::UnicodeWidthStr;

use crate::{
    active_session,
    components::{
        self, BufferView, Config, DisplayBuffer, EditorCtx, ExSession, Focus, Level, Session,
        Status, TextStyle, Viewport,
    },
};

const MIN_SIZE: Size = Size::new(6, 2);

pub fn render(ctx: &mut EditorCtx, area: Rect, frame_buf: &mut Buffer) {
    if area.width < MIN_SIZE.width || area.height < MIN_SIZE.height {
        return;
    }

    for (session, buf_view) in ctx.sessions.values_mut() {
        let buffer = ctx.buffers.get(&session.buf_id).unwrap();
        render_session(&ctx.config, session, buf_view, buffer, frame_buf);
    }

    match ctx.editor.focus {
        Focus::Ex => render_ex(
            &ctx.config,
            &ctx.ex_session,
            &mut ctx.ex_buffer_view,
            frame_buf,
        ),
        Focus::Session => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(1)])
                .split(area);
            render_status(&ctx.status, chunks[1], frame_buf);
        }
    }

    if let Some(c) = ctx.editor.char_at_cursor {
        let pos = cursor_pos(ctx);
        frame_buf[pos].set_char(c);
    }
}

pub fn cursor_pos(ctx: &EditorCtx) -> Position {
    match ctx.editor.focus {
        Focus::Ex => {
            let ex_cursor = ctx.ex_buffer_view.cursor;
            ctx.ex_session.viewport.cursor_pos(ex_cursor)
        }
        Focus::Session => {
            let (session, buf_view) = active_session!(ctx);
            session.viewport.cursor_pos(buf_view.cursor)
        }
    }
}

fn render_session(
    config: &Config,
    session: &Session,
    buf_view: &mut BufferView,
    buffer: &components::Buffer,
    frame_buf: &mut Buffer,
) {
    let viewport = &session.viewport;

    render_lines(
        config,
        buffer.rope(),
        viewport,
        &mut buf_view.display_buf,
        frame_buf,
    );

    let style_empty = Style::default().blue().bold();

    let start_line = viewport.scroll.row;
    let visible_lines = viewport.area.height as usize;
    let end_line = start_line
        .saturating_add(visible_lines)
        .min(buffer.rope().len_lines());

    let num_lines = end_line - start_line;
    for y in num_lines..visible_lines {
        frame_buf[(0, viewport.area.y + y as u16)]
            .set_char('~')
            .set_style(style_empty);
    }
}

fn render_ex(
    config: &Config,
    ex_session: &ExSession,
    buf_view: &mut BufferView,
    frame_buf: &mut Buffer,
) {
    render_lines(
        config,
        ex_session.rope(),
        &ex_session.viewport,
        &mut buf_view.display_buf,
        frame_buf,
    );
}

fn render_status(status: &Status, area: Rect, frame_buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Max(Status::MAX_CMD as u16),
            Constraint::Length(Status::MAX_RULER as u16),
        ])
        .split(area);

    let status_text = Line::from(status.msg.as_str());

    let status_text = match status.text_style {
        TextStyle::None => status_text,
        TextStyle::Bold => status_text.bold(),
        TextStyle::Italic => status_text.italic(),
        TextStyle::ItalicBold => status_text.italic().bold(),
    };

    let status_text = match status.level {
        Level::Info => status_text.white(),
        Level::Warn => status_text.yellow(),
        Level::Error => status_text.on_red(),
    };

    Paragraph::new(status_text).render(chunks[0], frame_buf);

    let cmd_text = Line::from(status.cmd.as_str());
    Paragraph::new(cmd_text).render(chunks[1], frame_buf);

    let ruler_text = Line::from(status.ruler.as_str());
    Paragraph::new(ruler_text)
        .block(Block::default().padding(Padding::horizontal(1)))
        .render(chunks[2], frame_buf);
}

fn render_lines(
    config: &Config,
    rope: &Rope,
    viewport: &Viewport,
    display_buf: &mut DisplayBuffer,
    frame_buf: &mut Buffer,
) {
    let style_default = Style::default();
    let style_too_wide = Style::default().blue();

    let width = viewport.area.width as usize;
    let height = viewport.area.height as usize;

    let start_line = viewport.scroll.row;
    let visible_lines = height;
    let end_line = start_line
        .saturating_add(visible_lines)
        .min(rope.len_lines());

    let start_col = viewport.scroll.col;
    let end_col = viewport.scroll.col.saturating_add(width);

    for (i, line_idx) in (start_line..end_line).enumerate() {
        let display_line = display_buf.ensure_line(config, rope, line_idx);
        let y = viewport.area.y + i as u16;

        for (g, span) in display_line.graphemes_between(start_col, end_col) {
            let width = g.width();
            if span.start + 1 == start_col && width == 2 {
                frame_buf[(0, y)].set_char('<').set_style(style_too_wide);
            } else if span.start + 1 == end_col && width == 2 {
                frame_buf[((span.start - start_col) as u16, y)]
                    .set_char('>')
                    .set_style(style_too_wide);
            } else if span.start >= start_col && span.start < end_col {
                frame_buf.set_stringn((span.start - start_col) as u16, y, g, width, style_default);
            }
        }
    }
}
