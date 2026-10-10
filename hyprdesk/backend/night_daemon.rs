// Night daemon — holds the screen gamma for the session and takes new values over a socket.
// Daemon de noche — mantiene el gamma de la pantalla durante la sesión y recibe valores nuevos por un socket.

use std::fs;
use std::io::{self, BufRead, BufReader, Seek, Write};
use std::os::fd::AsFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use wayland_client::protocol::{wl_output::WlOutput, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1;
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_v1::{self, ZwlrGammaControlV1};

pub const FLAG: &str = "--night-daemon";

const NEUTRAL_K: u32 = 6500;

fn runtime_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from)
}

// One socket and pid file per session / Un socket y un fichero de pid por sesión
fn session_file(ext: &str) -> PathBuf {
    let session = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .or_else(|_| std::env::var("WAYLAND_DISPLAY"))
        .unwrap_or_else(|_| "x".into());
    runtime_dir().join(format!("hyprdesk-night-{session}.{ext}"))
}

fn socket_path() -> PathBuf {
    session_file("sock")
}

fn pid_path() -> PathBuf {
    session_file("pid")
}

fn forget_files() {
    let _ = fs::remove_file(socket_path());
    let _ = fs::remove_file(pid_path());
}

fn request(line: &str) -> Option<String> {
    let mut stream = UnixStream::connect(socket_path()).ok()?;
    // If it takes more than a second it is hung / Si tarda más de un segundo es que está colgado
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    writeln!(stream, "{line}").ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    (!reply.trim().is_empty()).then(|| reply.trim().to_string())
}

// None means there is no daemon, false means it holds no output / None es que no hay daemon, false es que no tiene ninguna salida
pub fn set(temp_k: u32, brightness_pct: u32) -> Option<bool> {
    request(&format!("set {temp_k} {brightness_pct}")).map(|reply| reply == "ok")
}

// Rules out zombies and recycled pids / Descarta zombis y pids reciclados
fn is_daemon(pid: u32) -> bool {
    pid != std::process::id()
        && fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|c| c.split(|b| *b == 0).any(|arg| arg == FLAG.as_bytes()))
}

fn recorded_daemon() -> Option<u32> {
    let pid = fs::read_to_string(pid_path()).ok()?.trim().parse().ok()?;
    is_daemon(pid).then_some(pid)
}

// Stops the daemon and waits until the gamma is free / Para el daemon y espera a que el gamma quede libre
pub fn stop() {
    let recorded = recorded_daemon();
    let answered = request("quit").is_some();
    if let Some(pid) = recorded {
        let signal = |name: &str| {
            let _ = Command::new("kill").args([name, &pid.to_string()]).output();
        };
        for round in 0..80 {
            if !is_daemon(pid) {
                break;
            }
            // If it did not answer it is killed right away / Si no contestó se mata al momento
            if (round == 0 && !answered) || round == 40 {
                signal("-KILL");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    forget_files();
    if answered || recorded.is_some() {
        std::thread::sleep(Duration::from_millis(100));
    }
}

// Colour for that temperature compared with the 6500K white / Color de esa temperatura comparado con el blanco de 6500K
fn white_point(temp_k: u32) -> (f64, f64, f64) {
    let green = |t: f64| 99.470_802_586_1 * t.ln() - 161.119_568_166_1;
    let blue = |t: f64| if t <= 19.0 { 0.0 } else { 138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7 };
    let t = f64::from(temp_k.clamp(1000, NEUTRAL_K)) / 100.0;
    let neutral = f64::from(NEUTRAL_K) / 100.0;
    (1.0, (green(t) / green(neutral)).clamp(0.0, 1.0), (blue(t) / blue(neutral)).clamp(0.0, 1.0))
}

// The red, green and blue ramps one after another / Las rampas de rojo, verde y azul una detrás de otra
fn ramps(size: usize, temp_k: u32, brightness_pct: u32) -> Vec<u16> {
    let (r, g, b) = white_point(temp_k);
    let level = f64::from(brightness_pct.clamp(10, 100)) / 100.0;
    let last = size.saturating_sub(1).max(1) as f64;
    [r, g, b]
        .iter()
        .flat_map(|channel| (0..size).map(move |i| (i as f64 / last * channel * level * 65535.0).round() as u16))
        .collect()
}

// The compositor only needs the descriptor, so the file is deleted right away / El compositor solo necesita el descriptor, así que el fichero se borra al momento
fn ramp_file(table: &[u16]) -> io::Result<fs::File> {
    let path = runtime_dir().join(format!("hyprdesk-night-ramp-{}", std::process::id()));
    let _ = fs::remove_file(&path);
    let mut file = fs::OpenOptions::new().read(true).write(true).create_new(true).open(&path)?;
    let _ = fs::remove_file(&path);
    let bytes: Vec<u8> = table.iter().flat_map(|v| v.to_ne_bytes()).collect();
    file.write_all(&bytes)?;
    file.rewind()?;
    Ok(file)
}

struct Output {
    name: u32,
    output: WlOutput,
    control: Option<ZwlrGammaControlV1>,
    size: usize,
    refusals: u8,
}

// Outputs without gamma, like virtual ones, always refuse / Las salidas sin gamma, como las virtuales, se niegan siempre
const MAX_REFUSALS: u8 = 3;

struct Shared {
    temp_k: u32,
    brightness_pct: u32,
    manager: Option<ZwlrGammaControlManagerV1>,
    outputs: Vec<Output>,
}

// Keeps working after a panic in another thread / Sigue funcionando tras un pánico en otro hilo
fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    fn holds_any(&self) -> bool {
        self.outputs.iter().any(|o| o.control.is_some() && o.size > 0)
    }

    fn attach(&mut self, qh: &QueueHandle<State>) {
        let Some(manager) = &self.manager else { return };
        for out in self.outputs.iter_mut().filter(|o| o.control.is_none() && o.refusals < MAX_REFUSALS) {
            out.control = Some(manager.get_gamma_control(&out.output, qh, out.name));
        }
    }

    fn apply(&self, out: &Output) {
        let Some(control) = &out.control else { return };
        if out.size == 0 {
            return;
        }
        match ramp_file(&ramps(out.size, self.temp_k, self.brightness_pct)) {
            Ok(file) => control.set_gamma(file.as_fd()),
            Err(e) => eprintln!("hyprdesk night daemon: ramp for output {} failed: {e}", out.name),
        }
    }

    fn apply_all(&self) {
        for out in &self.outputs {
            self.apply(out);
        }
    }

    fn release_all(&mut self) {
        for out in self.outputs.drain(..) {
            if let Some(control) = out.control {
                control.destroy();
            }
        }
    }
}

struct State {
    shared: Arc<Mutex<Shared>>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let mut shared = lock(&state.shared);
        match event {
            wl_registry::Event::Global { name, interface, .. } => match interface.as_str() {
                "wl_output" => {
                    let output = registry.bind::<WlOutput, _, _>(name, 1, qh, ());
                    shared.outputs.push(Output { name, output, control: None, size: 0, refusals: 0 });
                    shared.attach(qh);
                }
                "zwlr_gamma_control_manager_v1" => {
                    shared.manager = Some(registry.bind::<ZwlrGammaControlManagerV1, _, _>(name, 1, qh, ()));
                    shared.attach(qh);
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(i) = shared.outputs.iter().position(|o| o.name == name) {
                    if let Some(control) = shared.outputs.remove(i).control {
                        control.destroy();
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrGammaControlV1, u32> for State {
    fn event(
        state: &mut Self,
        control: &ZwlrGammaControlV1,
        event: zwlr_gamma_control_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let mut shared = lock(&state.shared);
        let Some(i) = shared.outputs.iter().position(|o| o.name == *name) else { return };
        match event {
            zwlr_gamma_control_v1::Event::GammaSize { size } => {
                shared.outputs[i].size = size as usize;
                shared.apply(&shared.outputs[i]);
            }
            // Another client has it or it is gone, so it is asked again on the next change / Lo tiene otro cliente o ya no existe, así que se vuelve a pedir en el siguiente cambio
            zwlr_gamma_control_v1::Event::Failed => {
                eprintln!("hyprdesk night daemon: gamma control of output {name} failed");
                control.destroy();
                shared.outputs[i].control = None;
                shared.outputs[i].size = 0;
                shared.outputs[i].refusals += 1;
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for State {
    fn event(_: &mut Self, _: &WlOutput, _: <WlOutput as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrGammaControlManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwlrGammaControlManagerV1,
        _: <ZwlrGammaControlManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Never two daemons at once. A second start hands its values to the first and leaves / Nunca dos daemons a la vez. Un segundo arranque le pasa sus valores al primero y se va
fn listen(temp_k: u32, brightness_pct: u32) -> io::Result<Option<UnixListener>> {
    let mut last = io::Error::other("no attempt");
    for _ in 0..3 {
        if set(temp_k, brightness_pct).is_some() {
            return Ok(None);
        }
        match UnixListener::bind(socket_path()) {
            Ok(listener) => return Ok(Some(listener)),
            Err(e) => last = e,
        }
        // Another start won or the socket is a leftover / Otro arranque ganó o el socket es un resto
        if set(temp_k, brightness_pct).is_some() {
            return Ok(None);
        }
        stop();
    }
    Err(last)
}

fn serve(listener: UnixListener, shared: Arc<Mutex<Shared>>, conn: Connection, qh: QueueHandle<State>) {
    for stream in listener.incoming().flatten() {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            continue;
        }
        let mut words = line.split_whitespace();
        match words.next() {
            Some("set") => {
                let (Some(temp_k), Some(brightness_pct)) = (
                    words.next().and_then(|w| w.parse().ok()),
                    words.next().and_then(|w| w.parse().ok()),
                ) else {
                    continue;
                };
                let applied = {
                    let mut shared = lock(&shared);
                    shared.temp_k = temp_k;
                    shared.brightness_pct = brightness_pct;
                    shared.attach(&qh);
                    shared.apply_all();
                    shared.outputs.is_empty() || shared.holds_any()
                };
                let _ = conn.flush();
                let _ = writeln!(&stream, "{}", if applied { "ok" } else { "failed" });
            }
            Some("quit") => {
                lock(&shared).release_all();
                let _ = conn.flush();
                forget_files();
                let _ = writeln!(&stream, "ok");
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

// Runs until the session ends or it is told to quit / Corre hasta que acaba la sesión o se le dice que pare
pub fn run(args: &[String]) -> i32 {
    let value = |i: usize, default: u32| args.get(i).and_then(|a| a.parse().ok()).unwrap_or(default);
    let (temp_k, brightness_pct) = (value(0, NEUTRAL_K), value(1, 100));

    let listener = match listen(temp_k, brightness_pct) {
        Ok(Some(listener)) => listener,
        Ok(None) => return 0,
        Err(e) => {
            eprintln!("hyprdesk night daemon: socket {}: {e}", socket_path().display());
            return 1;
        }
    };
    let _ = fs::write(pid_path(), std::process::id().to_string());
    let fail = |msg: String| {
        eprintln!("hyprdesk night daemon: {msg}");
        forget_files();
        1
    };

    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(e) => return fail(format!("no Wayland display: {e}")),
    };
    let shared = Arc::new(Mutex::new(Shared { temp_k, brightness_pct, manager: None, outputs: Vec::new() }));
    let mut state = State { shared: shared.clone() };
    let mut queue = conn.new_event_queue();
    conn.display().get_registry(&queue.handle(), ());
    if let Err(e) = queue.roundtrip(&mut state) {
        return fail(format!("registry: {e}"));
    }
    if lock(&shared).manager.is_none() {
        return fail("the compositor has no gamma control".into());
    }
    if let Err(e) = queue.roundtrip(&mut state) {
        return fail(format!("outputs: {e}"));
    }
    let refused = {
        let shared = lock(&shared);
        !shared.outputs.is_empty() && !shared.holds_any()
    };
    if refused {
        return fail("no output gave its gamma, another program holds it".into());
    }

    let (for_socket, socket_conn, qh) = (shared.clone(), conn.clone(), queue.handle());
    std::thread::spawn(move || serve(listener, for_socket, socket_conn, qh));

    while queue.blocking_dispatch(&mut state).is_ok() {}
    forget_files();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_light_leaves_the_screen_untouched() {
        assert_eq!(white_point(6500), (1.0, 1.0, 1.0));
        assert_eq!(ramps(256, 6500, 100).last(), Some(&65535));
        assert!(ramps(256, 6500, 100).iter().step_by(256).all(|v| *v == 0));
    }

    #[test]
    fn warmer_light_takes_blue_first() {
        let (r, g, b) = white_point(3000);
        assert!(r > g && g > b && b > 0.0, "{r} {g} {b}");
        let (_, warmer_g, warmer_b) = white_point(2000);
        assert!(warmer_g < g && warmer_b < b);
    }

    #[test]
    fn ramps_rise_and_follow_the_brightness() {
        let table = ramps(1024, 4000, 50);
        assert_eq!(table.len(), 3 * 1024);
        for channel in table.chunks(1024) {
            assert!(channel.windows(2).all(|w| w[0] <= w[1]));
        }
        // Red stays whole, so it tops out at the brightness / El rojo queda entero, así que llega hasta el brillo
        assert_eq!(table[1023], 32768);
    }
}
