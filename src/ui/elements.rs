/*
 * Copyright (c):
 * 2023 zephyrj
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

use iced::Alignment;
use iced::widget::{Column, Text};
use iced_native::widget::{pick_list};
use crate::ui::Message;

pub fn create_drop_down_list<'a>(title: &'static str,
                             options: &'a Vec<String>,
                             current_selection: Option<String>,
                             on_select: fn(String) -> Message) -> Column<'a, Message> {
    let picklist = pick_list(options, current_selection, on_select);
    Column::new()
        .align_items(Alignment::Start)
        .push(Text::new(title))
        .push(picklist)
}