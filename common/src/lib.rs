use nom::{combinator::fail, IResult as _IResult};

pub mod setup_studio_model_transformations;

pub type IResult<'a, T> = _IResult<&'a str, T>;

// https://github.com/getreu/parse-hyperlinks/blob/5af034d14aa72ffb9e705da13bf557a564b1bebf/parse-hyperlinks/src/lib.rs#L41
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        let mut ignore_bracket = false;
        while let Some(n) = &i[index..].find(&[opening_bracket, closing_bracket, '\\', '"'][..]) {
            index += n;
            let mut it = i[index..].chars();
            match it.next() {
                Some('\\') => {
                    // Skip the escape char `\`.
                    index += '\\'.len_utf8();
                    // Skip also the following char.
                    if let Some(c) = it.next() {
                        index += c.len_utf8();
                    }
                }
                // ignore bracket inside quotation mark
                Some('"') => {
                    ignore_bracket = !ignore_bracket;
                    index += '"'.len_utf8();
                }
                Some(c) if c == opening_bracket => {
                    if !ignore_bracket {
                        bracket_counter += 1;
                    }

                    // need to increment when matching, otherwise deadlock
                    index += opening_bracket.len_utf8();
                }
                Some(c) if c == closing_bracket => {
                    if !ignore_bracket {
                        bracket_counter -= 1;
                    }

                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                return Ok((&i[index..], &i[0..index]));
            };
        }

        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Ok(fail(i)?)
        }
    }
}
