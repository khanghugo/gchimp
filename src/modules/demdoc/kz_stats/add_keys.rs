use dem::types::{SvcTempEntity, TeTextMessage, TempEntity};

use crate::modules::demdoc::Buttons;

use super::*;

pub fn add_keys(curr: Option<&KzInfo<'_>>) -> Option<SvcTempEntity> {
    curr?;

    let curr = curr.unwrap();

    let f = if (Buttons::Forward as u16) & curr.buttons != 0 {
        "W"
    } else {
        "-"
    };
    let l = if (Buttons::MoveLeft as u16) & curr.buttons != 0 {
        "A"
    } else {
        "-"
    };
    let r = if (Buttons::MoveRight as u16) & curr.buttons != 0 {
        "D"
    } else {
        "-"
    };
    let b = if (Buttons::Back as u16) & curr.buttons != 0 {
        "S"
    } else {
        "-"
    };

    let j = if (Buttons::Jump as u16) & curr.buttons != 0 {
        "J"
    } else {
        "-"
    };
    let d = if (Buttons::Duck as u16) & curr.buttons != 0 {
        "D"
    } else {
        "-"
    };

    let mut spacing_jump = 10;
    if (Buttons::Forward as u16) & curr.buttons != 0 {
        spacing_jump -= 2;
    }

    let mut spacing_duck = 7;
    if (Buttons::MoveLeft as u16) & curr.buttons != 0 {
        spacing_duck -= 1;
    }
    if (Buttons::MoveRight as u16) & curr.buttons != 0 {
        spacing_duck -= 2;
    }
    if (Buttons::Back as u16) & curr.buttons != 0 {
        spacing_duck -= 1;
    }

    let spacing_jump = " ".repeat(spacing_jump);
    let spacing_duck = " ".repeat(spacing_duck);

    let message = format!(
        " {}{}{}\n{} {} {}{}{}\0",
        f, spacing_jump, j, l, b, r, spacing_duck, d
    );
    let message = message.leak().as_bytes();

    let text = TeTextMessage {
        channel: 5,
        // (0, 0) is top left
        x: 0.75f32.coord_conversion(),
        y: 0.25f32.coord_conversion(),
        effect: 0,
        text_color: [255, 255, 255, 0].to_vec(),
        effect_color: [255, 255, 255, 0].to_vec(),
        fade_in_time: 25,
        fade_out_time: 76,
        hold_time: 60,
        effect_time: None,
        message: message.to_vec(),
    };

    let temp_entity = SvcTempEntity {
        entity_type: 29,
        entity: TempEntity::TeTextMessage(text),
    };

    Some(temp_entity)
}
