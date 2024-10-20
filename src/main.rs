use std::{
    fmt::Display,
    io::{self},
    path::PathBuf,
    sync::Arc,
};

use iced::{
    color, mouse,
    widget::{
        button, canvas, column, container, horizontal_rule, horizontal_space, image::Handle,
        responsive, row, rule, svg, text, text_input, tooltip, vertical_rule, Rule,
    },
    window::frames,
    Alignment, Color, Element, Font,
    Length::{self},
    Padding, Pixels, Point, Rectangle, Renderer, Settings, Size, Subscription, Task, Theme,
};
use iced_aw::{style::Status, SelectionList};
use image::GenericImageView;

pub fn main() -> iced::Result {
    iced::application("roygbiv", Roygbiv::update, Roygbiv::view)
        .theme(|_| Theme::CatppuccinMocha)
        .settings(Settings {
            default_text_size: Pixels(14.0),
            ..Default::default()
        })
        .subscription(Roygbiv::subscription)
        .run_with(|| {
            (
                Roygbiv {
                    canvas_state: CanvasState::default(),
                    canvas_width: 1280.,
                    canvas_height: 720.,

                    audio_file_path: None,
                    audio_file_contents: vec![],
                    is_loading_file: false,

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
    SelectLastLayer,
    Tick,
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
                let _ = &self.canvas_state.layers.remove(index);
                self.update_layer_names();

                Task::done(Message::SelectLastLayer)
            }
            Message::ImageFileOpened(result) => {
                if let Ok((path, contents)) = result {
                    let file_name = if let Some(file_name) = path.file_name() {
                        file_name.to_str()
                    } else {
                        path.to_str()
                    }
                    .unwrap_or("Unnamed");
                    let image = image::load_from_memory(&contents);
                    let image_size: Size = if let Ok(image) = image {
                        let dimensions = image.dimensions();

                        Size {
                            width: dimensions.0 as f32,
                            height: dimensions.1 as f32,
                        }
                    } else {
                        Size {
                            width: &self.canvas_width - 20.,
                            height: &self.canvas_height - 20.,
                        }
                    };
                    let layer = Layer {
                        name: format!("{}", file_name),
                        x: 0.,
                        y: 0.,
                        width: image_size.width,
                        height: image_size.height,
                        scale: 1.,
                        opacity: 1.,
                        handle: Handle::from_bytes(contents.to_vec()),
                    };
                    let _ = &self.canvas_state.layers.push(layer);
                    self.update_layer_names();
                }

                Task::done(Message::SelectLastLayer)
            }
            Message::LayerSelected(index, _string) => {
                self.selected_layer_index = index;

                Task::none()
            }
            Message::Tick => {
                self.canvas_state.update();

                Task::none()
            }
            Message::SelectLastLayer => {
                self.selected_layer_index = self.canvas_state.layers.len().max(1) - 1;

                Task::none()
            }
        }
    }

    fn update_layer_names(&mut self) {
        self.layer_names = self
            .canvas_state
            .layers
            .iter()
            .map(|layer| layer.name.clone())
            .collect()
    }

    fn layer_settings_view(&self, layer: Option<&Layer>) -> Element<Message> {
        if let Some(layer) = layer {
            column![
                column![text("x:"), text_input("x", &format!("{}", layer.x))].spacing(3.),
                column![text("y:"), text_input("y", &format!("{}", layer.y))].spacing(3.),
                column![
                    text("width:"),
                    text_input("width", &format!("{}", layer.width))
                ]
                .spacing(3.),
                column![
                    text("height:"),
                    text_input("height", &format!("{}", layer.height))
                ]
                .spacing(3.),
                column![
                    text("scale:"),
                    text_input("scale", &format!("{}", layer.scale))
                ]
                .spacing(3.),
                column![
                    text("opacity:"),
                    text_input("opacity", &format!("{}", layer.opacity))
                ]
                .spacing(3.),
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
            container(responsive(|size| {
                let canvas_width = self.canvas_width;
                let canvas_height = self.canvas_height;
                let aspect_ratio = canvas_width / canvas_height;

                let should_downsize = canvas_width > size.width;

                let final_width = if should_downsize {
                    Length::Fixed(size.width)
                } else {
                    Length::Fill
                };

                let final_height = if should_downsize {
                    Length::Fixed(size.width / aspect_ratio)
                } else {
                    Length::Fill
                };

                canvas(&self.canvas_state)
                    .width(final_width)
                    .height(final_height)
                    .into()
            }))
            .width(Length::Fixed(self.canvas_width))
            .height(Length::Fixed(self.canvas_height)),
        )
        .center(Length::Fill);

        let main_column = column![canvas_section, horizontal_separator(), audio_section]
            .width(Length::FillPortion(2));

        let selected_layer = self.canvas_state.layers.get(self.selected_layer_index);

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

    fn subscription(&self) -> Subscription<Message> {
        frames().map(|_| Message::Tick)
    }
}

#[derive(Debug)]
struct Layer {
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
    opacity: f32,
    handle: Handle,
}

impl Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug)]
struct CanvasState {
    layers: Vec<Layer>,
    background_cache: canvas::Cache,
    layers_cache: canvas::Cache,
}

impl CanvasState {
    pub fn new() -> CanvasState {
        CanvasState {
            layers: vec![],
            background_cache: canvas::Cache::default(),
            layers_cache: canvas::Cache::default(),
        }
    }

    pub fn update(&mut self) {
        self.layers_cache.clear();
    }
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
        let mut stuff: Vec<canvas::Geometry<Renderer>> = vec![];

        let bounds_size = bounds.size();

        let background = self.background_cache.draw(renderer, bounds_size, |frame| {
            frame.fill_rectangle(Point::ORIGIN, frame.size(), Color::BLACK);
        });
        stuff.push(background);

        stuff.push(self.layers_cache.draw(renderer, bounds_size, |frame| {
            for layer_index in 0..self.layers.len() {
                let layer = &self.layers.get(layer_index).unwrap();
                let aspect_ratio = layer.width / layer.height;

                let layer_width = layer.width;
                let layer_height = layer.height;

                let final_width = if layer_width > bounds_size.width {
                    bounds_size.width - 20.
                } else {
                    layer_width
                };

                let final_height = if final_width != layer_width {
                    final_width / aspect_ratio
                } else {
                    layer_height
                };

                frame.draw_image(
                    Rectangle {
                        x: layer.x,
                        y: layer.y,
                        width: final_width,
                        height: final_height,
                    },
                    &layer.handle,
                );
            }
        }));

        stuff
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
