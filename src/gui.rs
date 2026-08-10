use crate::{audio_engine::AudioEngine, config::GUIConfig};
use iced::{
    Center, ContentFit, Element,
    Length::{Fill, FillPortion},
    Result,
    alignment::Horizontal::Left,
    widget,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct State {
    config: GUIConfig,
    audio_engine: Arc<Mutex<AudioEngine>>,
}

#[derive(Debug, Clone)]
enum Message {
    Play(PathBuf),
    Stop,
    Volume(u32),
}

impl State {
    fn update(&mut self, message: Message) {
        let mut guard = self
            .audio_engine
            .lock()
            .expect("Failed to aquire audio engine mutex");
        match message {
            Message::Play(path) => guard.play(&path).unwrap_or_else(|err| {
                eprintln!("Error playing sound {}: {}", path.to_string_lossy(), err);
            }),
            Message::Stop => guard.stop_all(),
            Message::Volume(volume) => guard.set_volume(volume as f32 / 100.0),
        }
    }

    fn view(&self) -> widget::Column<'_, Message> {
        let mut guard = self
            .audio_engine
            .lock()
            .expect("Failed to aquire audio engine mutex");
        let volume = guard.get_volume();
        widget::column![
            widget::row![
                widget::button(widget::text("stop all").width(Fill).height(Fill).center())
                    .on_press(Message::Stop)
                    .width(FillPortion(2))
                    .height(Fill),
                widget::button(
                    widget::text("reset volume")
                        .width(Fill)
                        .height(Fill)
                        .center()
                )
                .on_press(Message::Volume(100))
                .width(FillPortion(2))
                .height(Fill),
                widget::slider(1..=300, (volume * 100.0) as u32, Message::Volume)
                    .default(100u32)
                    .shift_step(5u32)
                    .width(FillPortion(3)),
                widget::text(format!("{:.2}", volume))
                    .height(Fill)
                    .align_x(Left)
                    .align_y(Center),
            ]
            .spacing(5)
            .width(Fill)
            .height(FillPortion(1))
            .align_y(Center),
        ]
        .extend(self.config.buttons.iter().map(|row| {
            widget::Row::with_children(row.iter().map(|button| {
                Element::from(
                    widget::button(
                        widget::Column::with_children({
                            let name = Element::from(
                                widget::text(button.name.clone())
                                    .width(Fill)
                                    .height(FillPortion(1))
                                    .center(),
                            );
                            match button.image.clone() {
                                Some(path) => vec![
                                    Element::from(
                                        widget::image(path)
                                            .width(Fill)
                                            .height(FillPortion(3))
                                            .expand(true)
                                            .content_fit(ContentFit::Cover),
                                    ),
                                    name,
                                ],
                                None => vec![name],
                            }
                        })
                        .width(Fill)
                        .height(Fill)
                        .align_x(Center),
                    )
                    .on_press(Message::Play(button.sound.clone()))
                    .width(Fill)
                    .height(Fill),
                )
            }))
            .spacing(5)
            .width(Fill)
            .height(FillPortion(3))
            .align_y(Center)
            .into()
        }))
        .spacing(5)
        .padding(5)
        .width(Fill)
        .height(Fill)
        .align_x(Center)
    }
}

pub fn run_gui(config: GUIConfig, audio_engine: Arc<Mutex<AudioEngine>>) -> Result {
    let app = iced::application(
        move || State {
            audio_engine: audio_engine.clone(),
            config: config.clone(),
        },
        State::update,
        State::view,
    );
    app.run()
}
