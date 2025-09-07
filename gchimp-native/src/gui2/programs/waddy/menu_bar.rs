use iced::{widget::button, Element};
use iced_aw::{menu::Item, menu_items};

use crate::{gui2::programs::waddy::WaddyMessage, menu_submenu_button};

#[derive(Debug, Clone)]
pub struct MenuBar {
    texture_fit: bool,
}

#[derive(Debug, Clone)]
pub enum MenuBarMessage {
    None,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self { texture_fit: true }
    }
}

impl MenuBar {
    pub fn view(&'_ self) -> Element<'_, WaddyMessage> {
        // let menu_tpl_1 = |items| iced_aw::Menu::new(items).max_width(180.0).offset(15.0).spacing(5.0);
        let menu_tpl_2 = |items| {
            iced_aw::Menu::new(items)
                .max_width(180.0)
                .offset(0.0)
                .spacing(5.0)
        };

        let button1 = menu_submenu_button!("hello8", WaddyMessage::None);

        let menu1 = menu_tpl_2(menu_items!((menu_submenu_button!(
            "aa",
            WaddyMessage::None
        ))(button("hello2"))(button("hello3"))));

        let menu2 = menu_tpl_2(menu_items!((button("hello4"))(button("hello5"))(button(
            "hello6"
        ))));

        let menu3 = menu_tpl_2(menu_items!((button("hello7"))(button1, menu2)(button(
            "hello9"
        ))));

        let menu = iced_aw::menu_bar!((button("text1"), menu1)(button("text2"), menu3));

        menu.into()
    }
}
