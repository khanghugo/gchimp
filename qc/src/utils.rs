use common::take_until_unbalanced;
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
