use glam::DVec3;
use nom::number::complete::double as _double;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till},
    character::complete::{digit1, multispace0, space0},
    combinator::{fail, map, map_parser, map_res, opt, recognize},
    multi::many0,
    sequence::{preceded, terminated, tuple},
};

use crate::types::IResult;

pub fn _number(i: &str) -> IResult<i32> {
    map_res(recognize(preceded(opt(tag("-")), digit1)), |s: &str| {
        s.parse::<i32>()
    })(i)
}

pub fn number(i: &str) -> IResult<i32> {
    preceded(space0, _number)(i)
}

pub fn signed_double(i: &str) -> IResult<f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &str) -> IResult<f64> {
    preceded(space0, signed_double)(i)
}

pub fn quoted_text(i: &str) -> IResult<&str> {
    terminated(preceded(tag("\""), take_till(|c| c == '\"')), tag("\""))(i)
}

pub fn dvec3(i: &str) -> IResult<DVec3> {
    map(tuple((double, double, double)), |(x, y, z)| {
        DVec3::new(x, y, z)
    })(i)
}

// Do not consume space at the end because we don't know if we are at the end of line or not.
// This is pretty dangerous and it might take braces or any kind of arbitrary delimiter.
pub fn between_space(i: &str) -> IResult<&str> {
    let (i, res) = take_till(|c| c == ' ' || c == '\n' || c == '\r')(i)?;

    if res.is_empty() {
        Ok(fail(i)?)
    } else {
        Ok((i, res))
    }
}

// Filee name may or may not have quotation mark.
pub fn name_string(i: &str) -> IResult<&str> {
    alt((quoted_text, between_space))(i)
}

pub fn discard_comment_line(i: &str) -> IResult<&str> {
    terminated(
        preceded(tuple((multispace0, tag("//"))), take_till(|c| c == '\n')),
        multispace0,
    )(i)
}

pub fn discard_comment_lines(i: &str) -> IResult<&str> {
    map(many0(discard_comment_line), |_| "")(i)
}

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

pub fn between_braces<'a, T>(
    f: impl FnMut(&'a str) -> IResult<'a, T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    // Look ahead approach to avoid using name_string / between_space
    // between_space is very bad for things like this.

    map_parser(
        preceded(
            tuple((multispace0, tag("{"), multispace0)),
            terminated(
                take_until_unbalanced('{', '}'),
                tuple((tag("}"), multispace0)),
            ),
        ),
        f,
    )
}

pub fn line<'a, T>(
    f: impl FnMut(&'a str) -> IResult<'a, T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    // Take a line separated by either \r\n or just \n by looking ahead.
    map_parser(
        preceded(
            space0,
            terminated(take_till(|c| c == '\n' || c == '\r'), multispace0),
        ),
        f,
    )
}
