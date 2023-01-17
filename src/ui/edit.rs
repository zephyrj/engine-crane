/*
 * Copyright (c):
 * 2022 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

/*
 * Copyright (c):
 * 2022 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

use super::{Message, Tab};
use std::path::{PathBuf};
use iced::{Element};
use iced::widget::{Container, pick_list, Text};
use iced_aw::{TabLabel};
use crate::ui::{ApplicationData, ListPath};


pub struct EditTab {
    status_message: String,
    current_car: Option<PathBuf>
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    CarSelected(String)
}

impl EditTab {
    pub(crate) fn new() -> Self {
        EditTab {
            status_message: String::new(),
            current_car: None
        }
    }

    pub fn update(&mut self, message: EditMessage, app_data: &ApplicationData) {
        match message {
            EditMessage::CarSelected(_) => {}
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData) {
    }
}

impl Tab for EditTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Edit Car")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content<'a, 'b>(
        &'a self,
        app_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a
    {
        let content : Element<'_, EditMessage> = Container::new(Text::new("Edit")).into();
        content.map(Message::Edit)
    }
}

