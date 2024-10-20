use std::{fmt::Display, io, path::PathBuf, sync::Arc};

use iced::{
    color, mouse,
    widget::{
        button, canvas, column, container, horizontal_rule, horizontal_space, row, rule, svg, text,
        tooltip, vertical_rule, Rule,
    },
    Alignment, Color, Element, Font,
    Length::{self},
    Padding, Pixels, Point, Rectangle, Renderer, Settings, Task, Theme,
};
use iced_aw::{style::Status, SelectionList};

pub fn main() -> iced::Result {
    iced::application("roygbiv", Roygbiv::update, Roygbiv::view)
        .theme(|_| Theme::CatppuccinMocha)
        .settings(Settings {
            default_text_size: Pixels(14.0),
            ..Default::default()
        })
        .run_with(|| {
            (
                Roygbiv {
                    canvas_state: CanvasState::default(),
                    canvas_width: 1280.,
                    canvas_height: 720.,

                    audio_file_path: None,
                    audio_file_contents: vec![],
                    is_loading_file: false,

                    layers: vec![],
                    layer_names: vec![],
                    selected_layer_index: 0,
                },
                Task::none(),
            )
        })
}

// #[derive(Default)]
struct Roygbiv {
    canvas_state: CanvasState,
    canvas_width: f32,
    canvas_height: f32,

    audio_file_path: Option<PathBuf>,
    audio_file_contents: Vec<u8>,
    is_loading_file: bool,

    layers: Vec<Layer>,
    layer_names: Vec<String>,
    selected_layer_index: usize,
}

#[derive(Debug, Clone)]
enum Message {
    SetCanvasSize(f32, f32),

    OpenAudioFile,
    RemoveAudioFile,
    AudioFileOpened(Result<(PathBuf, Arc<Vec<u8>>), Error>),

    AddImageLayer,
    RemoveLayer(usize),
    ImageFileOpened(Result<(PathBuf, Arc<Vec<u8>>), Error>),
    LayerSelected(usize, String),
}

#[derive(Debug, Clone)]
pub enum Error {
    DialogClosed,
    IoError(io::ErrorKind),
}

async fn open_audio_file() -> Result<(PathBuf, Arc<Vec<u8>>), Error> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Open audio file...")
        .add_filter("Audio file", &["wav", "mp3", "flac"])
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    load_file(picked_file).await
}

async fn open_image_file() -> Result<(PathBuf, Arc<Vec<u8>>), Error> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Open image file...")
        .add_filter("Image file", &["png", "jpeg", "jpg", "webp"])
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    load_file(picked_file).await
}

async fn load_file(path: impl Into<PathBuf>) -> Result<(PathBuf, Arc<Vec<u8>>), Error> {
    let path = path.into();

    let contents = tokio::fs::read(&path)
        .await
        .map(Arc::new)
        .map_err(|error| Error::IoError(error.kind()))?;

    Ok((path, contents))
}

impl Roygbiv {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetCanvasSize(width, height) => {
                self.canvas_width = width;
                self.canvas_height = height;

                Task::none()
            }
            Message::OpenAudioFile => {
                if self.is_loading_file {
                    Task::none()
                } else {
                    self.is_loading_file = true;

                    Task::perform(open_audio_file(), Message::AudioFileOpened)
                }
            }
            Message::RemoveAudioFile => {
                self.is_loading_file = false;

                self.audio_file_path = None;
                self.audio_file_contents = vec![];

                Task::none()
            }
            Message::AudioFileOpened(result) => {
                self.is_loading_file = false;

                if let Ok((path, contents)) = result {
                    self.audio_file_path = Some(path);
                    self.audio_file_contents = contents.to_vec();
                }

                Task::none()
            }
            Message::AddImageLayer => Task::perform(open_image_file(), Message::ImageFileOpened),
            Message::RemoveLayer(index) => {
                let _ = &self.layers.remove(index);
                self.update_layer_names();

                Task::none()
            }
            Message::ImageFileOpened(result) => {
                if let Ok((path, contents)) = result {
                    let layers_length = &self.layers.len();
                    let file_name = if let Some(file_name) = path.file_name() {
                        file_name.to_str()
                    } else {
                        path.to_str()
                    }
                    .unwrap_or("Unnamed");
                    let layer = Layer {
                        id: format!("{}", layers_length),
                        name: format!("{}", file_name),
                        x: 0.,
                        y: 0.,
                        width: &self.canvas_width - 20.,
                        height: &self.canvas_height - 20.,
                        scale: 1.,
                        opacity: 1.,
                        contents: contents.to_vec(),
                    };
                    let _ = &self.layers.push(layer);
                    self.update_layer_names();
                }

                Task::none()
            }
            Message::LayerSelected(index, _) => {
                self.selected_layer_index = index;

                Task::none()
            }
        }
    }

    fn update_layer_names(&mut self) {
        self.layer_names = self.layers.iter().map(|layer| layer.name.clone()).collect()
    }

    fn layer_settings_view(&self, layer: Option<&Layer>) -> Element<Message> {
        if let Some(layer) = layer {
            column![
                text(format!("x: {}", layer.x)),
                text(format!("y: {}", layer.y)),
                text(format!("width: {}", layer.width)),
                text(format!("height: {}", layer.height)),
                text(format!("scale: {}", layer.scale)),
                text(format!("opacity: {}", layer.opacity)),
            ]
            .height(Length::Fill)
            .padding([6., 7.])
            .spacing(6.)
            .into()
        } else {
            container("No layer selected").center(Length::Fill).into()
        }
    }

    fn view(&self) -> Element<Message> {
        let audio_section_content = {
            match &self.audio_file_path {
                Some(path) => container({
                    let name = (path.file_name().unwrap_or(path.as_os_str())).to_str();

                    row![
                        text(name.unwrap_or("Audio file")),
                        horizontal_space(),
                        button("Remove audio file").on_press(Message::RemoveAudioFile)
                    ]
                    .align_y(Alignment::Center)
                }),
                None => container({
                    let select_file_button = button("Select audio file");
                    if !self.is_loading_file {
                        select_file_button.on_press(Message::OpenAudioFile)
                    } else {
                        select_file_button
                    }
                }),
            }
        };

        let audio_section = container(audio_section_content)
            .width(Length::Fill)
            .padding(Padding::from([6., 7.]));

        let canvas_section = container(
            canvas(&self.canvas_state)
                .width(Length::Fixed(self.canvas_width))
                .height(Length::Fixed(self.canvas_height)),
        )
        .center(Length::Fill);

        let main_column = column![canvas_section, horizontal_separator(), audio_section]
            .width(Length::FillPortion(2));

        let selected_layer = self.layers.get(self.selected_layer_index);

        let layer_selection_list = SelectionList::new_with(
            &self.layer_names,
            Message::LayerSelected,
            14.,
            [6., 7.],
            |theme: &Theme, status: Status| {
                let base = iced_aw::style::selection_list::Style::default();
                let palette = theme.extended_palette();

                match status {
                    Status::Hovered => iced_aw::style::selection_list::Style {
                        text_color: palette.secondary.weak.text.into(),
                        background: palette.secondary.weak.color.into(),
                        border_width: 0.,
                        ..base
                    },
                    Status::Selected => iced_aw::style::selection_list::Style {
                        text_color: palette.primary.weak.text.into(),
                        background: palette.primary.weak.color.into(),
                        border_width: 0.,
                        ..base
                    },
                    _ => iced_aw::style::selection_list::Style {
                        text_color: palette.background.base.text.into(),
                        background: palette.background.base.color.into(),
                        border_width: 0.,
                        ..base
                    },
                }
            },
            Some(self.selected_layer_index),
            Font::default(),
        );

        let layer_list_section = column![
            container("Layers").padding(Padding::from([6., 7.])),
            horizontal_separator(),
            container(layer_selection_list).center(Length::Fill),
            horizontal_separator(),
            container(
                row![
                    icon_button_with_tooltip("plus", "Add new layer", Some(Message::AddImageLayer)),
                    icon_button_with_tooltip(
                        "trash",
                        "Delete layer",
                        match selected_layer {
                            Some(_) => Some(Message::RemoveLayer(self.selected_layer_index)),
                            None => None,
                        }
                    )
                ]
                .spacing(6.)
            )
            .padding(Padding::from([6., 7.]))
        ]
        .height(Length::Fill);

        let selected_layer_settings_section = column![
            container(text(match selected_layer {
                Some(layer) => &layer.name,
                None => "Layer settings",
            }))
            .padding(Padding::from([6., 7.])),
            horizontal_separator(),
            self.layer_settings_view(selected_layer),
        ];

        let settings_column = column![
            selected_layer_settings_section,
            horizontal_separator(),
            layer_list_section
        ]
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        row![main_column, vertical_separator(), settings_column].into()
    }
}

#[derive(Debug)]
struct Layer {
    id: String,
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
    opacity: f32,
    contents: Vec<u8>,
}

impl Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug)]
struct CanvasState {
    background_cache: canvas::Cache,
}

impl CanvasState {
    pub fn new() -> CanvasState {
        CanvasState {
            background_cache: canvas::Cache::default(),
        }
    }

    // pub fn update(&mut self) {}
}

impl<Message> canvas::Program<Message> for CanvasState {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let background = self
            .background_cache
            .draw(renderer, bounds.size(), |frame| {
                frame.fill_rectangle(Point::ORIGIN, frame.size(), Color::BLACK);
            });

        vec![background]
    }
}

impl Default for CanvasState {
    fn default() -> Self {
        Self::new()
    }
}

fn icon(name: &str) -> svg::Handle {
    svg::Handle::from_path(format!(
        "{}/src/icons/{}.svg",
        env!("CARGO_MANIFEST_DIR"),
        name
    ))
}

fn horizontal_separator<'a>() -> Rule<'a> {
    horizontal_rule(1.).style(|theme: &Theme| {
        let palette = theme.extended_palette();
        rule::Style {
            color: palette.background.weak.color.into(),
            ..rule::default(theme)
        }
    })
}

fn vertical_separator<'a>() -> Rule<'a> {
    vertical_rule(1.).style(|theme: &Theme| {
        let palette = theme.extended_palette();
        rule::Style {
            color: palette.background.weak.color.into(),
            ..rule::default(theme)
        }
    })
}

fn icon_button_with_tooltip<'a, Message: Clone + 'a>(
    icon_name: &'a str,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let action = button(container(
        svg(icon(icon_name))
            .width(18.)
            .height(18.)
            .style(|_, _| svg::Style {
                color: Some(color!(0xffffff)),
            }),
    ))
    .padding(5.);
    if let Some(on_press) = on_press {
        tooltip(action.on_press(on_press), label, tooltip::Position::Top)
            .style(container::rounded_box)
            .into()
    } else {
        action.style(button::secondary).into()
    }
}
