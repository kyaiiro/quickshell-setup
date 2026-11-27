use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn socket_path(name: &str) -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(PathBuf::from(runtime).join("hypr").join(sig).join(name))
}

pub fn command(cmd: &str) -> Option<String> {
    let mut stream = UnixStream::connect(socket_path(".socket.sock")?).ok()?;
    stream.write_all(cmd.as_bytes()).ok()?;
    let mut out = String::new();
    stream.read_to_string(&mut out).ok()?;
    Some(out)
}

pub fn events() -> Option<impl Iterator<Item = (String, String)>> {
    let stream = UnixStream::connect(socket_path(".socket2.sock")?).ok()?;
    Some(
        BufReader::new(stream)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| {
                let (name, data) = line.split_once(">>")?;
                Some((name.to_string(), data.to_string()))
            }),
    )
}