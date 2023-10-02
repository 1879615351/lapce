use std::{rc::Rc, sync::Arc};

use floem::{
    cosmic_text::Style as FontStyle,
    event::EventListener,
    peniko::Color,
    reactive::{create_rw_signal, ReadSignal, RwSignal},
    style::CursorStyle,
    view::View,
    views::{
        container, container_box, label, list, scroll, stack, svg, text,
        virtual_list, Decorators, VirtualListDirection, VirtualListItemSize,
    },
};
use lapce_rpc::{
    dap_types::{DapId, ThreadId},
    terminal::TermId,
};

use super::{position::PanelPosition, view::panel_header};
use crate::{
    app::clickable_icon,
    command::InternalCommand,
    config::{color::LapceColor, icon::LapceIcons, LapceConfig},
    debug::{DapVariable, RunDebugMode, StackTraceData},
    editor::location::{EditorLocation, EditorPosition},
    listener::Listener,
    settings::checkbox,
    terminal::panel::TerminalPanelData,
    window_tab::WindowTabData,
};

pub fn debug_panel(
    window_tab_data: Rc<WindowTabData>,
    position: PanelPosition,
) -> impl View {
    let config = window_tab_data.common.config;
    let terminal = window_tab_data.terminal.clone();
    let internal_command = window_tab_data.common.internal_command;

    stack((
        {
            let terminal = terminal.clone();
            stack((
                panel_header("Processes".to_string(), config),
                debug_processes(terminal, config),
            ))
            .style(|s| s.width_pct(100.0).flex_col().height(150.0))
        },
        stack((
            panel_header("Variables".to_string(), config),
            new_variables_view(window_tab_data.clone()),
        ))
        .style(|s| s.width_pct(100.0).flex_grow(1.0).flex_basis(0.0).flex_col()),
        stack((
            panel_header("Stack Frames".to_string(), config),
            debug_stack_traces(terminal, internal_command, config),
        ))
        .style(|s| s.width_pct(100.0).flex_grow(1.0).flex_basis(0.0).flex_col()),
        stack((
            panel_header("Breakpoints".to_string(), config),
            breakpoints_view(window_tab_data.clone()),
        ))
        .style(|s| s.width_pct(100.0).flex_col().height(150.0)),
    ))
    .style(move |s| {
        s.width_pct(100.0)
            .apply_if(!position.is_bottom(), |s| s.flex_col())
    })
}

fn debug_process_icons(
    terminal: TerminalPanelData,
    term_id: TermId,
    dap_id: DapId,
    mode: RunDebugMode,
    stopped: bool,
    config: ReadSignal<Arc<LapceConfig>>,
) -> impl View {
    let paused = move || {
        let stopped = terminal
            .debug
            .daps
            .with_untracked(|daps| daps.get(&dap_id).map(|dap| dap.stopped));
        stopped.map(|stopped| stopped.get()).unwrap_or(false)
    };
    match mode {
        RunDebugMode::Run => container_box(stack((
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::DEBUG_RESTART,
                    move || {
                        terminal.restart_run_debug(term_id);
                    },
                    || false,
                    || false,
                    config,
                )
                .style(|s| s.margin_horiz(6.0))
            },
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::DEBUG_STOP,
                    move || {
                        terminal.stop_run_debug(term_id);
                    },
                    || false,
                    move || stopped,
                    config,
                )
                .style(|s| s.margin_right(6.0))
            },
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::CLOSE,
                    move || {
                        terminal.close_terminal(&term_id);
                    },
                    || false,
                    || false,
                    config,
                )
                .style(|s| s.margin_right(6.0))
            },
        ))),
        RunDebugMode::Debug => container_box(stack((
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::DEBUG_CONTINUE,
                    move || {
                        terminal.dap_continue(term_id);
                    },
                    || false,
                    move || !paused() || stopped,
                    config,
                )
                .style(|s| s.margin_horiz(6.0))
            },
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::DEBUG_PAUSE,
                    move || {
                        terminal.dap_pause(term_id);
                    },
                    || false,
                    move || paused() || stopped,
                    config,
                )
                .style(|s| s.margin_right(6.0))
            },
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::DEBUG_RESTART,
                    move || {
                        terminal.restart_run_debug(term_id);
                    },
                    || false,
                    || false,
                    config,
                )
                .style(|s| s.margin_right(6.0))
            },
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::DEBUG_STOP,
                    move || {
                        terminal.stop_run_debug(term_id);
                    },
                    || false,
                    move || stopped,
                    config,
                )
                .style(|s| s.margin_right(6.0))
            },
            {
                let terminal = terminal.clone();
                clickable_icon(
                    || LapceIcons::CLOSE,
                    move || {
                        terminal.close_terminal(&term_id);
                    },
                    || false,
                    || false,
                    config,
                )
                .style(|s| s.margin_right(6.0))
            },
        ))),
    }
}

fn debug_processes(
    terminal: TerminalPanelData,
    config: ReadSignal<Arc<LapceConfig>>,
) -> impl View {
    scroll({
        let terminal = terminal.clone();
        let local_terminal = terminal.clone();
        list(
            move || local_terminal.run_debug_process(true),
            |(term_id, p)| (*term_id, p.stopped),
            move |(term_id, p)| {
                let terminal = terminal.clone();
                let is_active =
                    move || terminal.debug.active_term.get() == Some(term_id);
                let local_terminal = terminal.clone();
                stack((
                    {
                        let svg_str = match (&p.mode, p.stopped) {
                            (RunDebugMode::Run, false) => LapceIcons::START,
                            (RunDebugMode::Run, true) => LapceIcons::RUN_ERRORS,
                            (RunDebugMode::Debug, false) => LapceIcons::DEBUG,
                            (RunDebugMode::Debug, true) => {
                                LapceIcons::DEBUG_DISCONNECT
                            }
                        };
                        svg(move || config.get().ui_svg(svg_str)).style(move |s| {
                            let config = config.get();
                            let size = config.ui.icon_size() as f32;
                            s.size(size, size).margin_horiz(10.0).color(
                                *config.get_color(LapceColor::LAPCE_ICON_ACTIVE),
                            )
                        })
                    },
                    label(move || p.config.name.clone()).style(|s| {
                        s.flex_grow(1.0)
                            .flex_basis(0.0)
                            .min_width(0.0)
                            .text_ellipsis()
                    }),
                    debug_process_icons(
                        terminal.clone(),
                        term_id,
                        p.config.dap_id,
                        p.mode,
                        p.stopped,
                        config,
                    ),
                ))
                .on_click(move |_| {
                    local_terminal.debug.active_term.set(Some(term_id));
                    local_terminal.focus_terminal(term_id);
                    true
                })
                .style(move |s| {
                    s.padding_vert(6.0)
                        .width_pct(100.0)
                        .items_center()
                        .apply_if(is_active(), |s| {
                            s.background(
                                *config
                                    .get()
                                    .get_color(LapceColor::PANEL_CURRENT_BACKGROUND),
                            )
                        })
                })
                .hover_style(move |s| {
                    s.cursor(CursorStyle::Pointer).background(
                        (*config
                            .get()
                            .get_color(LapceColor::PANEL_HOVERED_BACKGROUND))
                        .with_alpha_factor(0.3),
                    )
                })
            },
        )
        .style(|s| s.width_pct(100.0).flex_col())
    })
}

fn new_variables_view(window_tab_data: Rc<WindowTabData>) -> impl View {
    let terminal = window_tab_data.terminal.clone();
    let ui_line_height = window_tab_data.common.ui_line_height;
    let config = window_tab_data.common.config;
    container(
        scroll(
            virtual_list(
                VirtualListDirection::Vertical,
                VirtualListItemSize::Fixed(Box::new(move || ui_line_height.get())),
                move || {
                    let dap = terminal.get_active_dap(true);
                    dap.map(|dap| {
                        let process_stopped = terminal
                            .get_terminal(&dap.term_id)
                            .and_then(|t| {
                                t.run_debug.with(|r| r.as_ref().map(|r| r.stopped))
                            })
                            .unwrap_or(true);
                        if process_stopped {
                            return DapVariable::default();
                        }
                        dap.new_variables.get()
                    })
                    .unwrap_or_default()
                },
                |node| {
                    (
                        node.item.name().to_string(),
                        node.item.value().map(|v| v.to_string()),
                        node.item.reference(),
                        node.expanded,
                        node.level,
                    )
                },
                move |node| {
                    let level = node.level;
                    let reference = node.item.reference();
                    let name = node.item.name();
                    stack((
                        svg(move || {
                            let config = config.get();
                            let svg_str = match node.expanded {
                                true => LapceIcons::ITEM_OPENED,
                                false => LapceIcons::ITEM_CLOSED,
                            };
                            config.ui_svg(svg_str)
                        })
                        .style(move |s| {
                            let config = config.get();
                            let size = config.ui.icon_size() as f32;

                            let color = if reference > 0 {
                                *config.get_color(LapceColor::LAPCE_ICON_ACTIVE)
                            } else {
                                Color::TRANSPARENT
                            };
                            s.size(size, size).margin_left(10.0).color(color)
                        }),
                        text(name),
                    ))
                    .style(move |s| {
                        s.items_center()
                            .padding_right(10.0)
                            .padding_left((level * 10) as f32)
                            .min_width_pct(100.0)
                    })
                },
            )
            .style(|s| s.flex_col()),
        )
        .style(|s| s.absolute().size_full()),
    )
    .style(|s| s.size_full().line_height(1.6))
}

fn variables_view(window_tab_data: Rc<WindowTabData>) -> impl View {
    let terminal = window_tab_data.terminal.clone();
    container(
        scroll(
            list(
                move || {
                    let dap = terminal.get_active_dap(true);
                    dap.map(|dap| {
                        let process_stopped = terminal
                            .get_terminal(&dap.term_id)
                            .and_then(|t| {
                                t.run_debug.with(|r| r.as_ref().map(|r| r.stopped))
                            })
                            .unwrap_or(true);
                        if process_stopped {
                            return Vec::new();
                        }
                        dap.variables.get()
                    })
                    .unwrap_or_default()
                },
                move |(scope, variables)| {
                    (scope.name.clone(), scope.variables_reference)
                },
                move |(scope, variables)| {
                    stack((
                        text(scope.name),
                        list(
                            move || variables.clone(),
                            move |var| (var.name.clone(), var.value.clone()),
                            move |var| {
                                stack((
                                    text(var.name),
                                    text(var.value).style(|s| s.margin_left(10.0)),
                                    text(format!("{:?}", var.presentation_hint))
                                        .style(|s| s.margin_left(10.0)),
                                    text(format!("{:?}", var.evaluate_name))
                                        .style(|s| s.margin_left(10.0)),
                                    text(format!("{:?}", var.variables_reference))
                                        .style(|s| s.margin_left(10.0)),
                                ))
                                .style(|s| s.flex_col())
                            },
                        )
                        .style(|s| s.flex_col()),
                    ))
                    .style(|s| s.flex_col())
                },
            )
            .style(|s| s.flex_col()),
        )
        .style(|s| s.absolute().size_pct(100.0, 100.0)),
    )
    .style(|s| s.size_pct(100.0, 100.0))
}

fn debug_stack_frames(
    thread_id: ThreadId,
    stack_trace: StackTraceData,
    stopped: RwSignal<bool>,
    internal_command: Listener<InternalCommand>,
    config: ReadSignal<Arc<LapceConfig>>,
) -> impl View {
    let expanded = stack_trace.expanded;
    stack((
        container(label(move || thread_id.to_string()))
            .on_click(move |_| {
                expanded.update(|expanded| {
                    *expanded = !*expanded;
                });
                true
            })
            .style(|s| s.padding_horiz(10.0).min_width_pct(100.0))
            .hover_style(move |s| {
                s.cursor(CursorStyle::Pointer).background(
                    *config.get().get_color(LapceColor::PANEL_HOVERED_BACKGROUND),
                )
            }),
        list(
            move || {
                let expanded = stack_trace.expanded.get() && stopped.get();
                if expanded {
                    stack_trace.frames.get()
                } else {
                    im::Vector::new()
                }
            },
            |frame| frame.id,
            move |frame| {
                let full_path = frame.source.as_ref().and_then(|s| s.path.clone());
                let line = frame.line.saturating_sub(1);
                let col = frame.column.saturating_sub(1);

                let source_path = frame
                    .source
                    .as_ref()
                    .and_then(|s| s.path.as_ref())
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let has_source = !source_path.is_empty();
                let source_path = format!("{source_path}:{}", frame.line);

                container(stack((
                    label(move || frame.name.clone()).hover_style(move |s| {
                        s.background(
                            *config
                                .get()
                                .get_color(LapceColor::PANEL_HOVERED_BACKGROUND),
                        )
                    }),
                    label(move || source_path.clone()).style(move |s| {
                        s.margin_left(10.0)
                            .color(*config.get().get_color(LapceColor::EDITOR_DIM))
                            .font_style(FontStyle::Italic)
                            .apply_if(!has_source, |s| s.hide())
                    }),
                )))
                .on_click(move |_| {
                    if let Some(path) = full_path.clone() {
                        internal_command.send(InternalCommand::JumpToLocation {
                            location: EditorLocation {
                                path,
                                position: Some(EditorPosition::Position(
                                    lsp_types::Position {
                                        line: line as u32,
                                        character: col as u32,
                                    },
                                )),
                                scroll_offset: None,
                                ignore_unconfirmed: false,
                                same_editor_tab: false,
                            },
                        });
                    }
                    true
                })
                .style(move |s| {
                    s.padding_left(20.0)
                        .padding_right(10.0)
                        .min_width_pct(100.0)
                        .apply_if(!has_source, |s| {
                            s.color(*config.get().get_color(LapceColor::EDITOR_DIM))
                        })
                })
                .hover_style(move |s| {
                    s.background(
                        *config
                            .get()
                            .get_color(LapceColor::PANEL_HOVERED_BACKGROUND),
                    )
                    .apply_if(has_source, |s| s.cursor(CursorStyle::Pointer))
                })
            },
        )
        .style(|s| s.flex_col().min_width_pct(100.0)),
    ))
    .style(|s| s.flex_col().min_width_pct(100.0))
}

fn debug_stack_traces(
    terminal: TerminalPanelData,
    internal_command: Listener<InternalCommand>,
    config: ReadSignal<Arc<LapceConfig>>,
) -> impl View {
    container(
        scroll({
            let local_terminal = terminal.clone();
            list(
                move || {
                    let dap = local_terminal.get_active_dap(true);
                    if let Some(dap) = dap {
                        let process_stopped = local_terminal
                            .get_terminal(&dap.term_id)
                            .and_then(|t| {
                                t.run_debug.with(|r| r.as_ref().map(|r| r.stopped))
                            })
                            .unwrap_or(true);
                        if process_stopped {
                            return Vec::new();
                        }
                        let main_thread = dap.thread_id.get();
                        let stack_traces = dap.stack_traces.get();
                        let mut traces = stack_traces
                            .into_iter()
                            .map(|(thread_id, stack_trace)| {
                                (dap.dap_id, dap.stopped, thread_id, stack_trace)
                            })
                            .collect::<Vec<_>>();
                        traces.sort_by_key(|(_, _, id, _)| main_thread != Some(*id));
                        traces
                    } else {
                        Vec::new()
                    }
                },
                |(dap_id, stopped, thread_id, _)| {
                    (*dap_id, *thread_id, stopped.get_untracked())
                },
                move |(_, stopped, thread_id, stack_trace)| {
                    debug_stack_frames(
                        thread_id,
                        stack_trace,
                        stopped,
                        internal_command,
                        config,
                    )
                },
            )
            .style(|s| s.flex_col().min_width_pct(100.0))
        })
        .style(|s| s.absolute().size_pct(100.0, 100.0)),
    )
    .style(|s| {
        s.width_pct(100.0)
            .line_height(1.6)
            .flex_grow(1.0)
            .flex_basis(0.0)
    })
}

fn breakpoints_view(window_tab_data: Rc<WindowTabData>) -> impl View {
    let breakpoints = window_tab_data.terminal.debug.breakpoints;
    let config = window_tab_data.common.config;
    let workspace = window_tab_data.common.workspace.clone();
    let available_width = create_rw_signal(0.0);
    let internal_command = window_tab_data.common.internal_command;
    container(
        scroll(
            list(
                move || {
                    breakpoints
                        .get()
                        .into_iter()
                        .flat_map(|(path, breakpoints)| {
                            breakpoints.into_values().map(move |b| (path.clone(), b))
                        })
                },
                move |(path, breakpoint)| {
                    (path.clone(), breakpoint.line, breakpoint.active)
                },
                move |(path, breakpoint)| {
                    let line = breakpoint.line;
                    let full_path = path.clone();
                    let full_path_for_jump = path.clone();
                    let full_path_for_close = path.clone();
                    let path = if let Some(workspace_path) = workspace.path.as_ref()
                    {
                        path.strip_prefix(workspace_path)
                            .unwrap_or(&full_path)
                            .to_path_buf()
                    } else {
                        path
                    };

                    let file_name =
                        path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    let folder =
                        path.parent().and_then(|s| s.to_str()).unwrap_or("");
                    let folder_empty = folder.is_empty();

                    stack((
                        clickable_icon(
                            move || LapceIcons::CLOSE,
                            move || {
                                breakpoints.update(|breakpoints| {
                                    if let Some(breakpoints) =
                                        breakpoints.get_mut(&full_path_for_close)
                                    {
                                        breakpoints.remove(&line);
                                    }
                                });
                            },
                            || false,
                            || false,
                            config,
                        )
                        .on_event(EventListener::PointerDown, |_| true),
                        checkbox(move || breakpoint.active, config)
                            .style(|s| {
                                s.margin_right(6.0).cursor(CursorStyle::Pointer)
                            })
                            .on_click(move |_| {
                                breakpoints.update(|breakpoints| {
                                    if let Some(breakpoints) =
                                        breakpoints.get_mut(&full_path)
                                    {
                                        if let Some(breakpoint) =
                                            breakpoints.get_mut(&line)
                                        {
                                            breakpoint.active = !breakpoint.active;
                                        }
                                    }
                                });
                                true
                            }),
                        text(format!("{file_name}:{}", breakpoint.line + 1)).style(
                            move |s| {
                                let size = config.get().ui.icon_size() as f32;
                                s.text_ellipsis().max_width(
                                    available_width.get() as f32
                                        - 20.0
                                        - size
                                        - 6.0
                                        - size
                                        - 8.0,
                                )
                            },
                        ),
                        text(folder).style(move |s| {
                            s.text_ellipsis()
                                .flex_grow(1.0)
                                .flex_basis(0.0)
                                .color(
                                    *config.get().get_color(LapceColor::EDITOR_DIM),
                                )
                                .min_width(0.0)
                                .margin_left(6.0)
                                .apply_if(folder_empty, |s| s.hide())
                        }),
                    ))
                    .style(|s| s.items_center().padding_horiz(10.0).width_pct(100.0))
                    .hover_style(move |s| {
                        s.background(
                            *config
                                .get()
                                .get_color(LapceColor::PANEL_HOVERED_BACKGROUND),
                        )
                    })
                    .on_click(move |_| {
                        internal_command.send(InternalCommand::JumpToLocation {
                            location: EditorLocation {
                                path: full_path_for_jump.clone(),
                                position: Some(EditorPosition::Line(line)),
                                scroll_offset: None,
                                ignore_unconfirmed: false,
                                same_editor_tab: false,
                            },
                        });
                        true
                    })
                },
            )
            .style(|s| s.flex_col().line_height(1.6).width_pct(100.0)),
        )
        .on_resize(move |rect| {
            let width = rect.width();
            if available_width.get_untracked() != width {
                available_width.set(width);
            }
        })
        .style(|s| s.absolute().size_pct(100.0, 100.0)),
    )
    .style(|s| s.size_pct(100.0, 100.0))
}
