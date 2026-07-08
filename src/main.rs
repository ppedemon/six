use anyhow::Result;
use crossterm::{
    event, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, Stdout},
    time::Duration,
};

use crate::{components::EditorCtx, systems::init_cursor_pos};

mod cmd;
mod components;
mod digraphs;
mod ex;
mod misc;
mod rope;
mod systems;

fn main() -> Result<()> {
    #[cfg(feature = "color-eyre")]
    {
        let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default().into_hooks();

        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = crossterm::terminal::disable_raw_mode();
            let _ =
                crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);

            let report = panic_hook.panic_report(panic_info);
            eprintln!("{}", report);
        }));

        eyre_hook.install()?;
    }

    digraphs::load_digraphs();

    let mut world = hecs::World::new();
    let mut ctx = systems::create_editor(&mut world);

    let args = std::env::args().collect::<Vec<_>>();
    if args.len() == 1 {
        systems::create_empty_session(&mut ctx)?;
    } else {
        systems::load_session(&mut ctx, &args[1])?;
    };

    run(ctx)
}

fn run(ctx: EditorCtx) -> Result<()> {
    let mut terminal = setup_terminal()?;
    editor_loop(ctx, &mut terminal)?;
    restore_terminal(&mut terminal)?;
    Ok(())
}

fn editor_loop(
    mut ctx: EditorCtx,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<()> {
    let mut input_handler = systems::InputHandler::new();

    // The init_cursor_pos function jumps to the first non-blank character of
    // the active sesssion, which might be anywere and hence force scrolling.
    // Since scrolling requires viewport size, we do a pre-render pass first.
    terminal.draw(|frame| {
        systems::pre_render(&mut ctx, frame.area()).unwrap();
        init_cursor_pos(&ctx).unwrap();
    })?;

    while !systems::should_quit(&ctx)? {
        terminal.draw(|frame| {
            let area = frame.area();
            systems::pre_render(&mut ctx, area).unwrap();
            systems::render(&ctx, area, frame.buffer_mut()).unwrap();

            let cursor_pos = systems::cursor_pos(&ctx).unwrap();
            frame.set_cursor_position(cursor_pos);
        })?;

        if event::poll(Duration::from_millis(250))? {
            let event = event::read()?;
            input_handler.handle_event(&ctx, event)?;
        }

        systems::post_edit(&ctx)?;
        systems::handle_ex_state(&ctx);
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(terminal.show_cursor()?)
}
