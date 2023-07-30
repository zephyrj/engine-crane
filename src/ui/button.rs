use iced::{Color, ContentFit, Theme};
use iced::theme;
use iced::widget::{svg, Svg};
use iced_native::svg::Handle;
use iced::widget::Button;
use crate::ui::image_data::{ADD_SVG, DELETE_SVG};

pub fn create_add_button<Message>(on_press: Message) -> Button<'static, Message> {
    let img =
        Svg::new(Handle::from_memory(ADD_SVG))
            .style(theme::Svg::custom_fn(|_| { svg::Appearance{color: Some(Color::WHITE)} }))
            .content_fit(ContentFit::Fill);
    iced::widget::button(img)
        .on_press(on_press)
        .style(theme::Button::Positive)
        .padding(2)
}

pub fn create_disabled_add_button<Message>() -> Button<'static, Message> {
    let img =
        Svg::new(Handle::from_memory(ADD_SVG))
            .style(theme::Svg::custom_fn(|_| { svg::Appearance{color: Some(Color::WHITE)} }))
            .content_fit(ContentFit::Fill);
    iced::widget::button(img)
        .style(theme::Button::Positive)
        .padding(2)
}

pub fn create_delete_button<Message>(on_press: Message) -> Button<'static, Message> {
    let img =
        Svg::new(Handle::from_memory(DELETE_SVG))
            .style(theme::Svg::custom_fn(|_| { svg::Appearance{color: Some(Color::WHITE)} }))
            .content_fit(ContentFit::Fill);
    iced::widget::button(img)
        .on_press(on_press)
        .style(theme::Button::Destructive)
        .padding(2)
}
