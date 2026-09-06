use std::io::{self, BufRead, Read as _, Write as _};
use std::sync::{Arc, Mutex};

use base64::Engine;
use interprocess::local_socket::traits::Stream as _;
use interprocess::TryClone as _;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tracing::info;

use crate::ipc::LocalStream;
use crate::protocol::{
    self, AttachScrollDirection, AttachScrollSource, ClientMessage, RenderEncoding, ServerMessage,
    MAX_GRAPHICS_FRAME_SIZE,
};
use crate::server::socket_paths::client_socket_path;

use super::{do_handshake, init_logging, write_to_server, ClientError};

/// Runs a read-only terminal session observer and prints one JSON envelope per frame.
pub fn run_terminal_session_observe(target: String, cols: u16, rows: u16) -> io::Result<()> {
    let mut stream =
        connect_terminal_session_stream(&target, cols, rows, "observing terminal session")?;
    write_to_server(&mut stream, &ClientMessage::ObserveTerminal { target })?;
    write_terminal_session_output(stream)
}

/// Runs a writable terminal session controller.
pub fn run_terminal_session_control(
    target: String,
    takeover: bool,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let mut stream =
        connect_terminal_session_stream(&target, cols, rows, "controlling terminal session")?;
    write_to_server(
        &mut stream,
        &ClientMessage::ControlTerminal { target, takeover },
    )?;

    run_json_session_controller(stream)
}

/// Runs a writable rendered full-app session controller for bridge processes.
pub fn run_app_session_control(cols: u16, rows: u16) -> io::Result<()> {
    init_logging();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| io::Error::other(err.to_string()))?;
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| io::Error::other(err.to_string()))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|err| io::Error::other(err.to_string()))?;
    let executable = std::env::current_exe()?;
    let mut command = CommandBuilder::new(executable);
    command.env("TERM", "xterm-256color");
    command.env_remove(crate::HERDR_ENV_VAR);
    command.env_remove("HERDR_WORKSPACE_ID");
    command.env_remove("HERDR_TAB_ID");
    command.env_remove("HERDR_PANE_ID");
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| io::Error::other(err.to_string()))?;
    drop(pair.slave);

    let master = Arc::new(Mutex::new(pair.master));
    let child = Arc::new(Mutex::new(child));
    let input_master = Arc::clone(&master);
    let input_child = Arc::clone(&child);
    let _input_thread = std::thread::spawn(move || {
        let mut writer = writer;
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else {
                break;
            };
            if line.trim().is_empty() {
                continue;
            }
            match terminal_app_command_from_json(&line) {
                Ok(TerminalAppCommand::Input(data)) => {
                    if writer
                        .write_all(&data)
                        .and_then(|()| writer.flush())
                        .is_err()
                    {
                        return;
                    }
                }
                Ok(TerminalAppCommand::Resize(size)) => {
                    let result = input_master
                        .lock()
                        .map_err(|_| ())
                        .and_then(|master| master.resize(size).map_err(|_| ()));
                    if result.is_err() {
                        return;
                    }
                }
                Ok(TerminalAppCommand::Release) => {
                    if let Ok(mut child) = input_child.lock() {
                        let _ = child.kill();
                    }
                    return;
                }
                Err(err) => eprintln!("herdr: terminal app session input ignored: {err}"),
            }
        }
        if let Ok(mut child) = input_child.lock() {
            let _ = child.kill();
        }
    });

    let mut stdout = io::stdout().lock();
    let mut bytes = vec![0_u8; 64 * 1024];
    let mut seq = 0_u64;
    loop {
        match reader.read(&mut bytes) {
            Ok(0) => break,
            Ok(len) => {
                seq = seq.saturating_add(1);
                let size = master
                    .lock()
                    .ok()
                    .and_then(|master| master.get_size().ok())
                    .unwrap_or(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                write_terminal_session_frame(
                    &mut stdout,
                    seq,
                    size.cols,
                    size.rows,
                    seq == 1,
                    &bytes[..len],
                )?;
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) if pty_reader_reached_eof(&err) => break,
            Err(err) => return Err(err),
        }
    }
    if let Ok(mut child) = child.lock() {
        let _ = child.wait();
    }
    write_terminal_session_closed(&mut stdout, Some("released"))
}

fn pty_reader_reached_eof(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::BrokenPipe
        || error.kind() == io::ErrorKind::UnexpectedEof
        || (cfg!(unix) && error.raw_os_error() == Some(5))
}

fn run_json_session_controller(stream: LocalStream) -> io::Result<()> {
    let mut write_stream = stream.try_clone()?;
    let _input_thread = std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else {
                break;
            };
            if line.trim().is_empty() {
                continue;
            }
            match terminal_control_command_from_json(&line) {
                Ok(message) => {
                    let release = matches!(message, ClientMessage::Detach);
                    if write_to_server(&mut write_stream, &message).is_err() {
                        return;
                    }
                    if release {
                        return;
                    }
                }
                Err(err) => eprintln!("herdr: terminal session control input ignored: {err}"),
            }
        }
        let _ = write_to_server(&mut write_stream, &ClientMessage::Detach);
    });

    write_terminal_session_output(stream)
}

fn connect_terminal_session_stream(
    target: &str,
    cols: u16,
    rows: u16,
    log_message: &'static str,
) -> io::Result<LocalStream> {
    connect_json_session_stream(cols, rows, log_message, target)
}

fn connect_json_session_stream(
    cols: u16,
    rows: u16,
    log_message: &'static str,
    target: &str,
) -> io::Result<LocalStream> {
    init_logging();

    let socket_path = client_socket_path();
    crate::logging::startup("client");
    info!(path = %socket_path.display(), target, cols, rows, "{log_message}");

    let mut stream = match crate::ipc::connect_local_stream(&socket_path) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("herdr: {}", ClientError::ConnectionFailed(err));
            std::process::exit(1);
        }
    };

    match do_handshake(
        &mut stream,
        cols,
        rows,
        0,
        0,
        false,
        None,
        false,
        false,
        true,
    ) {
        Ok(handshake) if handshake.encoding == RenderEncoding::TerminalAnsi => {}
        Ok(handshake) => {
            eprintln!(
                "herdr: terminal session negotiated unsupported encoding {:?}",
                handshake.encoding
            );
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("herdr: {err}");
            std::process::exit(1);
        }
    }

    stream.set_nonblocking(false)?;
    Ok(stream)
}

fn write_terminal_session_output(mut stream: LocalStream) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    loop {
        match protocol::read_message(&mut stream, MAX_GRAPHICS_FRAME_SIZE) {
            Ok(ServerMessage::Terminal(frame)) => {
                write_terminal_session_frame(
                    &mut stdout,
                    frame.seq,
                    frame.width,
                    frame.height,
                    frame.full,
                    &frame.bytes,
                )?;
            }
            Ok(ServerMessage::MouseCapture {
                enabled,
                sgr_pixels,
            }) => {
                let bytes = terminal_session_mouse_capture_bytes(enabled, sgr_pixels);
                write_terminal_session_frame(&mut stdout, 0, 0, 0, false, &bytes)?;
            }
            Ok(ServerMessage::ServerShutdown { reason }) => {
                return write_terminal_session_closed(&mut stdout, reason.as_deref());
            }
            Ok(ServerMessage::Graphics { .. }) => {}
            Ok(_) => {}
            Err(protocol::FramingError::UnexpectedEof) => return Ok(()),
            Err(err) => return Err(io::Error::other(err.to_string())),
        }
    }
}

fn write_terminal_session_closed(
    stdout: &mut impl io::Write,
    reason: Option<&str>,
) -> io::Result<()> {
    let line = serde_json::json!({
        "type": "terminal.closed",
        "reason": reason,
    });
    serde_json::to_writer(&mut *stdout, &line)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

fn write_terminal_session_frame(
    stdout: &mut impl io::Write,
    seq: u64,
    width: u16,
    height: u16,
    full: bool,
    bytes: &[u8],
) -> io::Result<()> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let line = serde_json::json!({
        "type": "terminal.frame",
        "seq": seq,
        "encoding": "ansi",
        "width": width,
        "height": height,
        "full": full,
        "bytes": encoded,
    });
    serde_json::to_writer(&mut *stdout, &line)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

pub(super) fn terminal_session_mouse_capture_bytes(enabled: bool, sgr_pixels: bool) -> Vec<u8> {
    const RESET: &[u8] =
        b"\x1b[?1016l\x1b[?1006l\x1b[?1015l\x1b[?1005l\x1b[?1003l\x1b[?1002l\x1b[?1000l";
    const ENABLE: &[u8] = b"\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h";
    let mut bytes = RESET.to_vec();
    if enabled {
        bytes.extend_from_slice(ENABLE);
        bytes.extend_from_slice(if sgr_pixels {
            b"\x1b[?1016h"
        } else {
            b"\x1b[?1016l"
        });
    }
    bytes
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum TerminalControlCommand {
    #[serde(rename = "terminal.input")]
    Input {
        text: Option<String>,
        bytes: Option<String>,
    },
    #[serde(rename = "terminal.resize")]
    Resize {
        cols: u16,
        rows: u16,
        #[serde(default)]
        cell_width_px: u32,
        #[serde(default)]
        cell_height_px: u32,
    },
    #[serde(rename = "terminal.scroll")]
    Scroll {
        direction: TerminalControlScrollDirection,
        lines: u16,
        #[serde(default)]
        source: TerminalControlScrollSource,
        #[serde(default)]
        column: Option<u16>,
        #[serde(default)]
        row: Option<u16>,
        #[serde(default)]
        modifiers: u8,
    },
    #[serde(rename = "terminal.release")]
    Release {},
}

pub(super) enum TerminalAppCommand {
    Input(Vec<u8>),
    Resize(PtySize),
    Release,
}

pub(super) fn terminal_app_command_from_json(raw: &str) -> Result<TerminalAppCommand, String> {
    let command = serde_json::from_str::<TerminalControlCommand>(raw)
        .map_err(|err| format!("invalid json command: {err}"))?;
    match command {
        TerminalControlCommand::Input { text, bytes } => {
            terminal_input_bytes(text, bytes).map(TerminalAppCommand::Input)
        }
        TerminalControlCommand::Resize {
            cols,
            rows,
            cell_width_px,
            cell_height_px,
        } => {
            if cols == 0 || rows == 0 {
                return Err("terminal.resize cols and rows must be greater than 0".into());
            }
            Ok(TerminalAppCommand::Resize(PtySize {
                rows,
                cols,
                pixel_width: cell_width_px.min(u32::from(u16::MAX)) as u16,
                pixel_height: cell_height_px.min(u32::from(u16::MAX)) as u16,
            }))
        }
        TerminalControlCommand::Scroll {
            direction,
            lines,
            source,
            column,
            row,
            modifiers,
        } => {
            if lines == 0 {
                return Err("terminal.scroll lines must be greater than 0".into());
            }
            Ok(TerminalAppCommand::Input(terminal_app_scroll_bytes(
                direction, lines, source, column, row, modifiers,
            )))
        }
        TerminalControlCommand::Release {} => Ok(TerminalAppCommand::Release),
    }
}

fn terminal_input_bytes(text: Option<String>, bytes: Option<String>) -> Result<Vec<u8>, String> {
    match (text, bytes) {
        (Some(_), Some(_)) => Err("terminal.input accepts text or bytes, not both".into()),
        (Some(text), None) => Ok(text.into_bytes()),
        (None, Some(bytes)) => base64::engine::general_purpose::STANDARD
            .decode(bytes)
            .map_err(|err| format!("invalid terminal.input bytes: {err}")),
        (None, None) => Ok(Vec::new()),
    }
}

fn terminal_app_scroll_bytes(
    direction: TerminalControlScrollDirection,
    lines: u16,
    source: TerminalControlScrollSource,
    column: Option<u16>,
    row: Option<u16>,
    modifiers: u8,
) -> Vec<u8> {
    if matches!(source, TerminalControlScrollSource::PageKey) {
        let sequence: &[u8] = match direction {
            TerminalControlScrollDirection::Up => b"\x1b[5~",
            TerminalControlScrollDirection::Down => b"\x1b[6~",
        };
        return sequence.repeat(usize::from(lines));
    }

    let mut button = match direction {
        TerminalControlScrollDirection::Up => 64,
        TerminalControlScrollDirection::Down => 65,
    };
    if modifiers & 1 != 0 {
        button += 4;
    }
    if modifiers & 4 != 0 {
        button += 8;
    }
    if modifiers & 2 != 0 {
        button += 16;
    }
    let x = column.unwrap_or(0).saturating_add(1);
    let y = row.unwrap_or(0).saturating_add(1);
    format!("\x1b[<{button};{x};{y}M")
        .repeat(usize::from(lines))
        .into_bytes()
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum TerminalControlScrollDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum TerminalControlScrollSource {
    #[default]
    Wheel,
    PageKey,
}

pub(super) fn terminal_control_command_from_json(raw: &str) -> Result<ClientMessage, String> {
    let command = serde_json::from_str::<TerminalControlCommand>(raw)
        .map_err(|err| format!("invalid json command: {err}"))?;
    match command {
        TerminalControlCommand::Input { text, bytes } => {
            let data = match (text, bytes) {
                (Some(_), Some(_)) => {
                    return Err("terminal.input accepts text or bytes, not both".into())
                }
                (Some(text), None) => text.into_bytes(),
                (None, Some(bytes)) => base64::engine::general_purpose::STANDARD
                    .decode(bytes)
                    .map_err(|err| format!("invalid terminal.input bytes: {err}"))?,
                (None, None) => Vec::new(),
            };
            Ok(ClientMessage::Input { data })
        }
        TerminalControlCommand::Resize {
            cols,
            rows,
            cell_width_px,
            cell_height_px,
        } => {
            if cols == 0 || rows == 0 {
                return Err("terminal.resize cols and rows must be greater than 0".into());
            }
            Ok(ClientMessage::Resize {
                cols,
                rows,
                cell_width_px,
                cell_height_px,
                pixel_mouse: false,
            })
        }
        TerminalControlCommand::Scroll {
            direction,
            lines,
            source,
            column,
            row,
            modifiers,
        } => {
            if lines == 0 {
                return Err("terminal.scroll lines must be greater than 0".into());
            }
            let direction = match direction {
                TerminalControlScrollDirection::Up => AttachScrollDirection::Up,
                TerminalControlScrollDirection::Down => AttachScrollDirection::Down,
            };
            let source = match source {
                TerminalControlScrollSource::Wheel => AttachScrollSource::Wheel,
                TerminalControlScrollSource::PageKey => AttachScrollSource::PageKey {
                    input: match direction {
                        AttachScrollDirection::Up => b"\x1b[5~".to_vec(),
                        AttachScrollDirection::Down => b"\x1b[6~".to_vec(),
                    },
                },
            };
            Ok(ClientMessage::AttachScroll {
                source,
                direction,
                lines,
                column,
                row,
                modifiers,
            })
        }
        TerminalControlCommand::Release {} => Ok(ClientMessage::Detach),
    }
}
