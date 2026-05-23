use gridboard::protocol::{DEFAULT_ADDRESS, Request, Response};
use std::{
    env::args,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::PathBuf,
};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = args().collect();

    if args.len() < 2 {
        eprintln!("Usage: gridboardctl <command>");
        eprintln!("\nCommands:");
        eprintln!("play <path>        Play a sound at <path>.");
        eprintln!("stop               Stop the playback of all currently playing sounds.");
        eprintln!("volume             Get the current volume (1.0 is full volume).");
        eprintln!("volume <volume>    Set the current volume (1.0 is full volume).");
        anyhow::bail!("Invalid argument count: {}", args.len());
    }

    let request = match args[1].as_str() {
        "play" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: gridboardctl play <path>");
            }
            Request::Play {
                path: PathBuf::from(&args[2]),
            }
        }
        "stop" => Request::StopAll,
        "volume" => {
            if args.len() == 2 {
                Request::GetVolume
            } else {
                let v: f32 = args[2]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid volume"))?;
                Request::SetVolume { volume: v }
            }
        }
        other => anyhow::bail!("Unknown command: {}", other),
    };

    let mut stream = TcpStream::connect(DEFAULT_ADDRESS)?;

    let mut reader = BufReader::new(stream.try_clone()?);

    let request_string = serde_json::to_string(&request)?;
    stream.write_all(request_string.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response_string = String::new();
    reader.read_line(&mut response_string)?;

    let response: Response = serde_json::from_str(&response_string)?;

    match response {
        Response::Ok => println!("Ok"),
        Response::Volume(v) => println!("{}", v),
        Response::Error(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
