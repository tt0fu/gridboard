use anyhow::Result;
use crate::{
    audio_engine::AudioEngine,
    protocol::{DEFAULT_ADDRESS, Request, Response},
};
use serde_json::{from_str, to_string};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
};

pub async fn run_ipc_server(audio_engine: Arc<Mutex<AudioEngine>>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(DEFAULT_ADDRESS).await?;
    println!("Listening for IPC calls on {}", DEFAULT_ADDRESS);

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Incoming connection from {}", addr);

        let engine = audio_engine.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, engine).await {
                eprintln!("Client error: {}", e);
            }
        });
    }
}

async fn handle_client(stream: TcpStream, audio_engine: Arc<Mutex<AudioEngine>>) -> Result<()> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut request_string = String::new();

    while reader.read_line(&mut request_string).await? > 0 {
        let request: Request = match from_str(&request_string) {
            Ok(req) => req,
            Err(e) => {
                let resp = Response::Error(format!("Invalid JSON: {}", e));
                writer.write_all(to_string(&resp)?.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                request_string.clear();
                continue;
            }
        };

        let response = {
            let mut engine = audio_engine
                .lock()
                .expect("Failed to aquire audio engine mutex");
            match request {
                Request::Play { ref path } => {
                    engine.play(path);
                    Response::Ok
                }
                Request::StopAll => {
                    engine.stop_all();
                    Response::Ok
                }
                Request::GetVolume => Response::Volume(engine.get_volume()),
                Request::SetVolume { volume } => {
                    engine.set_volume(volume);
                    Response::Ok
                }
            }
        };
        let response_string = serde_json::to_string(&response)?;

        writer.write_all(response_string.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        request_string.clear();
    }
    Ok(())
}
